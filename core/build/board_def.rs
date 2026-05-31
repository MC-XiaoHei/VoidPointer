use crate::board_def::UsbSpeed::HighSpeed;

pub fn voidpointer_board() -> Vec<Def> {
    use Drive::*;
    use Func::*;
    use PinLevel::*;
    use Polarity::*;
    use Port::*;
    use Pull::*;

    vec![
        Def {
            name: "power_mode",
            pin: (B, 9),
            func: Out {
                drive: PP5mA,
                init: Some(Low),
            },
        },
        Def {
            name: "imu_int1",
            pin: (B, 16),
            func: In {
                pull: PullUp,
                digital: true,
            },
        },
        Def {
            name: "imu_int2",
            pin: (B, 17),
            func: In {
                pull: PullUp,
                digital: true,
            },
        },
        Def {
            name: "usb_dp",
            pin: (B, 13),
            func: Usb(HighSpeed),
        },
        Def {
            name: "usb_dn",
            pin: (B, 12),
            func: Usb(HighSpeed),
        },
        Def {
            name: "profile_switch",
            pin: (B, 10),
            func: In {
                pull: PullUp,
                digital: true,
            },
        },
        Def {
            name: "mode_switch",
            pin: (B, 7),
            func: In {
                pull: PullUp,
                digital: true,
            },
        },
        Def {
            name: "right_button",
            pin: (B, 6),
            func: In {
                pull: PullUp,
                digital: true,
            },
        },
        Def {
            name: "left_button",
            pin: (B, 5),
            func: In {
                pull: PullUp,
                digital: true,
            },
        },
        Def {
            name: "down_button",
            pin: (B, 4),
            func: In {
                pull: PullUp,
                digital: true,
            },
        },
        Def {
            name: "up_button",
            pin: (B, 3),
            func: In {
                pull: PullUp,
                digital: true,
            },
        },
        Def {
            name: "act_button",
            pin: (B, 2),
            func: In {
                pull: PullUp,
                digital: true,
            },
        },
        Def {
            name: "context_button",
            pin: (B, 1),
            func: In {
                pull: PullUp,
                digital: true,
            },
        },
        Def {
            name: "laser_led",
            pin: (B, 0),
            func: Pwm {
                id: 6,
                polar: ActiveHigh,
            },
        },
        Def {
            name: "battery_sensor",
            pin: (A, 12),
            func: Adc(2),
        },
        Def {
            name: "debug_rx",
            pin: (A, 15),
            func: Uart(0),
        },
        Def {
            name: "debug_tx",
            pin: (A, 14),
            func: Uart(0),
        },
        Def {
            name: "status_led",
            pin: (A, 2),
            func: Tmr {
                id: 3,
                polar: ActiveLow,
            },
        },
        Def {
            name: "charge_status",
            pin: (A, 5),
            func: Adc(1),
        },
        Def {
            name: "i2c_sda",
            pin: (B, 20),
            func: I2c,
        },
        Def {
            name: "i2c_scl",
            pin: (B, 21),
            func: I2c,
        },
        Def {
            name: "battery_sensor_enable",
            pin: (B, 22),
            func: Out {
                drive: PP5mA,
                init: Some(Low),
            },
        },
    ]
}

#[allow(unused)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pull {
    Floating,
    PullUp,
    PullDown,
}

#[allow(unused)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Drive {
    PP5mA,
    PP20mA,
}

#[allow(unused)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinLevel {
    Low,
    High,
}

#[allow(unused)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsbSpeed {
    FullSpeed,
    HighSpeed,
}

#[allow(unused)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Polarity {
    ActiveHigh,
    ActiveLow,
}

#[allow(unused)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Func {
    In {
        pull: Pull,
        digital: bool,
    },
    Out {
        drive: Drive,
        init: Option<PinLevel>,
    },
    Pwm {
        id: u8,
        polar: Polarity,
    },
    Tmr {
        id: u8,
        polar: Polarity,
    },
    Cap(u8),
    I2c,
    Spi(u8),
    Usb(UsbSpeed),
    Nfc,
    X32k,
    Led(u8),
    Uart(u8),
    Adc(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Port {
    A,
    B,
}

#[derive(Debug, Clone)]
pub struct Def {
    pub name: &'static str,
    pub pin: (Port, u8),
    pub func: Func,
}

impl Def {
    pub fn port(&self) -> Port {
        self.pin.0
    }

    pub fn pin_num(&self) -> u8 {
        self.pin.1
    }

    pub fn channel(&self) -> Option<u8> {
        match self.func {
            Func::Pwm { id: ch, .. }
            | Func::Tmr { id: ch, .. }
            | Func::Cap(ch)
            | Func::Uart(ch)
            | Func::Led(ch)
            | Func::Adc(ch)
            | Func::Spi(ch) => Some(ch),
            _ => None,
        }
    }
}
