use crate::attitude::get_current_attitude;
use crate::attitude::types::AttitudeData;
use crate::motion::axis::{HW_MAP_X, HW_MAP_Y};
use crate::motion::config::MotionConfig;
use crate::motion::state::MotionState;

pub struct TiltMotionSolver {
    cfg: MotionConfig,
    filtered_vx: f32,
    filtered_vy: f32,

    pub center_x_rad: f32,
    pub center_y_rad: f32,
}

impl TiltMotionSolver {
    pub fn new(cfg: MotionConfig) -> Self {
        let mut result = Self {
            cfg,
            filtered_vx: 0.0,
            filtered_vy: 0.0,
            center_x_rad: 0.0,
            center_y_rad: 0.0,
        };

        if let Some(attitude) = get_current_attitude() {
            result.calibrate(attitude);
        }

        result
    }

    pub fn calibrate(&mut self, attitude: AttitudeData) {
        self.center_x_rad = HW_MAP_X.extract(&attitude);
        self.center_y_rad = HW_MAP_Y.extract(&attitude);
        self.filtered_vx = 0.0;
        self.filtered_vy = 0.0;
    }

    pub fn update(&mut self, attitude: AttitudeData) -> MotionState {
        let current_x_rad = HW_MAP_X.extract(&attitude);
        let current_y_rad = HW_MAP_Y.extract(&attitude);

        let raw_x = normalize_angle(current_x_rad - self.center_x_rad);
        let raw_y = normalize_angle(current_y_rad - self.center_y_rad);

        let x = if self.cfg.invert_x { -raw_x } else { raw_x };
        let y = if self.cfg.invert_y { -raw_y } else { raw_y };

        let nx = normalize_axis(x, self.cfg.deadzone_x_rad, self.cfg.max_angle_rad);
        let ny = normalize_axis(y, self.cfg.deadzone_y_rad, self.cfg.max_angle_rad);

        let target_vx = nx * self.cfg.sensitivity_x;
        let target_vy = ny * self.cfg.sensitivity_y;

        self.filtered_vx += self.cfg.smoothing_alpha * (target_vx - self.filtered_vx);
        self.filtered_vy += self.cfg.smoothing_alpha * (target_vy - self.filtered_vy);

        if libm::fabsf(self.filtered_vx) < self.cfg.deadzone_speed {
            self.filtered_vx = 0.0;
        }
        if libm::fabsf(self.filtered_vy) < self.cfg.deadzone_speed {
            self.filtered_vy = 0.0;
        }

        MotionState {
            vx: self.filtered_vx,
            vy: self.filtered_vy,
            valid: true,
        }
    }
}

fn normalize_angle(mut angle: f32) -> f32 {
    while angle > core::f32::consts::PI {
        angle -= 2.0 * core::f32::consts::PI;
    }
    while angle < -core::f32::consts::PI {
        angle += 2.0 * core::f32::consts::PI;
    }
    angle
}

