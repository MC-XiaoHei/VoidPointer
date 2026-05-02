#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MouseReport {
    pub buttons: MouseButtons,
    pub dx: i8,
    pub dy: i8,
    pub wheel: i8,
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
