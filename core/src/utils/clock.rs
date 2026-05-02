use crate::ffi::bindings::{c_vp_rtc_micros, c_vp_rtc_millis};
use fugit::TimerInstantU32;

pub type MillisInstant = TimerInstantU32<1000>;
pub type MicrosInstant = TimerInstantU32<1_000_000>;

pub struct RTC;

impl RTC {
    pub fn millis() -> MillisInstant {
        MillisInstant::from_ticks(unsafe { c_vp_rtc_millis() })
    }

    pub fn micros() -> MicrosInstant {
        MicrosInstant::from_ticks(unsafe { c_vp_rtc_micros() })
    }
}
