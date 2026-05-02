use crate::ffi::bindings::{
    VP_INPUT_ACTION, VP_INPUT_ENCODER_A, VP_INPUT_ENCODER_B, VP_INPUT_LASER, VP_INPUT_LEFT,
    VP_INPUT_MIDDLE, VP_INPUT_RIGHT, c_vp_gpio_read,
};
use crate::input::encoder::RotaryEncoder;
use log::info;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InputStatus {
    pub left: bool,
    pub right: bool,
    pub middle: bool,
    pub light: bool,
    pub action: bool,
    pub wheel_delta: i8,
}

pub struct InputManager {
    encoder: RotaryEncoder,
}

impl InputManager {
    pub fn new() -> Self {
        Self {
            encoder: RotaryEncoder::new(),
        }
    }

    pub fn get_current_input(&mut self) -> InputStatus {
        let left = unsafe { c_vp_gpio_read(VP_INPUT_LEFT as u8) } != 0;
        let right = unsafe { c_vp_gpio_read(VP_INPUT_RIGHT as u8) } != 0;
        let middle = unsafe { c_vp_gpio_read(VP_INPUT_MIDDLE as u8) } != 0;
        let action = unsafe { c_vp_gpio_read(VP_INPUT_ACTION as u8) } != 0;
        let light = unsafe { c_vp_gpio_read(VP_INPUT_LASER as u8) } != 0;
        let enc_a = unsafe { c_vp_gpio_read(VP_INPUT_ENCODER_A as u8) } != 0;
        let enc_b = unsafe { c_vp_gpio_read(VP_INPUT_ENCODER_B as u8) } != 0;
        let wheel_delta = self.encoder.update(enc_a, enc_b);
        if wheel_delta != 0 {
            info!("{},{},{}", enc_a, enc_b, wheel_delta);
        }
        InputStatus {
            left,
            right,
            middle,
            light,
            action,
            wheel_delta,
        }
    }
}
