#![cfg_attr(coverage, coverage(off))]

use super::Runtime;
use crate::attitude::types::SflpGameRotationRaw;
use crate::attitude::{clear_current_attitude, update_current_attitude_from_raw};
use crate::ffi::bindings::{
    VP_STATUS_NOT_READY, VP_STATUS_OK, c_vp_exti_clear_pending, c_vp_exti_unmask,
};
use crate::led::LedProfile;
use crate::led::TICK_MS;
use crate::led::patterns::{CONNECTED, DISCONNECTED, MODE_2G4, MODE_BLE};
use crate::route::UsbState;
use crate::runtime::events::RuntimeEvent;

impl Runtime {
    pub fn play_transient_led<const N: usize>(
        &mut self,
        profile: &'static LedProfile<N>,
        timestamp: u32,
    ) {
        profile.play(crate::ffi::board_map::BoardSignal::STATUS_LED);
        self.led_manager
            .begin_transient(profile.playback_ms(), timestamp);
        Self::request_poll_after(TICK_MS as u32);
    }

    pub(crate) fn drain_events(&mut self) {
        const MAX_EVENTS_PER_PASS: usize = 8;
        let mut drained = 0;

        while drained < MAX_EVENTS_PER_PASS {
            let Some(event) = super::EVENT_QUEUE.pop() else {
                self.pending.events = false;
                return;
            };

            drained += 1;
            self.apply_event(event);
        }

        self.pending.events = !super::EVENT_QUEUE.is_empty();
        if self.pending.events {
            Self::request_poll();
        }
    }

    fn apply_event(&mut self, event: RuntimeEvent) {
        match event {
            RuntimeEvent::BleConnected { timestamp } => self.on_ble_connected(timestamp),
            RuntimeEvent::BleInputReady { timestamp } => self.on_ble_input_ready(timestamp),
            RuntimeEvent::BleDisconnected { timestamp, .. } => self.on_ble_disconnected(timestamp),
            RuntimeEvent::DongleConnected { timestamp } => self.on_dongle_connected(timestamp),
            RuntimeEvent::DongleDisconnected { timestamp, .. } => {
                self.on_dongle_disconnected(timestamp)
            }
            RuntimeEvent::UsbStateChanged { state, timestamp } => {
                self.on_usb_state_changed(state, timestamp)
            }
            RuntimeEvent::ButtonExti {
                button_id,
                level,
                timestamp,
            } => self.on_button_exti(button_id, level, timestamp),
            RuntimeEvent::ModeSwitchExti { level, timestamp } => {
                self.on_mode_switch_exti(level, timestamp)
            }
            RuntimeEvent::DebounceTick { timestamp } => self.on_debounce_tick(timestamp),
            RuntimeEvent::EncoderExti {
                a_level,
                b_level,
                timestamp,
            } => self.on_encoder_exti(a_level, b_level, timestamp),
            RuntimeEvent::ImuInt { timestamp } => self.on_imu_int(timestamp),
            RuntimeEvent::ImuSample {
                raw_x,
                raw_y,
                raw_z,
                timestamp,
            } => self.on_imu_sample(raw_x, raw_y, raw_z, timestamp),
            RuntimeEvent::ImuFifoDone {
                status, timestamp, ..
            } => self.on_imu_fifo_done(status, timestamp),
            RuntimeEvent::HidSendDone { timestamp, .. } => self.on_hid_send_done(timestamp),
            RuntimeEvent::VendorReportRx { timestamp, .. } => self.on_vendor_report_rx(timestamp),
        }
    }

    fn on_ble_connected(&mut self, timestamp: u32) {
        self.router.set_ble_connected(true);
        self.router.set_ble_input_ready(false);
        self.mark_activity(timestamp);
        self.play_transient_led(&CONNECTED, timestamp);
    }

    fn on_ble_input_ready(&mut self, timestamp: u32) {
        self.router.set_ble_input_ready(true);
        self.mark_activity(timestamp);
        self.dirty.report = true;
    }

    fn on_ble_disconnected(&mut self, timestamp: u32) {
        self.router.set_ble_input_ready(false);
        self.router.set_ble_connected(false);
        self.play_transient_led(&DISCONNECTED, timestamp);
        self.mark_activity(timestamp);
        self.dirty.report = true;
    }

