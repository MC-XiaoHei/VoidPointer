#[derive(Debug, Clone, Copy)]
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
            deadzone_x_rad: 0.1,
            deadzone_y_rad: 0.1,
            deadzone_speed: 0.1,
            max_angle_rad: 0.5,
            sensitivity_x: 1200.0,
            sensitivity_y: 1200.0,
            invert_x: false,
            invert_y: false,
            smoothing_alpha: 0.2,
        }
    }
}