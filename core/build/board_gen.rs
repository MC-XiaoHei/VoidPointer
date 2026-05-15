use crate::board_def::*;

pub struct HwConfig {
    pub mode: &'static str,
    pub digital: bool,
    pub remap: Option<(&'static str, bool)>,
    pub polarity: Option<bool>,
}

fn signal_name_to_enum(name: &str) -> String {
    format!("BOARD_SIGNAL_{}", name.to_uppercase())
}

fn port_char(port: Port) -> char {
    match port {
        Port::A => 'A',
        Port::B => 'B',
    }
}

fn gpio_port_prefix(port: Port) -> &'static str {
    match port {
        Port::A => "GPIOA",
        Port::B => "GPIOB",
    }
}

fn pin_mask(pin: u8) -> String {
    format!("GPIO_Pin_{}", pin)
}

pub fn derive_hw_config(def: &Def) -> HwConfig {
    use Drive::*;
    use Func::*;
    use Pull::*;
    use UsbSpeed::*;

    match (def.port(), def.pin_num(), def.func) {
        (
            _,
            _,
            In {
                pull: Floating,
                digital: d,
            },
        ) => HwConfig {
            mode: "GPIO_ModeIN_Floating",
            digital: d,
            remap: None,
            polarity: None,
        },
        (
            _,
            _,
            In {
                pull: PullUp,
                digital: d,
            },
        ) => HwConfig {
            mode: "GPIO_ModeIN_PU",
            digital: d,
            remap: None,
            polarity: None,
        },
        (
            _,
            _,
            In {
                pull: PullDown,
                digital: d,
            },
        ) => HwConfig {
            mode: "GPIO_ModeIN_PD",
            digital: d,
            remap: None,
            polarity: None,
        },
        (_, _, Out { drive: PP5mA, .. }) => HwConfig {
            mode: "GPIO_ModeOut_PP_5mA",
            digital: false,
            remap: None,
            polarity: None,
        },
        (_, _, Out { drive: PP20mA, .. }) => HwConfig {
            mode: "GPIO_ModeOut_PP_20mA",
            digital: false,
            remap: None,
            polarity: None,
        },

        // TMR0: PA9 → PB23
        (Port::A, 9, Tmr { id: 0, polar }) => HwConfig {
            mode: "GPIO_ModeOut_PP_5mA",
            digital: false,
            remap: Some(("RB_PIN_TMR0", false)),
            polarity: Some(polar == Polarity::ActiveLow),
        },
        (Port::B, 23, Tmr { id: 0, polar }) => HwConfig {
            mode: "GPIO_ModeOut_PP_5mA",
            digital: false,
            remap: Some(("RB_PIN_TMR0", true)),
            polarity: Some(polar == Polarity::ActiveLow),
        },

        // CAP0: PA9 → PB23
        (Port::A, 9, Cap(0)) => HwConfig {
            mode: "GPIO_ModeIN_Floating",
            digital: true,
            remap: Some(("RB_PIN_TMR0", false)),
            polarity: None,
        },
        (Port::B, 23, Cap(0)) => HwConfig {
            mode: "GPIO_ModeIN_Floating",
            digital: true,
            remap: Some(("RB_PIN_TMR0", true)),
            polarity: None,
        },

        // TMR1: PA10 → PB10
        (Port::A, 10, Tmr { id: 1, polar }) => HwConfig {
            mode: "GPIO_ModeOut_PP_5mA",
            digital: false,
            remap: Some(("RB_PIN_TMR1", false)),
            polarity: Some(polar == Polarity::ActiveLow),
        },
        (Port::B, 10, Tmr { id: 1, polar }) => HwConfig {
            mode: "GPIO_ModeOut_PP_5mA",
            digital: false,
            remap: Some(("RB_PIN_TMR1", true)),
            polarity: Some(polar == Polarity::ActiveLow),
        },
        (Port::A, 10, Cap(1)) => HwConfig {
            mode: "GPIO_ModeIN_Floating",
            digital: true,
            remap: Some(("RB_PIN_TMR1", false)),
            polarity: None,
        },
        (Port::B, 10, Cap(1)) => HwConfig {
            mode: "GPIO_ModeIN_Floating",
            digital: true,
            remap: Some(("RB_PIN_TMR1", true)),
            polarity: None,
        },

        // TMR2: PA11 → PB11
        (Port::A, 11, Tmr { id: 2, polar }) => HwConfig {
            mode: "GPIO_ModeOut_PP_5mA",
            digital: false,
            remap: Some(("RB_PIN_TMR2", false)),
            polarity: Some(polar == Polarity::ActiveLow),
        },
        (Port::B, 11, Tmr { id: 2, polar }) => HwConfig {
            mode: "GPIO_ModeOut_PP_5mA",
            digital: false,
            remap: Some(("RB_PIN_TMR2", true)),
            polarity: Some(polar == Polarity::ActiveLow),
        },
        (Port::A, 11, Cap(2)) => HwConfig {
            mode: "GPIO_ModeIN_Floating",
            digital: true,
            remap: Some(("RB_PIN_TMR2", false)),
            polarity: None,
        },
        (Port::B, 11, Cap(2)) => HwConfig {
            mode: "GPIO_ModeIN_Floating",
            digital: true,
            remap: Some(("RB_PIN_TMR2", true)),
            polarity: None,
        },

        // TMR3: PB22 → PA2
        (Port::B, 22, Tmr { id: 3, polar }) => HwConfig {
            mode: "GPIO_ModeOut_PP_5mA",
            digital: false,
            remap: Some(("RB_PIN_TMR3", false)),
            polarity: Some(polar == Polarity::ActiveLow),
        },
        (Port::A, 2, Tmr { id: 3, polar }) => HwConfig {
            mode: "GPIO_ModeOut_PP_5mA",
            digital: false,
            remap: Some(("RB_PIN_TMR3", true)),
            polarity: Some(polar == Polarity::ActiveLow),
        },
        (Port::B, 22, Cap(3)) => HwConfig {
            mode: "GPIO_ModeIN_Floating",
            digital: true,
            remap: Some(("RB_PIN_TMR3", false)),
            polarity: None,
        },
        (Port::A, 2, Cap(3)) => HwConfig {
            mode: "GPIO_ModeIN_Floating",
            digital: true,
            remap: Some(("RB_PIN_TMR3", true)),
            polarity: None,
        },

        // PWM4: PA12 → PA6
        (Port::A, 12, Pwm { id: 4, polar }) => HwConfig {
            mode: "GPIO_ModeOut_PP_5mA",
            digital: false,
            remap: Some(("RB_PIN_PWMX", false)),
            polarity: Some(polar == Polarity::ActiveLow),
        },
        (Port::A, 6, Pwm { id: 4, polar }) => HwConfig {
            mode: "GPIO_ModeOut_PP_5mA",
            digital: false,
            remap: Some(("RB_PIN_PWMX", true)),
            polarity: Some(polar == Polarity::ActiveLow),
        },

        // PWM5: PA13 → PA7
        (Port::A, 13, Pwm { id: 5, polar }) => HwConfig {
            mode: "GPIO_ModeOut_PP_5mA",
            digital: false,
            remap: Some(("RB_PIN_PWMX", false)),
            polarity: Some(polar == Polarity::ActiveLow),
        },
        (Port::A, 7, Pwm { id: 5, polar }) => HwConfig {
            mode: "GPIO_ModeOut_PP_5mA",
            digital: false,
            remap: Some(("RB_PIN_PWMX", true)),
            polarity: Some(polar == Polarity::ActiveLow),
        },

        // PWM7: PB4 → PB1
        (Port::B, 4, Pwm { id: 7, polar }) => HwConfig {
            mode: "GPIO_ModeOut_PP_5mA",
            digital: false,
            remap: Some(("RB_PIN_PWMX", false)),
            polarity: Some(polar == Polarity::ActiveLow),
        },
        (Port::B, 1, Pwm { id: 7, polar }) => HwConfig {
            mode: "GPIO_ModeOut_PP_5mA",
            digital: false,
            remap: Some(("RB_PIN_PWMX", true)),
            polarity: Some(polar == Polarity::ActiveLow),
        },

        // PWM8: PB6 → PB2
        (Port::B, 6, Pwm { id: 8, polar }) => HwConfig {
            mode: "GPIO_ModeOut_PP_5mA",
            digital: false,
            remap: Some(("RB_PIN_PWMX", false)),
            polarity: Some(polar == Polarity::ActiveLow),
        },
        (Port::B, 2, Pwm { id: 8, polar }) => HwConfig {
            mode: "GPIO_ModeOut_PP_5mA",
            digital: false,
            remap: Some(("RB_PIN_PWMX", true)),
            polarity: Some(polar == Polarity::ActiveLow),
        },

        // PWM9: PB7 → PB3
        (Port::B, 7, Pwm { id: 9, polar }) => HwConfig {
            mode: "GPIO_ModeOut_PP_5mA",
            digital: false,
            remap: Some(("RB_PIN_PWMX", false)),
            polarity: Some(polar == Polarity::ActiveLow),
        },
        (Port::B, 3, Pwm { id: 9, polar }) => HwConfig {
            mode: "GPIO_ModeOut_PP_5mA",
            digital: false,
            remap: Some(("RB_PIN_PWMX", true)),
            polarity: Some(polar == Polarity::ActiveLow),
        },

        // PWM6: PB0
        (Port::B, 0, Pwm { id: 6, polar }) => HwConfig {
            mode: "GPIO_ModeOut_PP_5mA",
            digital: false,
            remap: None,
            polarity: Some(polar == Polarity::ActiveLow),
        },

        // PWM10: PB14
        (Port::B, 14, Pwm { id: 10, polar }) => HwConfig {
            mode: "GPIO_ModeOut_PP_5mA",
            digital: false,
            remap: None,
            polarity: Some(polar == Polarity::ActiveLow),
        },

        // PWM11: PB23
        (Port::B, 23, Pwm { id: 11, polar }) => HwConfig {
            mode: "GPIO_ModeOut_PP_5mA",
            digital: false,
            remap: None,
            polarity: Some(polar == Polarity::ActiveLow),
        },

        (Port::B, 12, I2c) | (Port::B, 20, I2c) => HwConfig {
            mode: "GPIO_ModeIN_PU",
            digital: true,
            remap: Some(i2c_remap(def.port(), def.pin_num())),
            polarity: None,
        },
        (Port::B, 13, I2c) | (Port::B, 21, I2c) => HwConfig {
            mode: "GPIO_ModeIN_PU",
            digital: true,
            remap: Some(i2c_remap(def.port(), def.pin_num())),
            polarity: None,
        },

        // UART0 TX: PB7 → PA14
        (Port::B, 7, Uart(0)) => HwConfig {
            mode: "GPIO_ModeOut_PP_5mA",
            digital: false,
            remap: Some(("RB_PIN_UART0", false)),
            polarity: None,
        },
        (Port::A, 14, Uart(0)) => HwConfig {
            mode: "GPIO_ModeOut_PP_5mA",
            digital: false,
            remap: Some(("RB_PIN_UART0", true)),
            polarity: None,
        },
        // UART0 RX: PB4 → PA15
        (Port::B, 4, Uart(0)) => HwConfig {
            mode: "GPIO_ModeIN_PU",
            digital: false,
            remap: Some(("RB_PIN_UART0", false)),
            polarity: None,
        },
        (Port::A, 15, Uart(0)) => HwConfig {
            mode: "GPIO_ModeIN_PU",
            digital: false,
            remap: Some(("RB_PIN_UART0", true)),
            polarity: None,
        },

        // UART1 TX: PA9 → PB13
        (Port::A, 9, Uart(1)) => HwConfig {
            mode: "GPIO_ModeOut_PP_5mA",
            digital: false,
            remap: Some(("RB_PIN_UART1", false)),
            polarity: None,
        },
        (Port::B, 13, Uart(1)) => HwConfig {
            mode: "GPIO_ModeOut_PP_5mA",
            digital: false,
            remap: Some(("RB_PIN_UART1", true)),
            polarity: None,
        },
        // UART1 RX: PA8 → PB12
        (Port::A, 8, Uart(1)) => HwConfig {
            mode: "GPIO_ModeIN_PU",
            digital: false,
            remap: Some(("RB_PIN_UART1", false)),
            polarity: None,
        },
        (Port::B, 12, Uart(1)) => HwConfig {
            mode: "GPIO_ModeIN_PU",
            digital: false,
            remap: Some(("RB_PIN_UART1", true)),
            polarity: None,
        },

        // UART2 TX: PA7 → PB23
        (Port::A, 7, Uart(2)) => HwConfig {
            mode: "GPIO_ModeOut_PP_5mA",
            digital: false,
            remap: Some(("RB_PIN_UART2", false)),
            polarity: None,
        },
        (Port::B, 23, Uart(2)) => HwConfig {
            mode: "GPIO_ModeOut_PP_5mA",
            digital: false,
            remap: Some(("RB_PIN_UART2", true)),
            polarity: None,
        },
        // UART2 RX: PA6 → PB22
        (Port::A, 6, Uart(2)) => HwConfig {
            mode: "GPIO_ModeIN_PU",
            digital: false,
            remap: Some(("RB_PIN_UART2", false)),
            polarity: None,
        },
        (Port::B, 22, Uart(2)) => HwConfig {
            mode: "GPIO_ModeIN_PU",
            digital: false,
            remap: Some(("RB_PIN_UART2", true)),
            polarity: None,
        },

        // UART3 TX: PA5 → PB21
        (Port::A, 5, Uart(3)) => HwConfig {
            mode: "GPIO_ModeOut_PP_5mA",
            digital: false,
            remap: Some(("RB_PIN_UART3", false)),
            polarity: None,
        },
        (Port::B, 21, Uart(3)) => HwConfig {
            mode: "GPIO_ModeOut_PP_5mA",
            digital: false,
            remap: Some(("RB_PIN_UART3", true)),
            polarity: None,
        },
        // UART3 RX: PA4 → PB20
        (Port::A, 4, Uart(3)) => HwConfig {
            mode: "GPIO_ModeIN_PU",
            digital: false,
            remap: Some(("RB_PIN_UART3", false)),
            polarity: None,
        },
        (Port::B, 20, Uart(3)) => HwConfig {
            mode: "GPIO_ModeIN_PU",
            digital: false,
            remap: Some(("RB_PIN_UART3", true)),
            polarity: None,
        },

        // SPI0: PA12-PA15 → PB12-PB15
        (Port::A, 12, Spi(0)) => HwConfig {
            mode: "GPIO_ModeOut_PP_5mA",
            digital: false,
            remap: Some(("RB_PIN_SPI0", false)),
            polarity: None,
        },
        (Port::A, 13, Spi(0)) => HwConfig {
            mode: "GPIO_ModeOut_PP_5mA",
            digital: false,
            remap: Some(("RB_PIN_SPI0", false)),
            polarity: None,
        },
        (Port::A, 14, Spi(0)) => HwConfig {
            mode: "GPIO_ModeOut_PP_5mA",
            digital: false,
            remap: Some(("RB_PIN_SPI0", false)),
            polarity: None,
        },
        (Port::A, 15, Spi(0)) => HwConfig {
            mode: "GPIO_ModeIN_PU",
            digital: false,
            remap: Some(("RB_PIN_SPI0", false)),
            polarity: None,
        },
        (Port::B, 12, Spi(0)) => HwConfig {
            mode: "GPIO_ModeOut_PP_5mA",
            digital: false,
            remap: Some(("RB_PIN_SPI0", true)),
            polarity: None,
        },
        (Port::B, 13, Spi(0)) => HwConfig {
            mode: "GPIO_ModeOut_PP_5mA",
            digital: false,
            remap: Some(("RB_PIN_SPI0", true)),
            polarity: None,
        },
        (Port::B, 14, Spi(0)) => HwConfig {
            mode: "GPIO_ModeOut_PP_5mA",
            digital: false,
            remap: Some(("RB_PIN_SPI0", true)),
            polarity: None,
        },
        (Port::B, 15, Spi(0)) => HwConfig {
            mode: "GPIO_ModeIN_PU",
            digital: false,
            remap: Some(("RB_PIN_SPI0", true)),
            polarity: None,
        },

        // SPI1: PA0-PA2
        (Port::A, 0, Spi(1)) => HwConfig {
            mode: "GPIO_ModeOut_PP_5mA",
            digital: false,
            remap: None,
            polarity: None,
        },
        (Port::A, 1, Spi(1)) => HwConfig {
            mode: "GPIO_ModeOut_PP_5mA",
            digital: false,
            remap: None,
            polarity: None,
        },
        (Port::A, 2, Spi(1)) => HwConfig {
            mode: "GPIO_ModeIN_PU",
            digital: false,
            remap: None,
            polarity: None,
        },

        // USB FS: PB10/PB11
        (Port::B, 10, Usb(FullSpeed)) => HwConfig {
            mode: "GPIO_ModeIN_PU",
            digital: false,
            remap: None,
            polarity: None,
        },
        (Port::B, 11, Usb(FullSpeed)) => HwConfig {
            mode: "GPIO_ModeIN_PU",
            digital: false,
            remap: None,
            polarity: None,
        },
        // USB HS: PB12/PB13
        (Port::B, 12, Usb(HighSpeed)) => HwConfig {
            mode: "GPIO_ModeIN_PU",
            digital: false,
            remap: None,
            polarity: None,
        },
        (Port::B, 13, Usb(HighSpeed)) => HwConfig {
            mode: "GPIO_ModeIN_PU",
            digital: false,
            remap: None,
            polarity: None,
        },

        // NFC: PB8/PB9/PB16/PB17
        (Port::B, 8, Nfc) => HwConfig {
            mode: "GPIO_ModeIN_PU",
            digital: false,
            remap: None,
            polarity: None,
        },
        (Port::B, 9, Nfc) => HwConfig {
            mode: "GPIO_ModeIN_PU",
            digital: false,
            remap: None,
            polarity: None,
        },
        (Port::B, 16, Nfc) => HwConfig {
            mode: "GPIO_ModeIN_PU",
            digital: false,
            remap: None,
            polarity: None,
        },
        (Port::B, 17, Nfc) => HwConfig {
            mode: "GPIO_ModeIN_PU",
            digital: false,
            remap: None,
            polarity: None,
        },

        // X32K: PA10/PA11
        (Port::A, 10, X32k) => HwConfig {
            mode: "GPIO_ModeIN_Floating",
            digital: false,
            remap: None,
            polarity: None,
        },
        (Port::A, 11, X32k) => HwConfig {
            mode: "GPIO_ModeIN_Floating",
            digital: false,
            remap: None,
            polarity: None,
        },

        // LED: PA0-PA15
        (Port::A, 0, Led(0)) => HwConfig {
            mode: "GPIO_ModeOut_PP_5mA",
            digital: false,
            remap: None,
            polarity: None,
        },
        (Port::A, 1, Led(1)) => HwConfig {
            mode: "GPIO_ModeOut_PP_5mA",
            digital: false,
            remap: None,
            polarity: None,
        },
        (Port::A, 2, Led(2)) => HwConfig {
            mode: "GPIO_ModeOut_PP_5mA",
            digital: false,
            remap: None,
            polarity: None,
        },
        (Port::A, 3, Led(3)) => HwConfig {
            mode: "GPIO_ModeOut_PP_5mA",
            digital: false,
            remap: None,
            polarity: None,
        },
        (Port::A, 4, Led(8)) => HwConfig {
            mode: "GPIO_ModeOut_PP_5mA",
            digital: false,
            remap: None,
            polarity: None,
        },
        (Port::A, 5, Led(4)) => HwConfig {
            mode: "GPIO_ModeOut_PP_5mA",
            digital: false,
            remap: None,
            polarity: None,
        },
        (Port::A, 6, Led(5)) => HwConfig {
            mode: "GPIO_ModeOut_PP_5mA",
            digital: false,
            remap: None,
            polarity: None,
        },
        (Port::A, 7, Led(6)) => HwConfig {
            mode: "GPIO_ModeOut_PP_5mA",
            digital: false,
            remap: None,
            polarity: None,
        },
        (Port::A, 8, Led(7)) => HwConfig {
            mode: "GPIO_ModeOut_PP_5mA",
            digital: false,
            remap: None,
            polarity: None,
        },

        // ADC: 14 通道
        (Port::A, 4, Adc(0)) => HwConfig {
            mode: "GPIO_ModeIN_Floating",
            digital: false,
            remap: None,
            polarity: None,
        },
        (Port::A, 5, Adc(1)) => HwConfig {
            mode: "GPIO_ModeIN_Floating",
            digital: false,
            remap: None,
            polarity: None,
        },
        (Port::A, 12, Adc(2)) => HwConfig {
            mode: "GPIO_ModeIN_Floating",
            digital: false,
            remap: None,
            polarity: None,
        },
        (Port::A, 13, Adc(3)) => HwConfig {
            mode: "GPIO_ModeIN_Floating",
            digital: false,
            remap: None,
            polarity: None,
        },
        (Port::A, 14, Adc(4)) => HwConfig {
            mode: "GPIO_ModeIN_Floating",
            digital: false,
            remap: None,
            polarity: None,
        },
        (Port::A, 15, Adc(5)) => HwConfig {
            mode: "GPIO_ModeIN_Floating",
            digital: false,
            remap: None,
            polarity: None,
        },
        (Port::A, 3, Adc(6)) => HwConfig {
            mode: "GPIO_ModeIN_Floating",
            digital: false,
            remap: None,
            polarity: None,
        },
        (Port::A, 2, Adc(7)) => HwConfig {
            mode: "GPIO_ModeIN_Floating",
            digital: false,
            remap: None,
            polarity: None,
        },
        (Port::A, 1, Adc(8)) => HwConfig {
            mode: "GPIO_ModeIN_Floating",
            digital: false,
            remap: None,
            polarity: None,
        },
        (Port::A, 0, Adc(9)) => HwConfig {
            mode: "GPIO_ModeIN_Floating",
            digital: false,
            remap: None,
            polarity: None,
        },
        (Port::A, 6, Adc(10)) => HwConfig {
            mode: "GPIO_ModeIN_Floating",
            digital: false,
            remap: None,
            polarity: None,
        },
        (Port::A, 7, Adc(11)) => HwConfig {
            mode: "GPIO_ModeIN_Floating",
            digital: false,
            remap: None,
            polarity: None,
        },
        (Port::A, 8, Adc(12)) => HwConfig {
            mode: "GPIO_ModeIN_Floating",
            digital: false,
            remap: None,
            polarity: None,
        },
        (Port::A, 9, Adc(13)) => HwConfig {
            mode: "GPIO_ModeIN_Floating",
            digital: false,
            remap: None,
            polarity: None,
        },

        (port, pin, func) => {
            panic!(
                "\n❌ 板级配置错误：\n   P{}{} 不支持 {:?}\n\
                 请核对 CH585 数据手册确认该引脚的功能支持。\n",
                port_char(port),
                pin,
                func,
            );
        }
    }
}

