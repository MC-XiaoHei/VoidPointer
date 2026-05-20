use crate::motion::state::MotionState;
use crate::report::config::ReportConfig;
use crate::report::types::ReportDelta;

pub struct ReportState {
    cfg: ReportConfig,

    accum_x: f32,
    accum_y: f32,

    pending_x: i32,
    pending_y: i32,
}

impl ReportState {
    pub fn new(cfg: ReportConfig) -> Self {
        Self {
            cfg,
            accum_x: 0.0,
            accum_y: 0.0,
            pending_x: 0,
            pending_y: 0,
        }
    }

    pub fn apply_config(&mut self, cfg: ReportConfig) {
        self.cfg = cfg;
        self.reset_fractional();
        self.clear_pending();
    }

    pub fn ingest_motion(&mut self, motion: MotionState) {
        if !motion.valid || self.cfg.report_hz <= 0.0 {
            self.reset_fractional();
            return;
        }

        if motion.vx == 0.0 {
            self.reset_x_fractional();
        }

        if motion.vy == 0.0 {
            self.reset_y_fractional();
        }

        self.accum_x += motion.vx / self.cfg.report_hz;
        self.accum_y += motion.vy / self.cfg.report_hz;

        let dx = trunc_toward_zero(self.accum_x);
        let dy = trunc_toward_zero(self.accum_y);

        self.accum_x -= dx as f32;
        self.accum_y -= dy as f32;

        self.pending_x += dx;
        self.pending_y += dy;
    }

    pub fn peek_report(&self) -> Option<ReportDelta> {
        let dx = self.pending_x.clamp(i8::MIN as i32, i8::MAX as i32) as i8;
        let dy = self.pending_y.clamp(i8::MIN as i32, i8::MAX as i32) as i8;

        if dx == 0 && dy == 0 {
            None
        } else {
            Some(ReportDelta { dx, dy })
        }
    }

    pub fn commit_sent(&mut self, sent: ReportDelta) {
        self.pending_x -= sent.dx as i32;
        self.pending_y -= sent.dy as i32;
    }

    pub fn reset_x_fractional(&mut self) {
        self.accum_x = 0.0;
    }

    pub fn reset_y_fractional(&mut self) {
        self.accum_y = 0.0;
    }

    pub fn reset_fractional(&mut self) {
        self.reset_x_fractional();
        self.reset_y_fractional();
    }

    pub fn clear_x_pending(&mut self) {
        self.pending_x = 0;
    }

    pub fn clear_y_pending(&mut self) {
        self.pending_y = 0;
    }

    pub fn clear_pending(&mut self) {
        self.clear_x_pending();
        self.clear_y_pending();
    }

    pub fn reset_all(&mut self) {
        self.reset_fractional();
        self.clear_pending();
    }

    pub fn has_pending(&self) -> bool {
        self.pending_x != 0 || self.pending_y != 0
    }

    pub fn pending(&self) -> (i32, i32) {
        (self.pending_x, self.pending_y)
    }

    pub fn config(&self) -> ReportConfig {
        self.cfg
    }
}