    fn on_dongle_connected(&mut self, timestamp: u32) {
        self.router.set_dongle_connected(true);
        self.mark_activity(timestamp);
        self.dirty.report = true;
        self.play_transient_led(&CONNECTED, timestamp);
    }

    fn on_dongle_disconnected(&mut self, timestamp: u32) {
        self.router.set_dongle_connected(false);
        self.mark_activity(timestamp);
        self.play_transient_led(&DISCONNECTED, timestamp);
        self.dirty.report = true;
    }

    fn on_usb_state_changed(&mut self, state: u8, timestamp: u32) {
        let usb_state = UsbState::from(state);
        self.router.set_usb_state(usb_state);
        self.mark_activity(timestamp);
        log::debug!(
            "usb state changed;state={},wired_active={}",
            super::usb_state_log_name(usb_state),
            matches!(usb_state, UsbState::Configured)
        );
        if matches!(usb_state, UsbState::Configured) {
            self.play_transient_led(&CONNECTED, timestamp);
        } else if matches!(usb_state, UsbState::Detached) {
            self.play_transient_led(&DISCONNECTED, timestamp);
        }
        self.dirty.report = true;
    }

    fn on_button_exti(&mut self, button_id: u8, level: u8, timestamp: u32) {
        self.mark_activity(timestamp);
        if self.input.on_button_exti(button_id, level != 0) {
            self.dirty.input = true;
        }
    }

    fn on_mode_switch_exti(&mut self, level: u8, timestamp: u32) {
        self.mark_activity(timestamp);
        self.dirty.input = true;
        if level != 0 {
            self.play_transient_led(&MODE_2G4, timestamp);
        } else {
            self.play_transient_led(&MODE_BLE, timestamp);
        }
    }

    fn on_debounce_tick(&mut self, timestamp: u32) {
        self.mark_activity(timestamp);
        if self.input.on_debounce_tick() {
            self.dirty.input = true;
            self.dirty.report = true;
        }
    }

    fn on_encoder_exti(&mut self, a_level: u8, b_level: u8, timestamp: u32) {
        self.mark_activity(timestamp);
        if self.input.on_encoder_exti(a_level != 0, b_level != 0) {
            self.dirty.input = true;
            self.dirty.report = true;
        }
    }

    fn on_imu_int(&mut self, timestamp: u32) {
        self.mark_activity(timestamp);
        rearm_imu_interrupts();
        self.pending.imu_fifo_read = false;
        self.imu_poll_deadline_ms = Some(timestamp);
        self.dirty.power = true;
    }

    fn on_imu_sample(&mut self, raw_x: u16, raw_y: u16, raw_z: u16, timestamp: u32) {
        self.mark_activity(timestamp);
        let raw = SflpGameRotationRaw {
            x: raw_x,
            y: raw_y,
            z: raw_z,
        };
        update_current_attitude_from_raw(raw);
        self.latest_imu_sample = crate::runtime::LatestImuSample {
            raw,
            timestamp_ms: timestamp,
            valid: true,
        };
        self.dirty.motion = true;
        self.dirty.report = true;
    }

    fn on_imu_fifo_done(&mut self, status: u8, timestamp: u32) {
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

    fn on_hid_send_done(&mut self, timestamp: u32) {
        self.mark_activity(timestamp);
        self.pending.hid_retry = true;
    }

    fn on_vendor_report_rx(&mut self, timestamp: u32) {
        self.mark_activity(timestamp);
        self.vendor.mark_rx_pending();
        self.pending.vendor_rx = true;
    }
}

#[cfg_attr(coverage, coverage(off))]
pub fn rearm_imu_interrupts() {
    for input_id in [
        crate::ffi::bindings::VP_INPUT_IMU_INT1 as u8,
        crate::ffi::bindings::VP_INPUT_IMU_INT2 as u8,
    ] {
        let _ = unsafe { c_vp_exti_clear_pending(input_id) };
        let _ = unsafe { c_vp_exti_unmask(input_id) };
    }
}
