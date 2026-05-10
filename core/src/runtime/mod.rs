pub mod commands;
pub mod events;
pub mod report_runtime;

use crate::attitude::types::SflpGameRotationRaw;
use crate::attitude::{
    clear_current_attitude, get_current_attitude, update_current_attitude_from_raw,
};
use crate::config::ConfigManager;
use crate::ffi::bindings::{
    VP_INPUT_IMU_INT1, VP_INPUT_IMU_INT2, VP_STATUS_NOT_READY, VP_STATUS_OK, VP_WAKE_SOURCE_BUTTON,
    VP_WAKE_SOURCE_ENCODER, VP_WAKE_SOURCE_IMU, c_vp_exti_clear_pending, c_vp_exti_unmask,
    c_vp_hid_route_ready, c_vp_imu_config_active, c_vp_power_restore_from_sleep,
    c_vp_request_core_poll, c_vp_request_core_poll_after, c_vp_rtc_millis, c_vp_wake_source_enable,
    vp_hid_route_t,
};
use crate::hid::types::{HidSendStatus, MouseButtons, MouseReport};
use crate::input::types::{InputManager, InputStatus};
use crate::motion::config::MotionConfig;
use crate::motion::resolver::TiltMotionSolver;
use crate::motion::state::MotionState;
use crate::power::{PowerManager, PowerState};
use crate::report::config::ReportConfig;
use crate::report::state::ReportState;
use crate::route::{HidRoute, HidRouter, UsbState};
use crate::runtime::commands::{RuntimeCommand, RuntimeCommandResult};
use crate::runtime::events::{EventQueue, RuntimeEvent};
use crate::runtime::report_runtime::MouseReportRuntime;
use crate::utils::global::MainLoopGlobal;
use crate::vendor::{PendingVendorTx, VendorRuntime};
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

const HID_RETRY_DELAY_MS: u32 = 8;
const IMU_POLL_ACTIVE_MS: u32 = 30;
const MOTION_REPORT_MS: u32 = 10;
const ENABLE_POWER_MANAGER: bool = true;
const ENABLE_SLEEP_POWER_STATE: bool = false;

pub static RUNTIME: MainLoopGlobal<Runtime> = MainLoopGlobal::new();

pub static POLL_RUNNING: AtomicBool = AtomicBool::new(false);
pub static POLL_PENDING: AtomicBool = AtomicBool::new(false);
pub static EVENTS_PENDING: AtomicBool = AtomicBool::new(false);
pub static EVENT_QUEUE: EventQueue = EventQueue::new();

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DirtyFlags {
    pub input: bool,
    pub motion: bool,
    pub report: bool,
    pub power: bool,
    pub config: bool,
}

impl DirtyFlags {
    pub fn any(self) -> bool {
        self.input || self.motion || self.report || self.power || self.config
    }

    pub fn mark_input(&mut self) {
        self.input = true;
    }

    pub fn clear_input(&mut self) {
        self.input = false;
    }

    pub fn mark_motion(&mut self) {
        self.motion = true;
    }

    pub fn clear_motion(&mut self) {
        self.motion = false;
    }

    pub fn mark_report(&mut self) {
        self.report = true;
    }

    pub fn clear_report(&mut self) {
        self.report = false;
    }

    pub fn mark_power(&mut self) {
        self.power = true;
    }

    pub fn clear_power(&mut self) {
        self.power = false;
    }

    pub fn mark_config(&mut self) {
        self.config = true;
    }

