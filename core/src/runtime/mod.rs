pub mod commands;
pub mod event_handler;
pub mod events;
pub mod hid_send;
pub mod power_coord;
pub mod report_runtime;

use crate::attitude::clear_current_attitude;
use crate::attitude::types::SflpGameRotationRaw;
use crate::config::ConfigManager;
use crate::ffi::bindings::{
    VP_WAKE_SOURCE_BUTTON, VP_WAKE_SOURCE_ENCODER, VP_WAKE_SOURCE_IMU, c_vp_request_core_poll,
    c_vp_request_core_poll_after, c_vp_wake_source_enable,
};
use crate::input::types::{InputManager, InputStatus};
use crate::led::runtime::LedManager;
use crate::motion::session::MotionSession;
use crate::power::PowerManager;
use crate::report::config::ReportConfig;
use crate::route::UsbState;
use crate::runtime::commands::RuntimeCommand;
use crate::runtime::events::EventQueue;
use crate::runtime::report_runtime::ReportRuntime;
use crate::utils::clock::RTC;
use crate::utils::global::MainLoopGlobal;
use crate::vendor::VendorRuntime;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

const HID_RETRY_DELAY_MS: u32 = 8;
const IMU_POLL_ACTIVE_MS: u32 = 30;
const IMU_FIFO_MAX_SAMPLES: u16 = 8;
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
    pub router: crate::route::HidRouter,
    pub power: PowerManager,
    pub config: ConfigManager,
    pub vendor: VendorRuntime,
    pub input: InputManager,
    pub last_input_status: InputStatus,
    pub dirty: DirtyFlags,
    pub pending: PendingFlags,
    pub report: ReportRuntime,
    pub last_activity_ms: AtomicU32,
    pub power_recheck_deadline_ms: Option<u32>,
    pub imu_poll_deadline_ms: Option<u32>,
    pub latest_imu_sample: LatestImuSample,
    pub motion_report_deadline_ms: Option<u32>,
    pub motion_session: MotionSession,
    pub led_manager: LedManager,
}

impl Runtime {
    pub fn new() -> Self {
        let now = RTC::millis().ticks();
        let mut input = InputManager::new();
        let initial_input = input.sync_snapshot();
        clear_current_attitude();

        let config = ConfigManager::new();
        let motion_cfg = config.current_config().motion;

        Self {
            router: crate::route::HidRouter::new(),
            power: PowerManager::new(),
            config,
            vendor: VendorRuntime::new(),
            input,
            last_input_status: initial_input,
            dirty: DirtyFlags::default(),
            pending: PendingFlags::default(),
            report: ReportRuntime::new(ReportConfig {
                report_hz: 1000.0 / MOTION_REPORT_MS as f32,
            }),
            last_activity_ms: AtomicU32::new(now),
            power_recheck_deadline_ms: None,
            imu_poll_deadline_ms: Some(now),
            latest_imu_sample: LatestImuSample::default(),
            motion_report_deadline_ms: Some(now),
            motion_session: MotionSession::new(motion_cfg),
            led_manager: LedManager::new(),
        }
    }

    pub fn enable_input_interrupts(&mut self) {
        self.input.enable_interrupts();
    }

    pub fn request_poll() {
        // POLL_PENDING 必须在唤醒主循环之前立起，否则 ISR 和主循环之间存在竞态
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
            self.dirty.power = true;
            self.pending.power_recheck = true;
        }
    }

    pub fn sync_motion_config(&mut self) {
        let motion_cfg = self.config.current_config().motion;
        self.motion_session.reconfigure(motion_cfg);
        self.motion_report_deadline_ms = Some(RTC::millis().ticks());
        self.report.reset_all();
    }

    fn imu_poll_enabled(&self) -> bool {
        self.power.state() == crate::power::PowerState::Active
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

    pub fn maybe_start_imu_poll(&mut self) -> Option<RuntimeCommand> {
        if !self.imu_poll_enabled() {
            self.imu_poll_deadline_ms = None;
            return None;
        }

        if self.pending.imu_fifo_read {
            return None;
        }

        let now = RTC::millis().ticks();
        let deadline = self.imu_poll_deadline_ms.unwrap_or(now);
        if !deadline_due(now, deadline) {
            Self::request_poll_after(deadline_remaining_ms(now, deadline));
            return None;
        }

        self.pending.imu_fifo_read = true;
        self.imu_poll_deadline_ms = Some(now.wrapping_add(IMU_POLL_ACTIVE_MS));
        Some(RuntimeCommand::ReadImuFifo {
            max_samples: IMU_FIFO_MAX_SAMPLES,
        })
    }

    pub fn poll(&mut self) -> Option<RuntimeCommand> {
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

    fn process_once(&mut self) -> Option<RuntimeCommand> {
        let now = RTC::millis().ticks();
        self.led_manager.clear_tick_scheduled();
        if self.led_manager.poll(now) {
            Self::request_poll_after(10);
        }

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
            let prev_motion = self.config.current_config().motion;
            self.vendor
                .poll(&self.router, &mut self.config, &self.power);
            self.pending.vendor_rx = false;

            if self.config.current_config().motion != prev_motion {
                self.sync_motion_config();
            }
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
            return Some(RuntimeCommand::ReadImuFifo {
                max_samples: IMU_FIFO_MAX_SAMPLES,
            });
        }

        if self.pending.config_save || self.dirty.config {
            self.config.poll();
            self.pending.config_save = false;
            self.dirty.config = false;
        }

        if let Some(command) = self.poll_power() {
            return Some(command);
        }

        None
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

pub fn usb_state_log_name(state: UsbState) -> &'static str {
    match state {
        UsbState::Detached => "detached",
        UsbState::Attached => "attached",
        UsbState::Configured => "configured",
        UsbState::Suspended => "suspended",
        UsbState::Error => "error",
    }
}

pub fn deadline_due(now: u32, deadline: u32) -> bool {
    now.wrapping_sub(deadline) < 0x8000_0000
}

pub fn deadline_remaining_ms(now: u32, deadline: u32) -> u32 {
    if deadline_due(now, deadline) {
        1
    } else {
        deadline.wrapping_sub(now).max(1)
    }
}

pub fn clear_suspend_resume_sources() {
    for source in [
        VP_WAKE_SOURCE_BUTTON,
        VP_WAKE_SOURCE_ENCODER,
        VP_WAKE_SOURCE_IMU,
    ] {
        let _ = unsafe { c_vp_wake_source_enable(source, 0) };
    }
}
