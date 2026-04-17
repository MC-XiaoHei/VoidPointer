use crate::attitude::types::AttitudeData;
use crate::motion::config::MotionConfig;
use crate::motion::state::MotionState;

const ROLL_SIGN: i8 = 1;
const PITCH_SIGN: i8 = 1;
const YAW_SIGN: i8 = 1;

pub struct TiltMotionSolver {
    cfg: MotionConfig,
    filtered_vx: f32,
    filtered_vy: f32,
}

impl TiltMotionSolver {
    pub fn new(cfg: MotionConfig) -> Self {
        Self {
            cfg,
            filtered_vx: 0.0,
            filtered_vy: 0.0,
        }
    }

    pub fn update(&mut self, attitude: AttitudeData) -> MotionState {
        let roll = attitude.roll * ROLL_SIGN as f32;
        let pitch = attitude.pitch * PITCH_SIGN as f32;
        let _yaw = attitude.yaw * YAW_SIGN as f32;

        let raw_x = roll;
        let raw_y = pitch;

        let x = if self.cfg.invert_x { -raw_x } else { raw_x };
        let y = if self.cfg.invert_y { -raw_y } else { raw_y };

        let nx = normalize_axis(x, self.cfg.deadzone_x_rad, self.cfg.max_angle_rad);
        let ny = normalize_axis(y, self.cfg.deadzone_y_rad, self.cfg.max_angle_rad);

        let target_vx = nx * self.cfg.sensitivity_x;
        let target_vy = ny * self.cfg.sensitivity_y;

        self.filtered_vx += self.cfg.smoothing_alpha * (target_vx - self.filtered_vx);
        self.filtered_vy += self.cfg.smoothing_alpha * (target_vy - self.filtered_vy);

        if libm::fabsf(self.filtered_vx) < 0.001 {
            self.filtered_vx = 0.0;
        }
        if libm::fabsf(self.filtered_vy) < 0.001 {
            self.filtered_vy = 0.0;
        }

        MotionState {
            vx: self.filtered_vx,
            vy: self.filtered_vy,
            valid: true,
        }
    }
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
        sign * ((abs - deadzone) / (max_angle - deadzone))
    }
}