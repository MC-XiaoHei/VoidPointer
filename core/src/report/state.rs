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