fn trunc_toward_zero(v: f32) -> i32 {
    if v >= 0.0 {
        libm::floorf(v) as i32
    } else {
        libm::ceilf(v) as i32
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

    fn motion(vx: f32, vy: f32) -> MotionState {
        MotionState {
            vx,
            vy,
            valid: true,
        }
    }

    #[test]
    fn apply_config_updates_hz_and_resets_fractional() {
        let mut r = ReportState::new(ReportConfig { report_hz: 100.0 });
        r.ingest_motion(motion(150.0, 0.0));
        let dx_before = r.peek_report().unwrap().dx;
        assert!(dx_before > 0);

        r.apply_config(ReportConfig { report_hz: 500.0 });
        assert!(!r.has_pending());

        r.ingest_motion(motion(1000.0, 0.0));
        assert!(r.has_pending());
        let report = r.peek_report().unwrap();
        assert_eq!(report.dx, (1000 / 500) as i8);
    }

    #[test]
    fn new_has_no_pending() {
        let r = ReportState::new(cfg());
        assert!(!r.has_pending());
        assert_eq!(r.pending(), (0, 0));
    }

    #[test]
    fn ingest_accumulates() {
        let mut r = ReportState::new(cfg());
        r.ingest_motion(motion(500.0, 300.0));
        assert!(!r.has_pending());
    }

    #[test]
    fn ingest_triggers_report() {
        let mut r = ReportState::new(cfg());
        r.ingest_motion(motion(1500.0, 0.0));
        let report = r.peek_report().unwrap();
        assert_eq!(report.dx, 1);
        assert_eq!(report.dy, 0);
    }

    #[test]
    fn commit_sent_reduces_pending() {
        let mut r = ReportState::new(cfg());
        r.ingest_motion(motion(2500.0, 1500.0));
        let report = r.peek_report().unwrap();
        assert_eq!(report.dx, 2);
        assert_eq!(report.dy, 1);
        r.commit_sent(report);
        assert!(!r.has_pending());
    }

    #[test]
    fn invalid_motion_resets_accum() {
        let mut r = ReportState::new(cfg());
        r.ingest_motion(motion(1500.0, 0.0));
        assert!(r.peek_report().is_some());
        r.ingest_motion(MotionState {
            vx: 0.0,
            vy: 0.0,
            valid: false,
        });
        r.commit_sent(ReportDelta { dx: 1, dy: 0 });
        r.ingest_motion(motion(1500.0, 0.0));
        assert!(r.peek_report().is_some());
    }

    #[test]
    fn zero_motion_resets_fractional() {
        let mut r = ReportState::new(ReportConfig { report_hz: 4.0 });
        r.ingest_motion(motion(3.0, 3.0));
        r.ingest_motion(motion(0.0, 0.0));
        r.ingest_motion(motion(3.0, 0.0));
        assert_eq!(r.peek_report(), None);
        r.ingest_motion(motion(3.0, 0.0));
        assert_eq!(r.peek_report().unwrap().dx, 1);
    }

    #[test]
    fn reset_all_clears_everything() {
        let mut r = ReportState::new(cfg());
        r.ingest_motion(motion(1500.0, 1500.0));
        r.reset_all();
        assert!(!r.has_pending());
        assert_eq!(r.peek_report(), None);
    }

    #[test]
    fn config_returns_initial() {
        let r = ReportState::new(ReportConfig { report_hz: 123.0 });
        assert_eq!(r.config().report_hz, 123.0);
    }

    #[test]
    fn clear_pending_removes_pending() {
        let mut r = ReportState::new(cfg());
        r.ingest_motion(motion(1500.0, 0.0));
        assert!(r.has_pending());
        r.clear_pending();
        assert!(!r.has_pending());
    }

    #[test]
    fn zero_report_hz_resets_fractional() {
        let mut r = ReportState::new(ReportConfig { report_hz: 0.0 });
        r.ingest_motion(motion(1500.0, 0.0));
        assert_eq!(r.peek_report(), None);
    }

    #[test]
    fn partial_commit_leaves_remaining() {
        let mut r = ReportState::new(cfg());
        r.ingest_motion(motion(3500.0, 1000.0));
        let report = r.peek_report().unwrap();
        assert_eq!(report.dx, 3);
        assert_eq!(report.dy, 1);
        r.commit_sent(ReportDelta { dx: 1, dy: 1 });
        assert_eq!(r.pending(), (2, 0));
    }

    #[test]
    fn trunc_toward_zero_positive() {
        assert_eq!(trunc_toward_zero(3.7), 3);
        assert_eq!(trunc_toward_zero(0.5), 0);
        assert_eq!(trunc_toward_zero(0.0), 0);
    }

    #[test]
    fn trunc_toward_zero_negative() {
        assert_eq!(trunc_toward_zero(-3.7), -3);
        assert_eq!(trunc_toward_zero(-0.5), 0);
    }

    #[test]
    fn peek_report_clamps_to_i8() {
        let mut r = ReportState::new(cfg());
        r.ingest_motion(motion(300_000.0, -200_000.0));
        let report = r.peek_report().unwrap();
        assert_eq!(report.dx, 127);
        assert_eq!(report.dy, -128);
    }
}
