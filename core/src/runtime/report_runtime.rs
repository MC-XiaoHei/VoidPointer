use crate::hid::types::{HidSendStatus, MouseButtons, MouseReport};
use crate::motion::state::MotionState;
use crate::report::config::ReportConfig;
use crate::report::state::ReportState;
use crate::report::types::ReportDelta;

struct MouseReportRuntime {
    pending_wheel: i16,
    last_sent_buttons: u8,
}

impl MouseReportRuntime {
    fn new() -> Self {
        Self {
            pending_wheel: 0,
            last_sent_buttons: 0,
        }
    }

    fn reset_pending_output(&mut self) {
        self.pending_wheel = 0;
    }

    fn reset_route_sync(&mut self) {
        self.last_sent_buttons = 0;
    }

    fn reset_all(&mut self) {
        self.reset_pending_output();
        self.reset_route_sync();
    }

    fn ingest_wheel_delta(&mut self, wheel_delta: i8) {
        self.pending_wheel = self
            .pending_wheel
            .saturating_add(wheel_delta as i16)
            .clamp(i8::MIN as i16, i8::MAX as i16);
    }

    fn buttons_changed(&self, packed_buttons: u8) -> bool {
        packed_buttons != self.last_sent_buttons
    }

    fn build_report(
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

    fn commit_send(&mut self, report: &MouseReport) {
        self.pending_wheel = self.pending_wheel.saturating_sub(report.wheel as i16);
        self.last_sent_buttons = report.buttons.pack();
    }
}

pub struct ReportRuntime {
    mouse: MouseReportRuntime,
    state: ReportState,
}

impl ReportRuntime {
    pub fn new(cfg: ReportConfig) -> Self {
        Self {
            mouse: MouseReportRuntime::new(),
            state: ReportState::new(cfg),
        }
    }

    pub fn ingest_motion(&mut self, motion: MotionState) {
        self.state.ingest_motion(motion);
    }

    pub fn ingest_wheel_delta(&mut self, delta: i8) {
        self.mouse.ingest_wheel_delta(delta);
    }

    pub fn send_needed(&self, packed_buttons: u8, hid_retry: bool, report_dirty: bool) -> bool {
        self.state.peek_report().is_some()
            || self.mouse.pending_wheel != 0
            || self.mouse.buttons_changed(packed_buttons)
            || hid_retry
            || report_dirty
    }

    pub fn build_report(&self, buttons: MouseButtons) -> MouseReport {
        self.mouse.build_report(buttons, self.state.peek_report())
    }

    pub fn apply_send_status(&mut self, report: MouseReport, status: HidSendStatus) {
        if status != HidSendStatus::Sent {
            return;
        }
        self.mouse.commit_send(&report);
        self.state.commit_sent(ReportDelta {
            dx: report.dx,
            dy: report.dy,
        });
    }

    pub fn reset_all(&mut self) {
        self.mouse.reset_all();
        self.state.reset_all();
    }

    pub fn apply_config(&mut self, cfg: ReportConfig) {
        self.state.apply_config(cfg);
    }

    pub fn reset_route_sync(&mut self) {
        self.mouse.reset_route_sync();
    }

    pub fn has_pending(&self) -> bool {
        self.state.has_pending() || self.mouse.pending_wheel != 0
    }
}

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use super::*;
    use crate::motion::state::MotionState;

    fn cfg() -> ReportConfig {
        ReportConfig { report_hz: 1000.0 }
    }

    fn make_motion(vx: f32, vy: f32) -> MotionState {
        MotionState {
            vx,
            vy,
            valid: true,
        }
    }

    #[test]
    fn new_has_no_pending() {
        let r = ReportRuntime::new(cfg());
        assert!(!r.has_pending());
        assert!(!r.send_needed(0, false, false));
    }

    #[test]
    fn motion_accumulates_to_send_needed() {
        let mut r = ReportRuntime::new(cfg());
        r.ingest_motion(make_motion(1500.0, 0.0));
        assert!(r.send_needed(0, false, false));
    }

