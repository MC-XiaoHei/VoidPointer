#![cfg_attr(coverage, coverage(off))]

use super::Runtime;
use crate::attitude::clear_current_attitude;
use crate::ffi::bindings::{VP_STATUS_OK, c_vp_imu_config_active, c_vp_power_restore_from_sleep};
use crate::hid::types::MouseButtons;
use crate::power::PowerState;
use crate::runtime::commands::RuntimeCommand;
use crate::utils::clock::RTC;

impl Runtime {
    pub fn reschedule_power_recheck_deadline(&mut self) {
        if !super::ENABLE_POWER_MANAGER {
            return;
        }

        let now = RTC::millis().ticks();
        let Some(deadline) = self.power_recheck_deadline_ms else {
            return;
        };

        let delay_ms = super::deadline_remaining_ms(now, deadline);
        Self::request_poll_after(delay_ms);
    }

    pub(crate) fn poll_power(&mut self) -> Option<RuntimeCommand> {
        let now = RTC::millis().ticks();

        if let Some(deadline) = self.power_recheck_deadline_ms {
            if !super::deadline_due(now, deadline) {
                let delay_ms = super::deadline_remaining_ms(now, deadline);
                Self::request_poll_after(delay_ms);
                return None;
            }
        }

        self.power_recheck_deadline_ms = None;
        self.pending.power_recheck = false;
        self.dirty.power = false;

        let config_dirty = self.pending.config_save || self.dirty.config || self.config.is_dirty();
        let effective_config_dirty = config_dirty || !super::ENABLE_SLEEP_POWER_STATE;
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
            self.last_activity_ms
                .load(core::sync::atomic::Ordering::Acquire),
            effective_config_dirty,
            &self.router,
        );
        let current_state = self.power.state();
        if previous_state != current_state {
            if current_state == PowerState::Active {
                self.restore_active_runtime_state(previous_state);
            }
        }

        if let Some(ref t) = transition {
            if t.target != PowerState::Active {
                crate::led::stop_playback();
            }
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
            || self.config.is_dirty()
            || self.dirty.input
            || self.dirty.report
            || self.motion_session.is_active()
            || buttons_pressed
            || self.report.has_pending()
            || self
                .report
                .send_needed(packed_buttons, self.pending.hid_retry, self.dirty.report)
    }

    fn arm_power_recheck_deadline(&mut self, now: u32, effective_config_dirty: bool) {
        let next_delay = self.power.next_recheck_delay_ms(
            now,
            self.last_activity_ms
                .load(core::sync::atomic::Ordering::Acquire),
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

    pub fn restore_active_runtime_state(&mut self, previous_state: PowerState) {
        if previous_state == PowerState::Sleep {
            let status = unsafe { c_vp_power_restore_from_sleep() };
            if status != VP_STATUS_OK as u8 {
                log::warn!("sleep restore failed;status={}", status);
            }
        }

        super::clear_suspend_resume_sources();
        clear_current_attitude();
        self.latest_imu_sample = super::LatestImuSample::default();
        self.report.reset_all();
        self.motion_session.reset();

        let status = unsafe { c_vp_imu_config_active() };
        if status != VP_STATUS_OK as u8 {
            log::warn!("imu active restore failed;status={}", status);
        }
        let now = RTC::millis().ticks();
        self.motion_report_deadline_ms = Some(now);
        self.imu_poll_deadline_ms = Some(now);
    }
}
