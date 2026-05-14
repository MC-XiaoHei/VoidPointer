use crate::ffi::bindings::c_vp_pwm_set_duty;
use crate::ffi::board_map::BoardSignal;

pub fn set_laser_duty(duty: u8) {
    unsafe { c_vp_pwm_set_duty(BoardSignal::PWM_LASER as u8, duty) };
}
