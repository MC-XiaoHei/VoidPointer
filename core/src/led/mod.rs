pub mod builder;
pub mod macros;

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
}

#[cfg(test)]
mod tests {
    use crate::led::builder::Segment;
    use crate::{loop_profile, once_profile};

    #[test]
    fn once_profile_ends_with_zero() {
        once_profile!(P, 64, [Segment::Level(50, 20)]);
        assert!(!P.is_loop);
        assert!(P.len > 1);
        assert_eq!(P.data[P.len - 1], 0);
    }

    #[test]
    fn loop_profile_no_trailing_zero() {
        loop_profile!(P, 64, [Segment::Level(50, 20)]);
        assert!(P.is_loop);
        assert!(P.len > 0);
        assert_ne!(P.data[P.len - 1], 0);
    }

    #[test]
    fn profile_as_slice_length() {
        once_profile!(P, 64, [Segment::Level(50, 20)]);
        assert_eq!(P.as_slice().len(), P.len);
    }
}