fn i2c_remap(port: Port, pin: u8) -> (&'static str, bool) {
    match (port, pin) {
        (Port::B, 12) | (Port::B, 13) => ("RB_PIN_I2C", false),
        (Port::B, 20) | (Port::B, 21) => ("RB_PIN_I2C", true),
        _ => panic!(
            "I2C 无效引脚 P{}{}，仅支持 PB12/PB13（默认）和 PB20/PB21（重映射）",
            port_char(port),
            pin
        ),
    }
}

pub fn generate_c_header(defs: &[Def]) -> String {
    let mut s = String::new();

    s.push_str("// 自动生成，勿手动编辑\n\n");
    s.push_str("#ifndef VOIDPOINTER_BOARD_MAP_H\n");
    s.push_str("#define VOIDPOINTER_BOARD_MAP_H\n\n");
    s.push_str("#include <stdint.h>\n");
    s.push_str("#include <stdbool.h>\n\n");
    s.push_str("#ifdef __cplusplus\n");
    s.push_str("extern \"C\" {\n");
    s.push_str("#endif\n\n");

    s.push_str("typedef enum {\n");
    s.push_str("    BOARD_GPIO_GROUP_NONE = 0,\n");
    s.push_str("    BOARD_GPIO_GROUP_A = 1,\n");
    s.push_str("    BOARD_GPIO_GROUP_B = 2,\n");
    s.push_str("} BoardGpioGroup;\n\n");

    s.push_str("typedef struct {\n");
    s.push_str("    BoardGpioGroup group;\n");
    s.push_str("    uint32_t       pin;\n");
    s.push_str("} BoardGpio;\n\n");

    s.push_str("typedef enum {\n");
    for def in defs {
        s.push_str(&format!("    {},\n", signal_name_to_enum(def.name)));
    }
    s.push_str("    BOARD_SIGNAL_COUNT,\n");
    s.push_str("} BoardSignal;\n\n");

    s.push_str("BoardGpio board_signal_get(BoardSignal sig);\n");
    s.push_str("bool      board_signal_is_present(BoardSignal sig);\n\n");

    s.push_str("extern const BoardGpio BOARD_MAP_DEFAULT[BOARD_SIGNAL_COUNT];\n");
    s.push_str("extern BoardGpio       BOARD_MAP_CURRENT[BOARD_SIGNAL_COUNT];\n\n");

    s.push_str("void board_remap_reset(void);\n");
    s.push_str("void board_remap_apply(const BoardGpio mapping[BOARD_SIGNAL_COUNT]);\n");
    s.push_str("void board_gpio_init_all(void);\n");
    s.push_str("uint8_t board_signal_get_channel(BoardSignal sig);\n");
    s.push_str("uint8_t board_signal_get_polarity(BoardSignal sig);\n\n");

    s.push_str("#ifdef __cplusplus\n");
    s.push_str("}\n");
    s.push_str("#endif\n\n");
    s.push_str("#endif\n");

    s
}

