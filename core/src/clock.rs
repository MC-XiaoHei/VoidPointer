use crate::bindings::{c_get_rtc_micros, c_get_rtc_millis};
use fugit::TimerInstantU32;

pub type MillisInstant = TimerInstantU32<1000>;
pub type MicrosInstant = TimerInstantU32<1_000_000>;

pub struct RTC;

impl RTC {
    pub fn millis() -> MillisInstant {
        MillisInstant::from_ticks(unsafe { c_get_rtc_millis() })
    }

    pub fn micros() -> MicrosInstant {
        MicrosInstant::from_ticks(unsafe { c_get_rtc_micros() })
    }
}
