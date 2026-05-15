pub fn voidpointer_board() -> Vec<Def> {
    use Drive::*;
    use Func::*;
    use PinLevel::*;
    use Polarity::*;
    use Port::*;
    use Pull::*;

    vec![
        Def {
            name: "btn_left",
            pin: (A, 9),
            func: In {
                pull: PullUp,
                digital: true,
            },
        },
        Def {
            name: "btn_right",
            pin: (A, 8),
            func: In {
                pull: PullUp,
                digital: true,
            },
        },
        Def {
            name: "btn_middle",
            pin: (A, 5),
            func: In {
                pull: PullUp,
                digital: true,
            },
        },
        Def {
            name: "btn_action",
            pin: (A, 10),
            func: In {
                pull: PullUp,
                digital: true,
            },
        },
        Def {
            name: "btn_laser",
            pin: (A, 7),
            func: In {
                pull: PullUp,
                digital: true,
            },
        },
        Def {
            name: "mode_switch",
            pin: (A, 0),
            func: Out {
                drive: PP5mA,
                init: Some(Low),
            },
        },
        Def {
            name: "enc_a",
            pin: (A, 4),
            func: In {
                pull: PullUp,
                digital: true,
            },
        },
        Def {
            name: "enc_b",
            pin: (A, 6),
            func: In {
                pull: PullUp,
                digital: true,
            },
        },
        Def {
            name: "imu_int1",
            pin: (A, 11),
            func: In {
                pull: PullUp,
                digital: true,
            },
        },
        Def {
            name: "imu_int2",
            pin: (A, 12),
            func: In {
                pull: PullUp,
                digital: true,
            },
        },
        Def {
            name: "led_status",
            pin: (A, 2),
            func: Tmr {
                id: 3,
                polar: ActiveLow,
            },
        },
        Def {
            name: "pwm_laser",
            pin: (B, 1),
            func: Pwm {
                id: 7,
                polar: ActiveLow,
            },
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
            name: "debug_tx",
            pin: (A, 14),
            func: Uart(0),
        },
        Def {
            name: "debug_rx",
            pin: (A, 15),
            func: Uart(0),
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
