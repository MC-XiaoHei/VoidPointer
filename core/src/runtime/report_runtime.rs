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

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use super::*;
    use crate::report::config::ReportConfig;

    fn make_state() -> ReportState {
        ReportState::new(ReportConfig { report_hz: 1000.0 })
    }

    #[test]
    fn new_has_zero_state() {
        let r = MouseReportRuntime::new();
        assert_eq!(r.pending_wheel, 0);
        assert_eq!(r.last_sent_buttons, 0);
    }

    #[test]
    fn default_equals_new() {
        assert_eq!(
            MouseReportRuntime::default().pending_wheel,
            MouseReportRuntime::new().pending_wheel
        );
    }

    #[test]
    fn reset_pending_output_clears_wheel() {
        let mut r = MouseReportRuntime::new();
        r.ingest_wheel_delta(3);
        assert!(r.pending_wheel != 0);
        r.reset_pending_output();
        assert_eq!(r.pending_wheel, 0);
    }

    #[test]
    fn reset_route_sync_clears_buttons() {
        let mut r = MouseReportRuntime::new();
        let report = MouseReport {
            buttons: MouseButtons {
                left: true,
                middle: false,
                right: false,
            },
            dx: 0,
            dy: 0,
            wheel: 0,
        };
        let mut state = make_state();
        r.apply_send_status(report, HidSendStatus::Sent, &mut state);
        assert!(r.buttons_changed(0));
        r.reset_route_sync();
        assert!(!r.buttons_changed(0));
    }

    #[test]
    fn reset_all_clears_everything() {
        let mut r = MouseReportRuntime::new();
        r.ingest_wheel_delta(5);
        let report = MouseReport {
            buttons: MouseButtons {
                left: true,
                middle: false,
                right: false,
            },
            dx: 0,
            dy: 0,
            wheel: 0,
        };
        let mut state = make_state();
        r.apply_send_status(report, HidSendStatus::Sent, &mut state);
        r.reset_all();
        assert_eq!(r.pending_wheel, 0);
        assert_eq!(r.last_sent_buttons, 0);
    }

    #[test]
    fn ingest_wheel_accumulates() {
        let mut r = MouseReportRuntime::new();
        r.ingest_wheel_delta(1);
        r.ingest_wheel_delta(2);
        assert_eq!(r.pending_wheel, 3);
    }

    #[test]
    fn ingest_wheel_clamps_positive() {
        let mut r = MouseReportRuntime::new();
        r.ingest_wheel_delta(127);
        r.ingest_wheel_delta(127);
        assert_eq!(r.pending_wheel, 127);
    }

    #[test]
    fn ingest_wheel_clamps_negative() {
        let mut r = MouseReportRuntime::new();
        r.ingest_wheel_delta(-128);
        r.ingest_wheel_delta(-128);
        assert_eq!(r.pending_wheel, -128);
    }

    #[test]
    fn buttons_changed_initial() {
        let r = MouseReportRuntime::new();
        assert!(r.buttons_changed(1));
    }

    #[test]
    fn buttons_unchanged_after_sent() {
        let mut r = MouseReportRuntime::new();
        let report = MouseReport {
            buttons: MouseButtons {
                left: true,
                middle: false,
                right: false,
            },
            dx: 0,
            dy: 0,
            wheel: 0,
        };
        let mut state = make_state();
        r.apply_send_status(report, HidSendStatus::Sent, &mut state);
        assert!(!r.buttons_changed(report.buttons.pack()));
    }

    #[test]
    fn send_needed_motion_delta() {
        let r = MouseReportRuntime::new();
        assert!(r.send_needed(Some(ReportDelta { dx: 1, dy: 0 }), 0, false, false));
    }

    #[test]
    fn send_needed_wheel() {
        let mut r = MouseReportRuntime::new();
        r.ingest_wheel_delta(1);
        assert!(r.send_needed(None, 0, false, false));
    }

    #[test]
    fn send_needed_button_change() {
        let r = MouseReportRuntime::new();
        assert!(r.send_needed(None, 1, false, false));
    }

    #[test]
    fn send_needed_hid_retry() {
        let r = MouseReportRuntime::new();
        assert!(r.send_needed(None, 0, true, false));
    }

    #[test]
    fn send_needed_report_dirty() {
        let r = MouseReportRuntime::new();
        assert!(r.send_needed(None, 0, false, true));
    }

    #[test]
    fn send_needed_none_when_clean() {
        let r = MouseReportRuntime::new();
        assert!(!r.send_needed(None, 0, false, false));
    }

    #[test]
    fn build_report_with_motion() {
        let r = MouseReportRuntime::new();
        let buttons = MouseButtons {
            left: true,
            middle: false,
            right: false,
        };
        let report = r.build_report(buttons, Some(ReportDelta { dx: 10, dy: -5 }));
        assert_eq!(report.dx, 10);
        assert_eq!(report.dy, -5);
        assert_eq!(report.wheel, 0);
        assert!(report.buttons.left);
    }

    #[test]
    fn build_report_without_motion() {
        let r = MouseReportRuntime::new();
        let buttons = MouseButtons {
            left: false,
            middle: true,
            right: false,
        };
        let report = r.build_report(buttons, None);
        assert_eq!(report.dx, 0);
        assert_eq!(report.dy, 0);
        assert_eq!(report.wheel, 0);
        assert!(report.buttons.middle);
    }

    #[test]
    fn build_report_with_wheel() {
        let mut r = MouseReportRuntime::new();
        r.ingest_wheel_delta(42);
        let buttons = MouseButtons::default();
        let report = r.build_report(buttons, None);
        assert_eq!(report.wheel, 42);
    }

    #[test]
    fn build_report_clamps_wheel() {
        let mut r = MouseReportRuntime::new();
        r.ingest_wheel_delta(127);
        r.ingest_wheel_delta(127);
        let buttons = MouseButtons::default();
        let report = r.build_report(buttons, None);
        assert_eq!(report.wheel, 127);
    }

    #[test]
    fn apply_send_status_sent_updates_wheel_and_buttons() {
        let mut r = MouseReportRuntime::new();
        r.ingest_wheel_delta(10);
        let buttons = MouseButtons {
            left: true,
            middle: false,
            right: false,
        };
        let report = r.build_report(buttons, None);
        let mut state = make_state();
        r.apply_send_status(report, HidSendStatus::Sent, &mut state);
        assert_eq!(r.pending_wheel, 0);
        assert!(!r.buttons_changed(buttons.pack()));
    }

    #[test]
    fn apply_send_status_retry_later_does_not_update() {
        let mut r = MouseReportRuntime::new();
        r.ingest_wheel_delta(10);
        let buttons = MouseButtons {
            left: true,
            middle: false,
            right: false,
        };
        let report = r.build_report(buttons, Some(ReportDelta { dx: 3, dy: 0 }));
        let mut state = make_state();
        r.apply_send_status(report, HidSendStatus::RetryLater, &mut state);
        assert_eq!(r.pending_wheel, 10);
        assert!(r.buttons_changed(1));
    }

    #[test]
    fn apply_send_status_not_connected_does_not_update() {
        let mut r = MouseReportRuntime::new();
        r.ingest_wheel_delta(5);
        let buttons = MouseButtons::default();
        let report = r.build_report(buttons, None);
        let mut state = make_state();
        r.apply_send_status(report, HidSendStatus::NotConnected, &mut state);
        assert_eq!(r.pending_wheel, 5);
    }

    #[test]
    fn apply_send_status_fatal_does_not_update() {
        let mut r = MouseReportRuntime::new();
        r.ingest_wheel_delta(3);
        let buttons = MouseButtons::default();
        let report = r.build_report(buttons, None);
        let mut state = make_state();
        r.apply_send_status(report, HidSendStatus::Fatal, &mut state);
        assert_eq!(r.pending_wheel, 3);
    }

    #[test]
    fn multiple_wheel_delta_roundtrip() {
        let mut r = MouseReportRuntime::new();
        r.ingest_wheel_delta(3);
        r.ingest_wheel_delta(-1);
        r.ingest_wheel_delta(2);
        assert_eq!(r.pending_wheel, 4);
        let buttons = MouseButtons::default();
        let report = r.build_report(buttons, None);
        assert_eq!(report.wheel, 4);
        let mut state = make_state();
        r.apply_send_status(report, HidSendStatus::Sent, &mut state);
        assert_eq!(r.pending_wheel, 0);
    }
}
