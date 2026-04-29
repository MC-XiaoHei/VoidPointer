use crate::bindings::c_get_input_status;
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
        let raw = unsafe { c_get_input_status() };
        let wheel_delta = self.encoder.update(raw.enc_a, raw.enc_b);
        if wheel_delta != 0 {
            info!("{},{},{}", raw.enc_a, raw.enc_b, wheel_delta);
        }
        InputStatus {
            left: raw.left,
            right: raw.right,
            middle: raw.middle,
            light: raw.light,
            action: raw.action,
            wheel_delta,
        }
    }
}
