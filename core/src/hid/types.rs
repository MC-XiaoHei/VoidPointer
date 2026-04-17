#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MouseReport {
    pub buttons: u8,
    pub dx: i8,
    pub dy: i8,
    pub wheel: i8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HidSendStatus {
    Sent,
    RetryLater,
    Fatal,
}