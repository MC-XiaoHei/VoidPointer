use super::{HID_RETRY_DELAY_MS, MOTION_REPORT_MS, Runtime, deadline_due, deadline_remaining_ms};
use crate::ffi::bindings::{VP_STATUS_OK, c_vp_hid_route_ready, vp_hid_route_t};
use crate::hid::types::{HidSendStatus, MouseButtons, MouseReport};
use crate::runtime::commands::{RuntimeCommand, RuntimeCommandResult};
use crate::utils::clock::RTC;
use crate::vendor::PendingVendorTx;

impl Runtime {
    pub fn schedule_hid_retry(&mut self) {
        self.pending.hid_retry = true;
        Self::request_poll_after(HID_RETRY_DELAY_MS);
    }

    fn handle_hid_send_status(&mut self, status: HidSendStatus) {
        match status {
            HidSendStatus::Sent | HidSendStatus::NotConnected | HidSendStatus::Fatal => {
                self.pending.hid_retry = false;
            }
            HidSendStatus::RetryLater => {
                self.schedule_hid_retry();
            }
        }
    }

    fn apply_mouse_send_status(&mut self, report: MouseReport, status: HidSendStatus) {
        self.report.apply_send_status(report, status);
        self.dirty.report = false;
        self.handle_hid_send_status(status);
    }

    fn apply_vendor_send_status(
        &mut self,
        route: vp_hid_route_t,
        report: crate::hid::types::CustomReport,
        status: HidSendStatus,
    ) {
        match status {
            HidSendStatus::RetryLater => {
                self.vendor
                    .requeue_pending_tx(PendingVendorTx { route, report });
            }
            _ => {}
        }
        self.handle_hid_send_status(status);
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
                self.dirty.power = false;
            }
            RuntimeCommandResult::ImuFifoReadRequested { status } => {
                if status != VP_STATUS_OK as u8 {
                    self.pending.imu_fifo_read = false;
                    let now = RTC::millis().ticks();
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

    pub(crate) fn route_ready(&self, route: vp_hid_route_t) -> bool {
        unsafe { c_vp_hid_route_ready(route) != 0 }
    }

    fn clear_unsent_motion_output(&mut self) {
        self.report.reset_all();
        self.motion_report_deadline_ms = Some(RTC::millis().ticks());
    }

    /// 路由不可用时不累计失效 motion，避免恢复后回放旧移动
    fn defer_report_until_route_event(&mut self) -> Option<RuntimeCommand> {
        self.clear_unsent_motion_output();
        self.report.reset_route_sync();
        self.pending.hid_retry = false;
        self.dirty.report = false;
        None
    }

    pub(crate) fn defer_vendor_until_route_event(
        &mut self,
        tx: PendingVendorTx,
    ) -> Option<RuntimeCommand> {
        self.vendor.requeue_pending_tx(tx);
        self.pending.hid_retry = false;
        None
    }

    pub(crate) fn poll_input_and_hid(&mut self) -> Option<RuntimeCommand> {
        let input = self.step_input();
        let now = RTC::millis().ticks();
        self.step_laser(input.laser);
        self.step_wheel_button(
            input.wheel_delta,
            input.left,
            input.right,
            input.middle,
            now,
        );
        self.step_motion(now);

        if !self.report.send_needed(
            MouseButtons {
                left: input.left,
                right: input.right,
                middle: input.middle,
            }
            .pack(),
            self.pending.hid_retry,
            self.dirty.report,
        ) {
            return None;
        }

        self.try_send_mouse(input.left, input.right, input.middle)
    }

    fn step_input(&mut self) -> crate::input::types::InputStatus {
        use crate::attitude::get_current_attitude;

        let input = self.input.get_current_input();
        self.last_input_status = input;
        self.dirty.input = false;

        let motion_active =
            self.motion_session
                .update_trigger(crate::motion::session::TriggerButtons {
                    action: input.action,
                    middle: input.middle,
                });

        if motion_active
            && self.motion_session.should_process_sample(
                self.latest_imu_sample.timestamp_ms,
                self.latest_imu_sample.valid,
            )
        {
            if let Some(attitude) = get_current_attitude() {
                self.motion_session
                    .update_attitude(&attitude, self.latest_imu_sample.timestamp_ms);
            }
        }

        input
    }

    fn step_laser(&mut self, laser: bool) {
        if laser {
            crate::pwm::set_laser_duty(255);
        } else {
            crate::pwm::set_laser_duty(0);
        }
    }

    fn step_wheel_button(
        &mut self,
        wheel_delta: i8,
        left: bool,
        right: bool,
        middle: bool,
        now: u32,
    ) {
        self.report.ingest_wheel_delta(wheel_delta);

        let packed_buttons = MouseButtons {
            left,
            right,
            middle,
        }
        .pack();
        let button_changed = self.report.send_needed(packed_buttons, false, false);
        if wheel_delta != 0 || button_changed {
            self.mark_activity(now);
            self.dirty.report = true;
        }
    }

    fn step_motion(&mut self, now: u32) {
        let motion_active = self.motion_session.is_active();
        let motion_report_deadline = self.motion_report_deadline_ms.unwrap_or(now);

        if motion_active && deadline_due(now, motion_report_deadline) {
            self.report.ingest_motion(self.motion_session.output());
            self.motion_report_deadline_ms = Some(now.wrapping_add(MOTION_REPORT_MS));
            Self::request_poll_after(MOTION_REPORT_MS);
        } else if motion_active {
            Self::request_poll_after(deadline_remaining_ms(now, motion_report_deadline));
        }
    }

    fn try_send_mouse(&mut self, left: bool, right: bool, middle: bool) -> Option<RuntimeCommand> {
        let route = self.router.preferred_mouse_route();
        if route == crate::route::HidRoute::None {
            return self.defer_report_until_route_event();
        }

        if !self.route_ready(route.as_ffi()) {
            return self.defer_report_until_route_event();
        }

        let buttons = MouseButtons {
            left,
            right,
            middle,
        };
        Some(RuntimeCommand::SendMouse {
            route: route.as_ffi(),
            report: self.report.build_report(buttons),
        })
    }
}
