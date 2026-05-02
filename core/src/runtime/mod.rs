pub mod commands;
pub mod events;

use crate::config::ConfigManager;
use crate::ffi::bindings::{
    VP_HID_ROUTE_BLE, c_vp_request_core_poll, c_vp_request_core_poll_after, c_vp_rtc_millis,
};
use crate::hid::types::{HidSendStatus, MouseButtons, MouseReport};
use crate::input::types::InputManager;
use crate::power::PowerManager;
use crate::route::{HidRouter, UsbState};
use crate::runtime::commands::{RuntimeCommand, RuntimeCommandResult};
use crate::runtime::events::{EventQueue, RuntimeEvent};
use crate::utils::global::MainLoopGlobal;
use crate::vendor::VendorRuntime;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

const HID_RETRY_DELAY_MS: u32 = 8;
const ENABLE_POWER_MANAGER: bool = false;

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
    pub power_eval: bool,
}

impl PendingFlags {
    pub fn any(self) -> bool {
        self.events || self.imu_fifo_read || self.vendor_rx || self.config_save || self.power_eval
    }
}

pub struct Runtime {
    pub router: HidRouter,
    pub power: PowerManager,
    pub config: ConfigManager,
    pub vendor: VendorRuntime,
    pub input: InputManager,
    pub dirty: DirtyFlags,
    pub pending: PendingFlags,
    pub pending_wheel: i16,
    pub last_sent_buttons: u8,
    pub last_activity_ms: AtomicU32,
    pub power_eval_deadline_ms: Option<u32>,
}

impl Runtime {
    pub fn new() -> Self {
        let now = unsafe { c_vp_rtc_millis() };
        let mut input = InputManager::new();
        let _ = input.sync_snapshot();

        Self {
            router: HidRouter::new(),
            power: PowerManager::new(),
            config: ConfigManager::new(),
            vendor: VendorRuntime::new(),
            input,
            dirty: DirtyFlags::default(),
            pending: PendingFlags::default(),
            pending_wheel: 0,
            last_sent_buttons: 0,
            last_activity_ms: AtomicU32::new(now),
            power_eval_deadline_ms: None,
        }
    }

    pub fn request_poll() {
        POLL_PENDING.store(true, Ordering::Release);
        unsafe { c_vp_request_core_poll() };
    }

    pub fn request_poll_after(ms: u32) {
        unsafe { c_vp_request_core_poll_after(ms) };
    }

    pub fn mark_activity(&mut self, timestamp_ms: u32) {
        self.last_activity_ms.store(timestamp_ms, Ordering::Release);
        if ENABLE_POWER_MANAGER {
            self.power_eval_deadline_ms = None;
            self.dirty.power = true;
            self.pending.power_eval = true;
        }
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

    pub fn apply_command_result(&mut self, result: RuntimeCommandResult) {
        match result {
            RuntimeCommandResult::MouseSent {
                route,
                report,
                status,
            } => {
                if route != VP_HID_ROUTE_BLE as u8 {
                    return;
                }

                self.dirty.report = false;

                match status {
                    HidSendStatus::Sent => {
                        self.pending_wheel = self.pending_wheel.saturating_sub(report.wheel as i16);
                        self.last_sent_buttons = report.buttons.pack();
                        self.pending.hid_retry = false;
                    }
                    HidSendStatus::RetryLater => {
                        self.pending.hid_retry = true;
                        Self::request_poll_after(HID_RETRY_DELAY_MS);
                    }
                    HidSendStatus::NotConnected | HidSendStatus::Fatal => {
                        self.pending.hid_retry = false;
                    }
                }
            }
            RuntimeCommandResult::PowerTransitionDone { target, accepted } => {
                self.power_eval_deadline_ms = None;
                self.power.apply_transition_result(target, accepted);
                self.pending.power_eval = false;
                self.dirty.power = false;
            }
            RuntimeCommandResult::ImuFifoReadRequested { status: _ } => {
                // The platform reports completion via vp_on_imu_fifo_done() when an
                // async read is actually in flight. For Busy/Unsupported/Error, drop
                // this request to avoid a tight bottom-half retry loop; the next IMU
                // interrupt can request another read.
                self.pending.imu_fifo_read = false;
            }
        }

        if !self.pending.hid_retry {
            self.reschedule_power_eval_deadline();
        }

        if self.pending.events
            || self.pending.imu_fifo_read
            || self.pending.vendor_rx
            || self.pending.config_save
            || self.pending.power_eval
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
            self.vendor.poll();
            self.pending.vendor_rx = false;
        }

        if self.pending.imu_fifo_read {
            return Some(RuntimeCommand::ReadImuFifo { max_samples: 8 });
        }

        if self.pending.config_save || self.dirty.config {
            self.config.poll();
            self.pending.config_save = false;
            self.dirty.config = self.config.is_dirty();
        }

        let now = unsafe { c_vp_rtc_millis() };
        let power_eval_due = self
            .power_eval_deadline_ms
            .is_some_and(|deadline| deadline_due(now, deadline));

        if ENABLE_POWER_MANAGER && (self.pending.power_eval || self.dirty.power || power_eval_due) {
            let last_activity = self.last_activity_ms.load(Ordering::Acquire);
            if power_eval_due {
                self.power_eval_deadline_ms = None;
            }
            if let Some(transition) =
                self.power
                    .poll(now, last_activity, self.config.is_dirty(), &self.router)
            {
                self.power_eval_deadline_ms = None;
                return Some(RuntimeCommand::PowerTransition {
                    target: transition.target,
                });
            }
            self.schedule_next_power_eval(now, last_activity);
            self.pending.power_eval = false;
            self.dirty.power = false;
        } else if ENABLE_POWER_MANAGER {
            if let Some(deadline) = self.power_eval_deadline_ms {
                if !self.pending.hid_retry {
                    Self::request_poll_after(deadline_remaining_ms(now, deadline));
                }
            }
        }

        self.dirty.input = false;
        self.dirty.motion = false;
        self.dirty.report = false;

        None
    }

