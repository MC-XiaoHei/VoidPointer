/// 生成单次播放的 `LedProfile`，播放完毕自动熄灭
///
/// ```rs
/// once_profile!(CONNECTED, 256, [
///     Segment::Fade(0, 50, 300),
///     Segment::Level(50, 100),
///     Segment::Fade(50, 0, 800),
/// ]);
/// ```
#[macro_export]
macro_rules! once_profile {
    ($name:ident, $max:expr, [$($segment:expr),* $(,)?]) => {
        const ONCE_BLD: (usize, [u32; $max]) =
            $crate::led::builder::LedSequenceBuilder::new()
            $( .apply($segment) )*
            .finish();
        static $name: $crate::led::LedProfile<$max> = $crate::led::LedProfile {
            data: ONCE_BLD.1,
            len: ONCE_BLD.0,
            is_loop: false,
        };
    };
}

/// 生成循环播放的 `LedProfile`
///
/// ```rs
/// loop_profile!(BREATHING, 256, [
///     Segment::Fade(0, 50, 300),
///     Segment::Fade(50, 0, 300),
/// ]);
/// ```
#[macro_export]
macro_rules! loop_profile {
    ($name:ident, $max:expr, [$($segment:expr),* $(,)?]) => {
        const LOOP_BLD: (usize, [u32; $max]) =
            $crate::led::builder::LedSequenceBuilder::new()
            $( .apply($segment) )*
            .finish_loop();
        static $name: $crate::led::LedProfile<$max> = $crate::led::LedProfile {
            data: LOOP_BLD.1,
            len: LOOP_BLD.0,
            is_loop: true,
        };
    };
}
