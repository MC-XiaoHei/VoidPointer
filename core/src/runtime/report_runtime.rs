use crate::hid::types::{HidSendStatus, MouseButtons, MouseReport};
use crate::report::state::ReportState;
use crate::report::types::ReportDelta;

pub struct MouseReportRuntime {
    pending_wheel: i16,
    last_sent_buttons: u8,
}

impl MouseReportRuntime {
    pub fn new() -> Self {
        Self {
            pending_wheel: 0,
            last_sent_buttons: 0,
        }
    }

    pub fn reset_pending_output(&mut self) {
        self.pending_wheel = 0;
    }

    pub fn reset_route_sync(&mut self) {
        self.last_sent_buttons = 0;
    }

    pub fn reset_all(&mut self) {
        self.reset_pending_output();
        self.reset_route_sync();
    }

    pub fn ingest_wheel_delta(&mut self, wheel_delta: i8) {
        self.pending_wheel = self
            .pending_wheel
            .saturating_add(wheel_delta as i16)
            .clamp(i8::MIN as i16, i8::MAX as i16);
    }

    pub fn buttons_changed(&self, packed_buttons: u8) -> bool {
        packed_buttons != self.last_sent_buttons
    }

    pub fn send_needed(
        &self,
        motion_delta: Option<ReportDelta>,
        packed_buttons: u8,
        hid_retry: bool,
        report_dirty: bool,
    ) -> bool {
        motion_delta.is_some()
            || self.pending_wheel != 0
            || self.buttons_changed(packed_buttons)
            || hid_retry
            || report_dirty
    }

    pub fn build_report(
        &self,
        buttons: MouseButtons,
        motion_delta: Option<ReportDelta>,
    ) -> MouseReport {
        let wheel = self.pending_wheel.clamp(-127, 127) as i8;
        let (dx, dy) = match motion_delta {
            Some(delta) => (delta.dx, delta.dy),
            None => (0, 0),
        };

        MouseReport {
            buttons,
            dx,
            dy,
            wheel,
        }
    }

    pub fn apply_send_status(
        &mut self,
        report: MouseReport,
        status: HidSendStatus,
        report_state: &mut ReportState,
    ) {
        if status != HidSendStatus::Sent {
            return;
        }

        self.pending_wheel = self.pending_wheel.saturating_sub(report.wheel as i16);
        self.last_sent_buttons = report.buttons.pack();
        report_state.commit_sent(ReportDelta {
            dx: report.dx,
            dy: report.dy,
        });
    }
}

impl Default for MouseReportRuntime {
    fn default() -> Self {
        Self::new()
    }
}
