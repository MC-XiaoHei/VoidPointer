use crate::ffi::bindings::VP_LED_PWM_CYCLE;
use crate::led::TICK_MS;

pub enum Segment {
    Level(u8, usize),
    Fade(u8, u8, usize),
}

/// `scale` = PWM 周期计数值，默认取 C 侧 `VP_LED_PWM_CYCLE`
/// 占空比 = value * scale / 255
pub struct LedSequenceBuilder<const N: usize> {
    data: [u32; N],
    pos: usize,
    scale: u32,
}

impl<const N: usize> LedSequenceBuilder<N> {
    pub const fn new() -> Self {
        Self {
            data: [0u32; N],
            pos: 0,
            scale: VP_LED_PWM_CYCLE,
        }
    }

    pub const fn apply(self, segment: Segment) -> Self {
        match segment {
            Segment::Level(value, time_ms) => self.level(value, time_ms),
            Segment::Fade(start, end, time_ms) => self.fade(start, end, time_ms),
        }
    }

    pub const fn level(mut self, value: u8, time_ms: usize) -> Self {
        assert!(
            time_ms % TICK_MS == 0,
            "level: time_ms 必须是 TICK_MS 的整数倍"
        );
        let frames = time_ms / TICK_MS;
        let mut i = 0;
        while i < frames {
            assert!(self.pos < N, "level: 缓冲区溢出");
            self.data[self.pos] = (value as u32 * self.scale) / 255;
            self.pos += 1;
            i += 1;
        }
        self
    }

    pub const fn fade(mut self, start: u8, end: u8, time_ms: usize) -> Self {
        assert!(
            time_ms % TICK_MS == 0,
            "fade: time_ms 必须是 TICK_MS 的整数倍"
        );
        let frames = time_ms / TICK_MS;
        let mut i = 0;
        while i < frames {
            assert!(self.pos < N, "fade: 缓冲区溢出");
            let val = if end > start {
                let diff = end as usize - start as usize;
                start as usize + (diff * i * i) / (frames * frames)
            } else {
                let diff = start as usize - end as usize;
                let j = frames - i;
                end as usize + (diff * j * j) / (frames * frames)
            };
            self.data[self.pos] = (val as u32 * self.scale) / 255;
            self.pos += 1;
            i += 1;
        }
        self
    }

    /// 尾部追加 0，DMA 停止后输出低电平
    pub const fn finish(mut self) -> (usize, [u32; N]) {
        assert!(self.pos < N, "finish: 缓冲区溢出");
        self.data[self.pos] = 0;
        self.pos += 1;
        (self.pos, self.data)
    }

    pub const fn finish_loop(self) -> (usize, [u32; N]) {
        (self.pos, self.data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_equal_frames() {
        let (len, buf) = LedSequenceBuilder::<20>::new().level(100, 30).finish();
        assert_eq!(len, 4);
        assert!(buf[..3].iter().all(|&v| v > 0 && v == buf[0]));
        assert_eq!(buf[3], 0);
    }

    #[test]
    fn fade_up_increases() {
        let (len, buf) = LedSequenceBuilder::<20>::new().fade(0, 100, 20).finish();
        assert_eq!(len, 3);
        assert!(buf[0] <= buf[1]);
        assert_eq!(buf[2], 0);
    }

    #[test]
    fn fade_down_decreases() {
        let (len, buf) = LedSequenceBuilder::<20>::new().fade(100, 0, 20).finish();
        assert_eq!(len, 3);
        assert!(buf[0] >= buf[1]);
        assert_eq!(buf[2], 0);
    }

    #[test]
    fn multiple_segments() {
        let (len, buf) = LedSequenceBuilder::<30>::new()
            .apply(Segment::Level(50, 20))
            .apply(Segment::Fade(50, 0, 10))
            .finish();
        assert_eq!(len, 4);
        assert!(buf[0] > 0 && buf[0] == buf[1]);
        assert_eq!(buf[3], 0);
    }

    #[test]
    fn finish_loop_no_trailing_zero() {
        let (len, buf) = LedSequenceBuilder::<10>::new().level(100, 20).finish_loop();
        assert_eq!(len, 2);
        assert_ne!(buf[1], 0);
    }

    #[test]
    #[should_panic]
    fn level_time_not_divisible() {
        let _ = LedSequenceBuilder::<10>::new().level(50, 15);
    }

    #[test]
    #[should_panic]
    fn fade_time_not_divisible() {
        let _ = LedSequenceBuilder::<10>::new().fade(0, 50, 7);
    }

    #[test]
    #[should_panic]
    fn buffer_overflow_level() {
        let _ = LedSequenceBuilder::<2>::new().level(1, 30).finish();
    }

    #[test]
    #[should_panic]
    fn buffer_overflow_finish_no_room() {
        let _ = LedSequenceBuilder::<1>::new().level(1, 10).finish();
    }
}