    fn schedule_next_power_eval(&mut self, now: u32, last_activity: u32) {
        if let Some(delay_ms) =
            self.power
                .next_eval_delay_ms(now, last_activity, self.config.is_dirty(), &self.router)
        {
            self.power_eval_deadline_ms = Some(now.wrapping_add(delay_ms));
            if !self.pending.hid_retry {
                Self::request_poll_after(delay_ms);
            }
        } else {
            self.power_eval_deadline_ms = None;
        }
    }

    fn reschedule_power_eval_deadline(&self) {
        if let Some(deadline) = self.power_eval_deadline_ms {
            let now = unsafe { c_vp_rtc_millis() };
            Self::request_poll_after(deadline_remaining_ms(now, deadline));
        }
    }

    fn drain_events(&mut self) {
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
                self.mark_activity(timestamp);
            }
            RuntimeEvent::BleDisconnected { timestamp, .. } => {
                self.router.set_ble_connected(false);
                self.mark_activity(timestamp);
            }
            RuntimeEvent::DongleConnected { timestamp } => {
                self.router.set_dongle_connected(true);
                self.mark_activity(timestamp);
            }
            RuntimeEvent::DongleDisconnected { timestamp, .. } => {
                self.router.set_dongle_connected(false);
                self.mark_activity(timestamp);
            }
            RuntimeEvent::UsbStateChanged { state, timestamp } => {
                self.router.set_usb_state(UsbState::from(state));
                self.mark_activity(timestamp);
            }
            RuntimeEvent::ButtonExti {
                button_id,
                level,
                timestamp,
            } => {
                self.mark_activity(timestamp);
                if self.input.on_button_exti(button_id, level != 0) {
                    self.dirty.input = true;
                }
            }
            RuntimeEvent::ModeSwitchExti { timestamp, .. } => {
                self.mark_activity(timestamp);
                self.dirty.input = true;
            }
            RuntimeEvent::DebounceTick { timestamp } => {
                self.mark_activity(timestamp);
                if self.input.on_debounce_tick() {
                    self.dirty.input = true;
                    self.dirty.report = true;
                }
            }
            RuntimeEvent::EncoderExti {
                a_level,
                b_level,
                timestamp,
            } => {
                self.mark_activity(timestamp);
                if self.input.on_encoder_exti(a_level != 0, b_level != 0) {
                    self.dirty.input = true;
                    self.dirty.report = true;
                }
            }
            RuntimeEvent::ImuInt { timestamp } => {
                self.mark_activity(timestamp);
                self.pending.imu_fifo_read = true;
                self.dirty.motion = true;
            }
            RuntimeEvent::ImuSample { timestamp, .. } => {
                self.mark_activity(timestamp);
                self.dirty.motion = true;
                self.dirty.report = true;
            }
            RuntimeEvent::ImuFifoDone { timestamp, .. } => {
                self.mark_activity(timestamp);
                self.pending.imu_fifo_read = false;
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

    fn poll_input_and_hid(&mut self) -> Option<RuntimeCommand> {
        let input = self.input.get_current_input();
        let buttons = MouseButtons {
            left: input.left,
            right: input.right,
            middle: input.middle,
        };
        let packed_buttons = buttons.pack();

        self.pending_wheel = self
            .pending_wheel
            .saturating_add(input.wheel_delta as i16)
            .clamp(i8::MIN as i16, i8::MAX as i16);

        if input.wheel_delta != 0 || packed_buttons != self.last_sent_buttons {
            let now = unsafe { c_vp_rtc_millis() };
            self.mark_activity(now);
            self.dirty.report = true;
        }

        if !self.router.is_ble_connected() {
            self.pending.hid_retry = false;
            return None;
        }

        if self.pending_wheel == 0
            && packed_buttons == self.last_sent_buttons
            && !self.pending.hid_retry
        {
            return None;
        }

        let wheel = self.pending_wheel.clamp(-127, 127) as i8;
        let report = MouseReport {
            buttons,
            dx: 0,
            dy: 0,
            wheel,
        };

        Some(RuntimeCommand::SendMouse {
            route: VP_HID_ROUTE_BLE as u8,
            report,
        })
    }
}

fn deadline_due(now: u32, deadline: u32) -> bool {
    now.wrapping_sub(deadline) < 0x8000_0000
}

fn deadline_remaining_ms(now: u32, deadline: u32) -> u32 {
    if deadline_due(now, deadline) {
        1
    } else {
        deadline.wrapping_sub(now).max(1)
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}
