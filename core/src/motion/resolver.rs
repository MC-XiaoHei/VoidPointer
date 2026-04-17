use crate::attitude::get_current_attitude;
use crate::attitude::types::AttitudeData;
use crate::motion::config::MotionConfig;
use crate::motion::state::MotionState;

const ROLL_SIGN: i8 = 1;
const PITCH_SIGN: i8 = 1;

pub struct TiltMotionSolver {
    cfg: MotionConfig,
    filtered_vx: f32,
    filtered_vy: f32,

    pub center_roll: f32,
    pub center_pitch: f32,
}

impl TiltMotionSolver {
    pub fn new(cfg: MotionConfig) -> Self {
        let mut result = Self {
            cfg,
            filtered_vx: 0.0,
            filtered_vy: 0.0,
            center_roll: 0.0,
            center_pitch: 0.0,
        };

        let attitude = get_current_attitude().unwrap();

        result.calibrate(attitude);

        result
    }

    pub fn calibrate(&mut self, attitude: AttitudeData) {
        self.center_roll = attitude.roll;
        self.center_pitch = attitude.pitch;
    }

    pub fn update(&mut self, attitude: AttitudeData) -> MotionState {
        let roll_diff = normalize_angle(attitude.roll - self.center_roll);
        let pitch_diff = normalize_angle(attitude.pitch - self.center_pitch);

        let roll = roll_diff * ROLL_SIGN as f32;
        let pitch = pitch_diff * PITCH_SIGN as f32;

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