pub fn generate_c_source(defs: &[Def]) -> String {
    let mut s = String::new();

    s.push_str("// 自动生成，勿手动编辑\n\n");
    s.push_str("#include \"board_map.h\"\n");
    s.push_str("#include \"CH58x_common.h\"\n\n");

    s.push_str("const BoardGpio BOARD_MAP_DEFAULT[BOARD_SIGNAL_COUNT] = {\n");
    for def in defs {
        let group = match def.port() {
            Port::A => "BOARD_GPIO_GROUP_A",
            Port::B => "BOARD_GPIO_GROUP_B",
        };
        s.push_str(&format!(
            "    [{}] = {{ .group = {}, .pin = {} }},\n",
            signal_name_to_enum(def.name),
            group,
            pin_mask(def.pin_num())
        ));
    }
    s.push_str("};\n\n");

    s.push_str("BoardGpio BOARD_MAP_CURRENT[BOARD_SIGNAL_COUNT];\n\n");

    s.push_str("BoardGpio board_signal_get(BoardSignal sig) {\n");
    s.push_str("    if (sig >= BOARD_SIGNAL_COUNT) {\n");
    s.push_str("        return (BoardGpio){ .group = BOARD_GPIO_GROUP_NONE, .pin = 0u };\n");
    s.push_str("    }\n");
    s.push_str("    return BOARD_MAP_CURRENT[sig];\n");
    s.push_str("}\n\n");

    s.push_str("bool board_signal_is_present(BoardSignal sig) {\n");
    s.push_str("    if (sig >= BOARD_SIGNAL_COUNT) {\n");
    s.push_str("        return false;\n");
    s.push_str("    }\n");
    s.push_str("    const BoardGpio g = BOARD_MAP_CURRENT[sig];\n");
    s.push_str("    return g.group != BOARD_GPIO_GROUP_NONE && g.pin != 0u;\n");
    s.push_str("}\n\n");

    s.push_str("void board_remap_reset(void) {\n");
    s.push_str("    for (int i = 0; i < BOARD_SIGNAL_COUNT; i++) {\n");
    s.push_str("        BOARD_MAP_CURRENT[i] = BOARD_MAP_DEFAULT[i];\n");
    s.push_str("    }\n");
    s.push_str("}\n\n");

    s.push_str("void board_remap_apply(const BoardGpio mapping[BOARD_SIGNAL_COUNT]) {\n");
    s.push_str("    for (int i = 0; i < BOARD_SIGNAL_COUNT; i++) {\n");
    s.push_str("        BOARD_MAP_CURRENT[i] = mapping[i];\n");
    s.push_str("    }\n");
    s.push_str("}\n\n");

    s.push_str("uint8_t board_signal_get_channel(BoardSignal sig) {\n");
    s.push_str("    switch (sig) {\n");
    for def in defs {
        if let Some(ch) = def.channel() {
            s.push_str(&format!(
                "        case {}: return {};\n",
                signal_name_to_enum(def.name),
                ch
            ));
        }
    }
    s.push_str("        default: return 0;\n");
    s.push_str("    }\n");
    s.push_str("}\n\n");

    // ── 极性查询 ──
    s.push_str(&format!(
        "static const uint8_t BOARD_SIGNAL_POLARITY[BOARD_SIGNAL_COUNT] = {{ \
         {}\
         }};\n\n",
        defs.iter()
            .map(|d| format!(
                "    [{}] = {},",
                signal_name_to_enum(d.name),
                derive_hw_config(d).polarity.map_or(0u8, |v| v as u8)
            ))
            .collect::<Vec<_>>()
            .join("\n")
    ));

    s.push_str("uint8_t board_signal_get_polarity(BoardSignal sig) {\n");
    s.push_str("    if (sig >= BOARD_SIGNAL_COUNT) { return 0u; }\n");
    s.push_str("    return BOARD_SIGNAL_POLARITY[sig];\n");
    s.push_str("}\n\n");

    // ── remap 冲突检测（编译期 panic）──
    {
        use std::collections::HashMap;
        let mut remap_map: HashMap<&str, bool> = HashMap::new();
        for def in defs {
            let hw = derive_hw_config(def);
            if let Some((reg, enabled)) = &hw.remap {
                if let Some(&prev) = remap_map.get(reg) {
                    assert!(
                        prev == *enabled,
                        "REMAP 冲突：信号 '{}' 要求 {}={}，但之前信号已要求 {}={}",
                        def.name,
                        reg,
                        enabled,
                        reg,
                        prev
                    );
                } else {
                    remap_map.insert(reg, *enabled);
                }
            }
        }
    }

    s.push_str("void board_gpio_init_all(void) {\n");
    s.push_str("    board_remap_reset();\n\n");

    let mut remap_done: Vec<&str> = Vec::new();
    for def in defs {
        let hw = derive_hw_config(def);
        if let Some((reg, enabled)) = &hw.remap {
            if !remap_done.contains(reg) {
                s.push_str(&format!(
                    "    GPIOPinRemap({}, {});\n",
                    if *enabled { "ENABLE" } else { "DISABLE" },
                    reg
                ));
                remap_done.push(reg);
            }
        }
    }
    if !remap_done.is_empty() {
        s.push_str("\n");
    }

    for def in defs {
        let hw = derive_hw_config(def);
        let pre = gpio_port_prefix(def.port());
        let pin = pin_mask(def.pin_num());
        if hw.digital {
            s.push_str(&format!("    {}DigitalCfg(ENABLE, {});\n", pre, pin));
        }
        s.push_str(&format!("    {}_ModeCfg({}, {});\n", pre, pin, hw.mode));
        if let Func::Out { init, .. } = &def.func {
            if let Some(level) = init {
                match level {
                    PinLevel::Low => s.push_str(&format!("    {}_ResetBits({});\n", pre, pin)),
                    PinLevel::High => s.push_str(&format!("    {}_SetBits({});\n", pre, pin)),
                }
            }
        }
    }

    s.push_str("}\n");
    s
}

