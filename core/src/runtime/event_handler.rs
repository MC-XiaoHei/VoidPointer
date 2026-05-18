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
        // 事件队列单次只排一小段，避免事件风暴长期霸占主循环
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
            RuntimeEvent::BleConnected { timestamp } => {
                self.router.set_ble_connected(true);
                self.router.set_ble_input_ready(false);
                self.mark_activity(timestamp);
                self.play_transient_led(&CONNECTED, timestamp);
            }
            RuntimeEvent::BleInputReady { timestamp } => {
                self.router.set_ble_input_ready(true);
                self.mark_activity(timestamp);
                self.dirty.report = true;
            }
            RuntimeEvent::BleDisconnected { timestamp, .. } => {
                self.router.set_ble_input_ready(false);
                self.router.set_ble_connected(false);
                self.play_transient_led(&DISCONNECTED, timestamp);
                self.mark_activity(timestamp);
                self.dirty.report = true;
            }
            RuntimeEvent::DongleConnected { timestamp } => {
                self.router.set_dongle_connected(true);
                self.mark_activity(timestamp);
                self.dirty.report = true;
                self.play_transient_led(&CONNECTED, timestamp);
            }
            RuntimeEvent::DongleDisconnected { timestamp, .. } => {
                self.router.set_dongle_connected(false);
                self.mark_activity(timestamp);
                self.play_transient_led(&DISCONNECTED, timestamp);
                self.dirty.report = true;
            }
            RuntimeEvent::UsbStateChanged { state, timestamp } => {
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
            RuntimeEvent::ModeSwitchExti { level, timestamp } => {
                self.mark_activity(timestamp);
                self.dirty.input = true;
                if level != 0 {
                    self.play_transient_led(&MODE_2G4, timestamp);
                } else {
                    self.play_transient_led(&MODE_BLE, timestamp);
                }
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
                rearm_imu_interrupts();

                self.pending.imu_fifo_read = false;
                self.imu_poll_deadline_ms = Some(timestamp);
                self.dirty.power = true;
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
                self.latest_imu_sample = crate::runtime::LatestImuSample {
                    raw,
                    timestamp_ms: timestamp,
                    valid: true,
                };
                self.dirty.motion = true;
                self.dirty.report = true;
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
}

pub fn rearm_imu_interrupts() {
    for input_id in [
        crate::ffi::bindings::VP_INPUT_IMU_INT1 as u8,
        crate::ffi::bindings::VP_INPUT_IMU_INT2 as u8,
    ] {
        let _ = unsafe { c_vp_exti_clear_pending(input_id) };
        let _ = unsafe { c_vp_exti_unmask(input_id) };
    }
}
