/// 语义化渐亮/渐暗段
///
/// ```rs
/// fade!(from 0 to 128 over 300 ms)
/// ```
#[macro_export]
macro_rules! fade {
    (from $start:tt to $end:tt over $time:tt ms) => {
        $crate::led::builder::Segment::Fade($start, $end, $time)
    };
}

/// 语义化恒定亮度段
///
/// ```rs
/// level!(102 for 150 ms)
/// ```
#[macro_export]
macro_rules! level {
    ($value:tt for $time:tt ms) => {
        $crate::led::builder::Segment::Level($value, $time)
    };
}

/// 内容透传，配合 `led_profile!` 表示单次播放。
/// `led_profile!(x, once!{ ... })` 播完自动熄灭。
#[macro_export]
macro_rules! once {
    ({ $($inner:tt)* }) => { $($inner)* };
}

/// 内容透传，配合 `led_profile!` 表示循环播放。
/// `led_profile!(x, repeat!{ ... })` 持续循环。
#[macro_export]
macro_rules! repeat {
    ({ $($inner:tt)* }) => { $($inner)* };
}

/// 生成 `LedProfile`，`once!{ }` 单次播放，`repeat!{ }` 循环播放
///
/// ```rs
/// led_profile!(CONNECTED, once!{
///     fade!(from 0 to 128 over 300 ms),
///     fade!(from 128 to 0 over 800 ms),
/// });
///
/// led_profile!(CHARGING, repeat!{
///     fade!(from 0 to 51 over 1500 ms),
///     fade!(from 51 to 0 over 1500 ms),
///     level!(0 for 3000 ms),
/// });
/// ```
#[macro_export]
macro_rules! led_profile {
    ($name:ident, once! { $($segment:expr),* $(,)? }) => {
        pub static $name: $crate::led::LedProfile<1024> = {
            let (len, data) = $crate::led::builder::LedSequenceBuilder::new()
                $( .apply($segment) )*
                .finish();
            $crate::led::LedProfile {
                data,
                len,
                is_loop: false,
            }
        };
    };
    ($name:ident, repeat! { $($segment:expr),* $(,)? }) => {
        pub static $name: $crate::led::LedProfile<1024> = {
            let (len, data) = $crate::led::builder::LedSequenceBuilder::new()
                $( .apply($segment) )*
                .finish_loop();
            $crate::led::LedProfile {
                data,
                len,
                is_loop: true,
            }
        };
    };
}