fn normalize_axis(value: f32, deadzone: f32, max_angle: f32) -> f32 {
    if max_angle <= deadzone {
        return 0.0;
    }

    let sign = if value < 0.0 { -1.0 } else { 1.0 };
    let abs = libm::fabsf(value);

    if abs <= deadzone {
        0.0
    } else if abs >= max_angle {
        sign
    } else {
        let t = (abs - deadzone) / (max_angle - deadzone);

        sign * (t * t)
    }
}

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn normalize_angle_wraps_above_pi() {
        let result = normalize_angle(core::f32::consts::PI + 1.0);
        assert!((result - (-core::f32::consts::PI + 1.0)).abs() < 1e-6);
    }

    #[test]
    fn normalize_angle_wraps_below_neg_pi() {
        let result = normalize_angle(-core::f32::consts::PI - 1.0);
        assert!((result - (core::f32::consts::PI - 1.0)).abs() < 1e-6);
    }

    #[test]
    fn normalize_angle_stays_in_range() {
        assert!((normalize_angle(0.5) - 0.5).abs() < 1e-6);
        assert!((normalize_angle(-0.5) - (-0.5)).abs() < 1e-6);
        assert!((normalize_angle(core::f32::consts::PI) - core::f32::consts::PI).abs() < 1e-6);
    }

    #[test]
    fn normalize_axis_zero_at_deadzone() {
        assert_eq!(normalize_axis(0.04, 0.05, 1.0), 0.0);
        assert_eq!(normalize_axis(-0.04, 0.05, 1.0), 0.0);
    }

    #[test]
    fn normalize_axis_sign_at_max() {
        assert_eq!(normalize_axis(1.5, 0.05, 1.0), 1.0);
        assert_eq!(normalize_axis(-1.5, 0.05, 1.0), -1.0);
    }

    #[test]
    fn normalize_axis_zero_when_max_le_deadzone() {
        assert_eq!(normalize_axis(0.5, 0.5, 0.5), 0.0);
        assert_eq!(normalize_axis(0.5, 0.6, 0.5), 0.0);
    }

    #[test]
    fn normalize_axis_quadratic_mid() {
        let r = normalize_axis(0.5, 0.0, 1.0);
        assert!((r - 0.25).abs() < 1e-6);

        let r = normalize_axis(-0.5, 0.0, 1.0);
        assert!((r - (-0.25)).abs() < 1e-6);
    }

    fn attitude(roll: f32, pitch: f32, yaw: f32) -> AttitudeData {
        AttitudeData {
            roll,
            pitch,
            yaw,
            w: 1.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    #[test]
    fn solver_new_no_calibration() {
        let s = TiltMotionSolver::new(MotionConfig::default());
        assert_eq!(s.center_x_rad, 0.0);
        assert_eq!(s.center_y_rad, 0.0);
    }

    #[test]
    fn solver_new_with_cached_attitude() {
        let raw = crate::attitude::types::SflpGameRotationRaw { x: 0, y: 0, z: 0 };
        crate::attitude::update_current_attitude_from_raw(raw);
        let s = TiltMotionSolver::new(MotionConfig::default());
        crate::attitude::clear_current_attitude();
        assert!((s.center_x_rad).abs() < 1e-6);
        assert!((s.center_y_rad).abs() < 1e-6);
    }

    #[test]
    fn solver_calibrate_sets_center() {
        let mut s = TiltMotionSolver::new(MotionConfig::default());
        s.calibrate(attitude(0.5, -0.3, 0.0));
        assert!((s.center_x_rad - (-0.0)).abs() < 1e-6);
        assert!((s.center_y_rad - (-0.3)).abs() < 1e-6);
    }

    #[test]
    fn solver_update_after_calibrate() {
        let mut s = TiltMotionSolver::new(MotionConfig::default());
        s.calibrate(attitude(0.0, 0.0, 0.0));
        let result = s.update(attitude(0.1, 0.2, 0.0));
        assert!(result.valid);
        assert!(result.vx.abs() > 0.0 || result.vy.abs() > 0.0);
    }

    #[test]
    fn solver_update_smoothing() {
        let mut s = TiltMotionSolver::new(MotionConfig::default());
        s.calibrate(attitude(0.0, 0.0, 0.0));
        for _ in 0..10 {
            s.update(attitude(0.5, 0.0, 0.0));
        }
        assert!(libm::fabsf(s.filtered_vy) < 1.0);
    }

    #[test]
    fn solver_invert_y() {
        let mut cfg = MotionConfig::default();
        cfg.invert_y = true;
        let mut s = TiltMotionSolver::new(cfg);
        s.calibrate(attitude(0.0, 0.0, 0.0));
        // pitch=0.3 引起正 vy，invert_y 后变为负
        let result = s.update(attitude(0.0, 0.3, 0.0));
        assert!(result.vy < 0.0);
    }

    #[test]
    fn solver_invert_x() {
        let mut cfg = MotionConfig::default();
        cfg.invert_x = true;
        let mut s = TiltMotionSolver::new(cfg);
        s.calibrate(attitude(0.0, 0.0, 0.0));
        let result = s.update(attitude(0.0, 0.0, 0.5));
        assert!(result.vx > 0.0);
    }

    #[test]
    fn solver_deadzone_suppresses() {
        let mut s = TiltMotionSolver::new(MotionConfig::default());
        s.calibrate(attitude(0.0, 0.0, 0.0));
        let result = s.update(attitude(0.01, 0.01, 0.01));
        assert_eq!(result.vx, 0.0);
        assert_eq!(result.vy, 0.0);
    }
}
