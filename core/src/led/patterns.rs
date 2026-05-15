use crate::{fade, led_profile, level};

led_profile!(
    CONNECTED,
    once! {
        fade!(from 0 to 128 over 300 ms),
        fade!(from 128 to 0 over 800 ms),
    }
);

led_profile!(
    CHARGING,
    repeat! {
        fade!(from 0 to 51 over 1500 ms),
        fade!(from 51 to 0 over 1500 ms),
        level!(0 for 3000 ms),
    }
);

led_profile!(
    LOW_BATTERY,
    repeat! {
        level!(102 for 150 ms),
        level!(0 for 150 ms),
        level!(102 for 150 ms),
        level!(0 for 5000 ms),
    }
);

led_profile!(
    MODE_BLE,
    once! {
        fade!(from 0 to 153 over 200 ms),
        fade!(from 153 to 0 over 200 ms),
        fade!(from 0 to 153 over 200 ms),
        fade!(from 153 to 0 over 200 ms),
        fade!(from 0 to 153 over 200 ms),
        fade!(from 153 to 0 over 200 ms),
    }
);

led_profile!(
    MODE_2G4,
    once! {
        level!(204 for 150 ms),
        level!(0 for 150 ms),
        level!(204 for 150 ms),
        level!(0 for 150 ms),
        level!(204 for 150 ms),
        level!(0 for 150 ms),
    }
);