    #[test]
    fn apply_config_updates_report_params() {
        let mut r = ReportRuntime::new(cfg());
        r.ingest_motion(make_motion(1500.0, 0.0));
        assert!(r.send_needed(0, false, false));

        r.apply_config(ReportConfig { report_hz: 500.0 });
        assert!(!r.send_needed(0, false, false));

        r.ingest_motion(make_motion(1000.0, 0.0));
        assert!(r.send_needed(0, false, false));
        let report = r.build_report(MouseButtons::default());
        assert_eq!(report.dx, 1000 / 500);
    }

    #[test]
    fn wheel_triggers_send_needed() {
        let mut r = ReportRuntime::new(cfg());
        r.ingest_wheel_delta(1);
        assert!(r.send_needed(0, false, false));
    }

    #[test]
    fn button_change_triggers_send_needed() {
        let r = ReportRuntime::new(cfg());
        assert!(r.send_needed(1, false, false));
    }

    #[test]
    fn hid_retry_triggers_send_needed() {
        let r = ReportRuntime::new(cfg());
        assert!(r.send_needed(0, true, false));
    }

    #[test]
    fn report_dirty_triggers_send_needed() {
        let r = ReportRuntime::new(cfg());
        assert!(r.send_needed(0, false, true));
    }

    #[test]
    fn build_report_with_motion() {
        let mut r = ReportRuntime::new(cfg());
        r.ingest_motion(make_motion(1500.0, 500.0));
        let buttons = MouseButtons {
            left: true,
            middle: false,
            right: false,
        };
        let report = r.build_report(buttons);
        assert_eq!(report.dx, 1);
        assert_eq!(report.dy, 0);
        assert!(report.buttons.left);
    }

    #[test]
    fn build_report_with_wheel() {
        let mut r = ReportRuntime::new(cfg());
        r.ingest_wheel_delta(3);
        let buttons = MouseButtons::default();
        let report = r.build_report(buttons);
        assert_eq!(report.wheel, 3);
    }

    #[test]
    fn apply_send_commits_wheel_and_buttons() {
        let mut r = ReportRuntime::new(cfg());
        r.ingest_wheel_delta(5);
        let buttons = MouseButtons {
            left: true,
            middle: false,
            right: false,
        };
        let report = r.build_report(buttons);
        r.apply_send_status(report, HidSendStatus::Sent);
        assert!(!r.has_pending());
    }

    #[test]
    fn apply_send_retry_does_not_commit() {
        let mut r = ReportRuntime::new(cfg());
        r.ingest_wheel_delta(5);
        let buttons = MouseButtons::default();
        let report = r.build_report(buttons);
        r.apply_send_status(report, HidSendStatus::RetryLater);
        assert!(r.has_pending());
    }

    #[test]
    fn apply_send_not_connected_does_not_commit() {
        let mut r = ReportRuntime::new(cfg());
        r.ingest_wheel_delta(3);
        let buttons = MouseButtons::default();
        let report = r.build_report(buttons);
        r.apply_send_status(report, HidSendStatus::NotConnected);
        assert!(r.has_pending());
    }

    #[test]
    fn reset_all_clears_everything() {
        let mut r = ReportRuntime::new(cfg());
        r.ingest_motion(make_motion(3000.0, 0.0));
        r.ingest_wheel_delta(5);
        r.reset_all();
        assert!(!r.has_pending());
        assert!(!r.send_needed(0, false, false));
    }

    #[test]
    fn reset_route_sync_clears_buttons() {
        let mut r = ReportRuntime::new(cfg());
        let buttons = MouseButtons {
            left: true,
            middle: false,
            right: false,
        };
        let report = r.build_report(buttons);
        r.apply_send_status(report, HidSendStatus::Sent);
        r.reset_route_sync();
        assert!(r.send_needed(1, false, false));
    }

    #[test]
    fn motion_and_wheel_roundtrip() {
        let mut r = ReportRuntime::new(cfg());
        r.ingest_motion(make_motion(1500.0, 1000.0));
        r.ingest_wheel_delta(3);
        r.ingest_wheel_delta(-1);

        let buttons = MouseButtons::default();
        let report = r.build_report(buttons);
        assert_eq!(report.dx, 1);
        assert_eq!(report.dy, 1);
        assert_eq!(report.wheel, 2);

        r.apply_send_status(report, HidSendStatus::Sent);
        assert!(!r.has_pending());
    }

    #[test]
    fn send_needed_false_when_clean() {
        let r = ReportRuntime::new(cfg());
        assert!(!r.send_needed(0, false, false));
    }
}
