#[derive(Debug, Clone, Copy, Default)]
pub struct MotionState {
    pub vx: f32,
    pub vy: f32,
    pub valid: bool,
}