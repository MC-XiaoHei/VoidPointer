use crate::ffi::bindings::{VP_PWM_ID_LASER, c_vp_pwm_set_duty};

pub fn set_laser_duty(duty: u8) {
    unsafe { c_vp_pwm_set_duty(VP_PWM_ID_LASER as u8, duty) };
}