pub fn generate_rust_bindings(defs: &[Def]) -> String {
    let mut s = String::new();

    s.push_str("// 自动生成，勿手动编辑\n\n");

    s.push_str("#[repr(C)]\n");
    s.push_str("#[derive(Debug, Clone, Copy)]\n");
    s.push_str("pub struct BoardGpio {\n");
    s.push_str("    pub group: u32,\n");
    s.push_str("    pub pin: u32,\n");
    s.push_str("}\n\n");

    s.push_str("pub const BOARD_SIGNAL_COUNT: usize = ");
    s.push_str(&defs.len().to_string());
    s.push_str(";\n\n");

    s.push_str("#[allow(non_camel_case_types)]\n");
    s.push_str("#[repr(u8)]\n");
    s.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq)]\n");
    s.push_str("pub enum BoardSignal {\n");
    for def in defs {
        let variant = def.name.to_uppercase();
        s.push_str(&format!("    {},\n", variant));
    }
    s.push_str("}\n\n");

    s.push_str("impl BoardSignal {\n");
    s.push_str("    pub fn channel(self) -> u8 {\n");
    s.push_str("        match self {\n");
    for def in defs {
        if let Some(ch) = def.channel() {
            let variant = def.name.to_uppercase();
            s.push_str(&format!(
                "            BoardSignal::{} => {},\n",
                variant, ch
            ));
        }
    }
    s.push_str("            _ => 0,\n");
    s.push_str("        }\n");
    s.push_str("    }\n");
    s.push_str("}\n\n");

    s.push_str("unsafe extern \"C\" {\n");
    s.push_str("    pub static BOARD_MAP_DEFAULT: [BoardGpio; BOARD_SIGNAL_COUNT];\n");
    s.push_str("    pub static mut BOARD_MAP_CURRENT: [BoardGpio; BOARD_SIGNAL_COUNT];\n");
    s.push_str("    pub fn board_signal_get(sig: BoardSignal) -> BoardGpio;\n");
    s.push_str("    pub fn board_signal_is_present(sig: BoardSignal) -> bool;\n");
    s.push_str("    pub fn board_signal_get_channel(sig: BoardSignal) -> u8;\n");
    s.push_str("    pub fn board_signal_get_polarity(sig: BoardSignal) -> u8;\n");
    s.push_str("    pub fn board_remap_reset();\n");
    s.push_str("    pub fn board_remap_apply(mapping: *const BoardGpio);\n");
    s.push_str("    pub fn board_gpio_init_all();\n");
    s.push_str("}\n");

    s
}
