#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MouseReport {
    pub buttons: MouseButtons,
    pub dx: i8,
    pub dy: i8,
    pub wheel: i8,
}

pub const CUSTOM_REPORT_PAYLOAD_CAPACITY: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CustomReport {
    pub len: u16,
    pub data: [u8; CUSTOM_REPORT_PAYLOAD_CAPACITY],
}

impl Default for CustomReport {
    fn default() -> Self {
        Self {
            len: 0,
            data: [0u8; CUSTOM_REPORT_PAYLOAD_CAPACITY],
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MouseButtons {
    pub left: bool,
    pub right: bool,
    pub middle: bool,
}

impl MouseButtons {
    pub fn pack(&self) -> u8 {
        let mut b = 0u8;
        if self.left {
            b |= 1 << 0;
        }
        if self.right {
            b |= 1 << 1;
        }
        if self.middle {
            b |= 1 << 2;
        }
        b
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HidSendStatus {
    Sent,
    RetryLater,
    NotConnected,
    Fatal,
}

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn custom_report_default_all_zero() {
        let r = CustomReport::default();
        assert_eq!(r.len, 0);
        assert!(r.data.iter().all(|&b| b == 0));
    }

    #[test]
    fn mouse_buttons_pack_none() {
        let b = MouseButtons {
            left: false,
            right: false,
            middle: false,
        };
        assert_eq!(b.pack(), 0);
    }

    #[test]
    fn mouse_buttons_pack_left() {
        let b = MouseButtons {
            left: true,
            right: false,
            middle: false,
        };
        assert_eq!(b.pack(), 1 << 0);
    }

    #[test]
    fn mouse_buttons_pack_right() {
        let b = MouseButtons {
            left: false,
            right: true,
            middle: false,
        };
        assert_eq!(b.pack(), 1 << 1);
    }

    #[test]
    fn mouse_buttons_pack_middle() {
        let b = MouseButtons {
            left: false,
            right: false,
            middle: true,
        };
        assert_eq!(b.pack(), 1 << 2);
    }

    #[test]
    fn mouse_buttons_pack_all() {
        let b = MouseButtons {
            left: true,
            right: true,
            middle: true,
        };
        assert_eq!(b.pack(), (1 << 0) | (1 << 1) | (1 << 2));
    }
}
