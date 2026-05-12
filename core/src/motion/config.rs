use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MotionConfig {
    pub deadzone_x_rad: f32,
    pub deadzone_y_rad: f32,
    pub deadzone_speed: f32,
    pub max_angle_rad: f32,
    pub sensitivity_x: f32,
    pub sensitivity_y: f32,
    pub invert_x: bool,
    pub invert_y: bool,
    pub smoothing_alpha: f32,
}

impl Default for MotionConfig {
    fn default() -> Self {
        Self {
            deadzone_x_rad: 0.05,
            deadzone_y_rad: 0.05,
            deadzone_speed: 0.1,
            max_angle_rad: 1.0,
            sensitivity_x: 12000.0,
            sensitivity_y: 12000.0,
            invert_x: false,
            invert_y: false,
            smoothing_alpha: 0.2,
        }
    }
}