    pub fn clear_config(&mut self) {
        self.config = false;
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PendingFlags {
    pub events: bool,
    pub hid_retry: bool,
    pub imu_fifo_read: bool,
    pub vendor_rx: bool,
    pub config_save: bool,
    pub power_recheck: bool,
}

impl PendingFlags {
    pub fn any(self) -> bool {
        self.events
            || self.hid_retry
            || self.imu_fifo_read
            || self.vendor_rx
            || self.config_save
            || self.power_recheck
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LatestImuSample {
    pub raw: SflpGameRotationRaw,
    pub timestamp_ms: u32,
    pub valid: bool,
}

pub struct Runtime {
    pub router: HidRouter,
    pub power: PowerManager,
    pub config: ConfigManager,
    pub vendor: VendorRuntime,
    pub input: InputManager,
    pub last_input_status: InputStatus,
    pub dirty: DirtyFlags,
    pub pending: PendingFlags,
    pub mouse_report: MouseReportRuntime,
    pub last_activity_ms: AtomicU32,
    pub power_recheck_deadline_ms: Option<u32>,
    pub imu_poll_deadline_ms: Option<u32>,
    pub latest_imu_sample: LatestImuSample,
    pub last_motion_sample_ts: Option<u32>,
    pub motion_report_deadline_ms: Option<u32>,
    pub motion_calibration_pending: bool,
    pub motion_solver: TiltMotionSolver,
    pub current_motion: MotionState,
    pub report_state: ReportState,
    pub motion_capture_active: bool,
}

impl Runtime {
    pub fn new() -> Self {
        let now = unsafe { c_vp_rtc_millis() };
        let mut input = InputManager::new();
        let initial_input = input.sync_snapshot();
        clear_current_attitude();

        Self {
            router: HidRouter::new(),
            power: PowerManager::new(),
            config: ConfigManager::new(),
            vendor: VendorRuntime::new(),
            input,
            last_input_status: initial_input,
            dirty: DirtyFlags::default(),
            pending: PendingFlags::default(),
            mouse_report: MouseReportRuntime::new(),
            last_activity_ms: AtomicU32::new(now),
            power_recheck_deadline_ms: None,
            imu_poll_deadline_ms: Some(now),
            latest_imu_sample: LatestImuSample::default(),
            last_motion_sample_ts: None,
            motion_report_deadline_ms: Some(now),
            motion_calibration_pending: false,
            motion_solver: TiltMotionSolver::new(MotionConfig::default()),
            current_motion: MotionState::default(),
            report_state: ReportState::new(ReportConfig {
                report_hz: 1000.0 / MOTION_REPORT_MS as f32,
            }),
            motion_capture_active: false,
        }
    }

    pub fn enable_input_interrupts(&mut self) {
        self.input.enable_interrupts();
    }

    pub fn request_poll() {
        // 先立 pending 标记，再唤醒主循环，避免边沿事件在窗口里丢失
        POLL_PENDING.store(true, Ordering::Release);
        unsafe { c_vp_request_core_poll() };
    }

    pub fn request_poll_after(ms: u32) {
        unsafe { c_vp_request_core_poll_after(ms) };
    }

    pub fn mark_activity(&mut self, timestamp_ms: u32) {
        self.last_activity_ms.store(timestamp_ms, Ordering::Release);
        if ENABLE_POWER_MANAGER {
            self.power_recheck_deadline_ms = None;
            self.dirty.mark_power();
            self.pending.power_recheck = true;
        }
    }

    fn reset_motion_capture_state(&mut self, calibration_pending: bool) {
        self.report_state.reset_all();
        self.current_motion = MotionState::default();
        self.last_motion_sample_ts = None;
        self.motion_report_deadline_ms = Some(unsafe { c_vp_rtc_millis() });
        self.motion_calibration_pending = calibration_pending;
    }

    fn imu_poll_enabled(&self) -> bool {
        self.power.state() == PowerState::Active
    }

    fn schedule_next_imu_poll(&mut self, base_timestamp_ms: u32) {
        if !self.imu_poll_enabled() {
            self.imu_poll_deadline_ms = None;
            return;
        }

        let deadline = base_timestamp_ms.wrapping_add(IMU_POLL_ACTIVE_MS);
        self.imu_poll_deadline_ms = Some(deadline);
        Self::request_poll_after(IMU_POLL_ACTIVE_MS);
    }

    fn maybe_start_imu_poll(&mut self) -> Option<RuntimeCommand> {
        if !self.imu_poll_enabled() {
            self.imu_poll_deadline_ms = None;
            return None;
        }

        if self.pending.imu_fifo_read {
            return None;
        }

        let now = unsafe { c_vp_rtc_millis() };
        let deadline = self.imu_poll_deadline_ms.unwrap_or(now);
        if !deadline_due(now, deadline) {
            Self::request_poll_after(deadline_remaining_ms(now, deadline));
            return None;
        }

        self.pending.imu_fifo_read = true;
        self.imu_poll_deadline_ms = Some(now.wrapping_add(IMU_POLL_ACTIVE_MS));
        Some(RuntimeCommand::ReadImuFifo { max_samples: 8 })
    }

    pub fn poll(&mut self) -> Option<RuntimeCommand> {
        // 单次 poll 允许短暂追平级联状态，但要限制 pass 数避免主循环长时间自旋
        const MAX_PASSES: usize = 4;
        let mut passes = 0;

        while passes < MAX_PASSES {
            passes += 1;
            POLL_PENDING.store(false, Ordering::Release);

            if let Some(command) = self.process_once() {
                return Some(command);
            }

            if !POLL_PENDING.load(Ordering::Acquire) && !self.pending.any() && !self.dirty.any() {
                break;
            }
        }

        if POLL_PENDING.load(Ordering::Acquire) || self.pending.any() || self.dirty.any() {
            Self::request_poll();
        }

        None
    }

    fn schedule_hid_retry(&mut self) {
        self.pending.hid_retry = true;
        Self::request_poll_after(HID_RETRY_DELAY_MS);
    }

    fn apply_mouse_send_status(&mut self, report: MouseReport, status: HidSendStatus) {
        self.mouse_report
            .apply_send_status(report, status, &mut self.report_state);
        self.dirty.clear_report();

        match status {
            HidSendStatus::Sent => {
                self.pending.hid_retry = false;
            }
            HidSendStatus::RetryLater => {
                self.schedule_hid_retry();
            }
            HidSendStatus::NotConnected | HidSendStatus::Fatal => {
                self.pending.hid_retry = false;
            }
        }
    }

    fn apply_vendor_send_status(
        &mut self,
        route: vp_hid_route_t,
        report: crate::hid::types::CustomReport,
        status: HidSendStatus,
    ) {
        match status {
            HidSendStatus::Sent => {
                self.pending.hid_retry = false;
            }
            HidSendStatus::RetryLater => {
                self.vendor
                    .requeue_pending_tx(PendingVendorTx { route, report });
                self.schedule_hid_retry();
            }
            HidSendStatus::NotConnected | HidSendStatus::Fatal => {
                self.pending.hid_retry = false;
            }
        }
    }

    pub fn apply_command_result(&mut self, result: RuntimeCommandResult) {
        match result {
            RuntimeCommandResult::MouseSent {
                route: _,
                report,
                status,
            } => {
                self.apply_mouse_send_status(report, status);
            }
            RuntimeCommandResult::VendorSent {
                route,
                report,
                status,
            } => {
                self.apply_vendor_send_status(route, report, status);
            }
            RuntimeCommandResult::PowerStateRequestDone { target, accepted } => {
                self.power_recheck_deadline_ms = None;
                self.power.apply_request_result(target, accepted);
                self.pending.power_recheck = false;
                self.dirty.clear_power();
            }
            RuntimeCommandResult::ImuFifoReadRequested { status } => {
                if status != VP_STATUS_OK as u8 {
                    self.pending.imu_fifo_read = false;
                    let now = unsafe { c_vp_rtc_millis() };
                    self.schedule_next_imu_poll(now);
                }
            }
        }

        if !self.pending.hid_retry {
            self.reschedule_power_recheck_deadline();
        }

        if self.pending.events
            || self.pending.hid_retry
            || self.pending.imu_fifo_read
            || self.pending.vendor_rx
            || self.pending.config_save
            || self.pending.power_recheck
            || self.dirty.any()
        {
            Self::request_poll();
        }
    }

    fn process_once(&mut self) -> Option<RuntimeCommand> {
        if EVENTS_PENDING.load(Ordering::Acquire) {
            EVENTS_PENDING.store(false, Ordering::Release);
            self.pending.events = true;
        }

        if self.pending.events {
            self.drain_events();
        }

        if let Some(command) = self.poll_input_and_hid() {
            return Some(command);
        }

        if self.pending.vendor_rx {
            self.vendor
                .poll(&self.router, &mut self.config, &self.power);
            self.pending.vendor_rx = false;
        }

        if let Some(command) = self.maybe_start_imu_poll() {
            return Some(command);
        }

        if let Some(tx) = self.vendor.take_pending_tx() {
            if !self.route_ready(tx.route) {
                return self.defer_vendor_until_route_event(tx);
            }

            self.pending.hid_retry = false;
            return Some(RuntimeCommand::SendVendor {
                route: tx.route,
                report: tx.report,
            });
        }

        if self.pending.imu_fifo_read {
            return Some(RuntimeCommand::ReadImuFifo { max_samples: 8 });
        }

        if self.pending.config_save || self.dirty.config {
            self.config.poll();
            self.pending.config_save = false;
            self.dirty.clear_config();
        }

        if let Some(command) = self.poll_power() {
            return Some(command);
        }

        None
    }

    fn reschedule_power_recheck_deadline(&mut self) {
        if !ENABLE_POWER_MANAGER {
            return;
        }

        let now = unsafe { c_vp_rtc_millis() };
        let Some(deadline) = self.power_recheck_deadline_ms else {
            return;
        };

        let delay_ms = deadline_remaining_ms(now, deadline);
        Self::request_poll_after(delay_ms);
    }

    fn poll_power(&mut self) -> Option<RuntimeCommand> {
        let now = unsafe { c_vp_rtc_millis() };

        if let Some(deadline) = self.power_recheck_deadline_ms {
            if !deadline_due(now, deadline) {
                let delay_ms = deadline_remaining_ms(now, deadline);
                Self::request_poll_after(delay_ms);
                return None;
            }
        }

        self.power_recheck_deadline_ms = None;
        self.pending.power_recheck = false;
        self.dirty.clear_power();

        let config_dirty = self.pending.config_save || self.dirty.config;
        let effective_config_dirty = config_dirty || !ENABLE_SLEEP_POWER_STATE;
        let previous_state = self.power.state();

        if self.power_has_blockers() {
            if self.power.state() != PowerState::Active {
                self.power.apply_request_result(PowerState::Active, true);
                self.restore_active_runtime_state(previous_state);
            }
            self.arm_power_recheck_deadline(now, effective_config_dirty);
            return None;
        }

        let transition = self.power.poll(
            now,
            self.last_activity_ms.load(Ordering::Acquire),
            effective_config_dirty,
            &self.router,
        );
        let current_state = self.power.state();
        if previous_state != current_state && current_state == PowerState::Active {
            self.restore_active_runtime_state(previous_state);
        }

        self.arm_power_recheck_deadline(now, effective_config_dirty);
        transition.map(|t| RuntimeCommand::RequestPowerState { target: t.target })
    }

    fn power_has_blockers(&self) -> bool {
        let buttons_pressed = self.last_input_status.left
            || self.last_input_status.right
            || self.last_input_status.middle
            || self.last_input_status.action;
        let packed_buttons = MouseButtons {
            left: self.last_input_status.left,
            right: self.last_input_status.right,
            middle: self.last_input_status.middle,
        }
        .pack();

        self.pending.events
            || self.pending.hid_retry
            || self.pending.imu_fifo_read
            || self.pending.vendor_rx
            || self.vendor.has_pending_tx()
            || self.pending.config_save
            || self.dirty.input
            || self.dirty.report
            || self.motion_capture_active
            || buttons_pressed
            || self.report_state.has_pending()
            || self.mouse_report.send_needed(
                self.report_state.peek_report(),
                packed_buttons,
                self.pending.hid_retry,
                self.dirty.report,
            )
    }

    fn arm_power_recheck_deadline(&mut self, now: u32, effective_config_dirty: bool) {
        let next_delay = self.power.next_recheck_delay_ms(
            now,
            self.last_activity_ms.load(Ordering::Acquire),
            effective_config_dirty,
            &self.router,
        );

        if let Some(delay_ms) = next_delay {
            self.power_recheck_deadline_ms = Some(now.wrapping_add(delay_ms));
            Self::request_poll_after(delay_ms);
        } else {
            self.power_recheck_deadline_ms = None;
        }
    }

    fn restore_active_runtime_state(&mut self, previous_state: PowerState) {
        if previous_state == PowerState::Sleep {
            let status = unsafe { c_vp_power_restore_from_sleep() };
            if status != VP_STATUS_OK as u8 {
                log::warn!("sleep restore failed;status={}", status);
            }
        }

        clear_suspend_resume_sources();
        clear_current_attitude();
        self.latest_imu_sample = LatestImuSample::default();
        self.current_motion = MotionState::default();
        self.last_motion_sample_ts = None;
        self.report_state.reset_all();
        self.mouse_report.reset_all();

        let status = unsafe { c_vp_imu_config_active() };
        if status != VP_STATUS_OK as u8 {
            log::warn!("imu active restore failed;status={}", status);
        }
        let now = unsafe { c_vp_rtc_millis() };
        self.motion_report_deadline_ms = Some(now);
        self.imu_poll_deadline_ms = Some(now);
    }

    fn drain_events(&mut self) {
        // 事件队列单次只排一小段，避免事件风暴长期霸占主循环
        const MAX_EVENTS_PER_PASS: usize = 8;
        let mut drained = 0;

        while drained < MAX_EVENTS_PER_PASS {
            let Some(event) = EVENT_QUEUE.pop() else {
                self.pending.events = false;
                return;
            };

            drained += 1;
            self.apply_event(event);
        }

        self.pending.events = !EVENT_QUEUE.is_empty();
        if self.pending.events {
            Self::request_poll();
        }
    }

    fn apply_event(&mut self, event: RuntimeEvent) {
        match event {
            RuntimeEvent::BleConnected { timestamp } => {
                self.router.set_ble_connected(true);
                self.router.set_ble_input_ready(false);
                self.mark_activity(timestamp);
            }
            RuntimeEvent::BleInputReady { timestamp } => {
                self.router.set_ble_input_ready(true);
                self.mark_activity(timestamp);
                self.dirty.mark_report();
            }
            RuntimeEvent::BleDisconnected { timestamp, .. } => {
                self.router.set_ble_input_ready(false);
                self.router.set_ble_connected(false);
                self.mark_activity(timestamp);
                self.dirty.mark_report();
            }
            RuntimeEvent::DongleConnected { timestamp } => {
                self.router.set_dongle_connected(true);
                self.mark_activity(timestamp);
                self.dirty.mark_report();
            }
            RuntimeEvent::DongleDisconnected { timestamp, .. } => {
                self.router.set_dongle_connected(false);
                self.mark_activity(timestamp);
                self.dirty.mark_report();
            }
            RuntimeEvent::UsbStateChanged { state, timestamp } => {
                let usb_state = UsbState::from(state);
                self.router.set_usb_state(usb_state);
                self.mark_activity(timestamp);
                log::debug!(
                    "usb state changed;state={},wired_active={}",
                    usb_state_log_name(usb_state),
                    matches!(usb_state, UsbState::Configured)
                );
                self.dirty.mark_report();
            }
            RuntimeEvent::ButtonExti {
                button_id,
                level,
                timestamp,
            } => {
                self.mark_activity(timestamp);
                if self.input.on_button_exti(button_id, level != 0) {
                    self.dirty.mark_input();
                }
            }
            RuntimeEvent::ModeSwitchExti { timestamp, .. } => {
                self.mark_activity(timestamp);
                self.dirty.mark_input();
            }
            RuntimeEvent::DebounceTick { timestamp } => {
                self.mark_activity(timestamp);
                if self.input.on_debounce_tick() {
                    self.dirty.mark_input();
                    self.dirty.mark_report();
                }
            }
            RuntimeEvent::EncoderExti {
                a_level,
                b_level,
                timestamp,
            } => {
                self.mark_activity(timestamp);
                if self.input.on_encoder_exti(a_level != 0, b_level != 0) {
                    self.dirty.mark_input();
                    self.dirty.mark_report();
                }
            }
            RuntimeEvent::ImuInt { timestamp } => {
                self.mark_activity(timestamp);
                rearm_imu_interrupts();

                self.pending.imu_fifo_read = false;
                self.imu_poll_deadline_ms = Some(timestamp);
                self.dirty.mark_power();
            }
            RuntimeEvent::ImuSample {
                raw_x,
                raw_y,
                raw_z,
                timestamp,
            } => {
                self.mark_activity(timestamp);
                let raw = SflpGameRotationRaw {
                    x: raw_x,
                    y: raw_y,
                    z: raw_z,
                };
                let _ = update_current_attitude_from_raw(raw);
                self.latest_imu_sample = LatestImuSample {
                    raw,
                    timestamp_ms: timestamp,
                    valid: true,
                };
                self.dirty.mark_motion();
                self.dirty.mark_report();
            }
            RuntimeEvent::ImuFifoDone {
                status,
                timestamp,
                dropped_count: _,
            } => {
                self.mark_activity(timestamp);
                self.pending.imu_fifo_read = false;
                if status != VP_STATUS_OK as u8 {
                    if status != VP_STATUS_NOT_READY as u8 {
                        log::warn!("imu fifo read failed;status={},ts={}", status, timestamp);
                    }
                    self.latest_imu_sample.valid = false;
                    clear_current_attitude();
                }
                self.schedule_next_imu_poll(timestamp);
            }
            RuntimeEvent::HidSendDone { timestamp, .. } => {
                self.mark_activity(timestamp);
                self.pending.hid_retry = true;
            }
            RuntimeEvent::VendorReportRx { timestamp, .. } => {
                self.mark_activity(timestamp);
                self.vendor.mark_rx_pending();
                self.pending.vendor_rx = true;
            }
        }
    }

    fn route_ready(&self, route: vp_hid_route_t) -> bool {
        unsafe { c_vp_hid_route_ready(route) != 0 }
    }

    fn clear_unsent_motion_output(&mut self) {
        self.report_state.reset_all();
        self.current_motion = MotionState::default();
        self.motion_report_deadline_ms = Some(unsafe { c_vp_rtc_millis() });
    }

    /// 鼠标路由不存在或尚未 ready 时，要收敛本次发送尝试
    /// 等真正改变路由可用性的事件再次唤醒，例如 BLE ready、USB 状态变化或新的输入活动
    fn defer_report_until_route_event(&mut self) -> Option<RuntimeCommand> {
        // route 不可用时不累计失效 motion，避免恢复后回放旧移动。
        self.clear_unsent_motion_output();
        self.mouse_report.reset_all();
        self.pending.hid_retry = false;
        self.dirty.clear_report();
        None
    }

    /// vendor 待发包已经真实存在，但 route not-ready 不等于可短退避重试
    /// 这里保留待发包并收敛本次尝试，等待 BLE ready、USB 状态变化等 route 事件再次唤醒
    fn defer_vendor_until_route_event(&mut self, tx: PendingVendorTx) -> Option<RuntimeCommand> {
        self.vendor.requeue_pending_tx(tx);
        self.pending.hid_retry = false;
        None
    }

    fn poll_input_and_hid(&mut self) -> Option<RuntimeCommand> {
        let input = self.input.get_current_input();
        self.last_input_status = input;
        self.dirty.clear_input();
        let buttons = MouseButtons {
            left: input.left,
            right: input.right,
            middle: input.middle,
        };
        let packed_buttons = buttons.pack();
        let motion_capture_active = input.action || input.middle;

        if motion_capture_active && !self.motion_capture_active {
            self.reset_motion_capture_state(true);
        } else if !motion_capture_active && self.motion_capture_active {
            self.reset_motion_capture_state(false);
        }
        self.motion_capture_active = motion_capture_active;

        if motion_capture_active
            && self.latest_imu_sample.valid
            && self.last_motion_sample_ts != Some(self.latest_imu_sample.timestamp_ms)
        {
            if let Some(attitude) = get_current_attitude() {
                if self.motion_calibration_pending {
                    self.motion_solver.calibrate(attitude);
                    self.current_motion = MotionState::default();
                    self.report_state.reset_all();
                    self.motion_calibration_pending = false;
                } else {
                    self.current_motion = self.motion_solver.update(attitude);
                }
                self.last_motion_sample_ts = Some(self.latest_imu_sample.timestamp_ms);
            }
        }

        let now = unsafe { c_vp_rtc_millis() };
        let motion_report_deadline = self.motion_report_deadline_ms.unwrap_or(now);
        if motion_capture_active && deadline_due(now, motion_report_deadline) {
            self.report_state.ingest_motion(self.current_motion);
            self.motion_report_deadline_ms = Some(now.wrapping_add(MOTION_REPORT_MS));
            Self::request_poll_after(MOTION_REPORT_MS);
        }
        let motion_delta = self.report_state.peek_report();

        // 运行时把输入侧滚轮增量汇总到发送侧缓冲，避免短时间多个步进被后来的 report 覆盖
        self.mouse_report.ingest_wheel_delta(input.wheel_delta);

        let button_changed = self.mouse_report.buttons_changed(packed_buttons);
        let wheel_changed = input.wheel_delta != 0;
        if wheel_changed || button_changed {
            self.mark_activity(now);
            self.dirty.mark_report();
        }

        if motion_capture_active && !deadline_due(now, motion_report_deadline) {
            Self::request_poll_after(deadline_remaining_ms(now, motion_report_deadline));
        }

        if !self.mouse_report.send_needed(
            motion_delta,
            packed_buttons,
            self.pending.hid_retry,
            self.dirty.report,
        ) {
            // 没有 motion、wheel、button 变化，也没有 retry/dirty 压力时直接退出
            return None;
        }

        let route = self.router.preferred_mouse_route();
        if route == HidRoute::None {
            return self.defer_report_until_route_event();
        }

        if !self.route_ready(route.as_ffi()) {
            return self.defer_report_until_route_event();
        }

        let report = self.mouse_report.build_report(buttons, motion_delta);

        Some(RuntimeCommand::SendMouse {
            route: route.as_ffi(),
            report,
        })
    }
}

fn usb_state_log_name(state: UsbState) -> &'static str {
    match state {
        UsbState::Detached => "detached",
        UsbState::Attached => "attached",
        UsbState::Configured => "configured",
        UsbState::Suspended => "suspended",
        UsbState::Error => "error",
    }
}

fn deadline_due(now: u32, deadline: u32) -> bool {
    // 用 wrapping 比较处理 rtc 回绕，避免简单大小比较在回绕点失真
    now.wrapping_sub(deadline) < 0x8000_0000
}

fn deadline_remaining_ms(now: u32, deadline: u32) -> u32 {
    // 已到期时仍返回 1ms，让调度路径尽快重新进入而不是返回 0
    if deadline_due(now, deadline) {
        1
    } else {
        deadline.wrapping_sub(now).max(1)
    }
}

fn clear_suspend_resume_sources() {
    // 清理 Suspend 期间使用的恢复源配置。
    for source in [
        VP_WAKE_SOURCE_BUTTON,
        VP_WAKE_SOURCE_ENCODER,
        VP_WAKE_SOURCE_IMU,
    ] {
        let _ = unsafe { c_vp_wake_source_enable(source, 0) };
    }
}

fn rearm_imu_interrupts() {
    for input_id in [VP_INPUT_IMU_INT1 as u8, VP_INPUT_IMU_INT2 as u8] {
        let _ = unsafe { c_vp_exti_clear_pending(input_id) };
        let _ = unsafe { c_vp_exti_unmask(input_id) };
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}
