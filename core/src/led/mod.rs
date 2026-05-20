pub mod builder;
pub mod macros;
pub mod patterns;
pub mod runtime;

use crate::ffi::bindings::{c_vp_led_play, c_vp_led_stop};
use crate::ffi::board_map::BoardSignal;

/// TICK_MS = 10, 硬件 PWM 每帧持续 10ms, 必须能被 1s 整除
pub const TICK_MS: usize = 10;
const _: () = assert!(1000 % TICK_MS == 0, "TICK_MS 必须能被 1s 整除");

pub struct LedProfile<const N: usize> {
    pub data: [u32; N],
    pub len: usize,
    pub is_loop: bool,
}

impl<const N: usize> LedProfile<N> {
    pub fn as_slice(&self) -> &[u32] {
        &self.data[..self.len]
    }

    pub fn playback_ms(&self) -> u32 {
        self.len as u32 * TICK_MS as u32
    }

    #[cfg_attr(coverage, coverage(off))]
    pub fn play(&'static self, led_sig: BoardSignal) {
        let ptr = self.data.as_ptr();
        let len = self.len as u16;
        let is_loop = if self.is_loop { 1u8 } else { 0u8 };
        unsafe {
            c_vp_led_play(led_sig as u8, ptr, len, is_loop);
        }
    }
}

#[cfg_attr(coverage, coverage(off))]
pub fn stop_playback() {
    unsafe { c_vp_led_stop() }
}

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use crate::led::builder::Segment;
    use crate::led_profile;

    #[test]
    fn once_profile_ends_with_zero() {
        led_profile!(P, once! { Segment::Level(50, 20) });
        assert!(!P.is_loop);
        assert!(P.len > 1);
        assert_eq!(P.data[P.len - 1], 0);
    }

    #[test]
    fn loop_profile_no_trailing_zero() {
        led_profile!(P, repeat! { Segment::Level(50, 20) });
        assert!(P.is_loop);
        assert!(P.len > 0);
        assert_ne!(P.data[P.len - 1], 0);
    }

    #[test]
    fn profile_as_slice_length() {
        led_profile!(P, once! { Segment::Level(50, 20) });
        assert_eq!(P.as_slice().len(), P.len);
    }

    #[test]
    fn playback_ms_computed_correctly() {
        led_profile!(P, once! { Segment::Level(50, 30) });
        assert_eq!(P.playback_ms(), P.len as u32 * crate::led::TICK_MS as u32);
    }
}
