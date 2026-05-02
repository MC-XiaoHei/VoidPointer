use crate::ffi::bindings::*;

use crate::hid::types::{HidSendStatus, MouseReport};

pub trait HidSender {
    fn send_mouse_report(&mut self, report: MouseReport) -> HidSendStatus;
}

#[derive(Default)]
pub struct BleHidSender;

impl BleHidSender {
    pub fn new() -> Self {
        Self
    }

    #[allow(non_upper_case_globals)]
    fn map_status(status: vp_hid_send_status_t) -> HidSendStatus {
        match status {
            x if x == VP_HID_SEND_SENT as u8 => HidSendStatus::Sent,
            x if x == VP_HID_SEND_RETRY_LATER as u8 => HidSendStatus::RetryLater,
            x if x == VP_HID_SEND_NOT_CONNECTED as u8 => HidSendStatus::NotConnected,
            x if x == VP_HID_SEND_FATAL as u8 => HidSendStatus::Fatal,
            _ => HidSendStatus::Fatal,
        }
    }
}

impl HidSender for BleHidSender {
    fn send_mouse_report(&mut self, report: MouseReport) -> HidSendStatus {
        let status = unsafe {
            c_vp_hid_send_mouse(
                VP_HID_ROUTE_BLE as u8,
                report.buttons.pack(),
                report.dx,
                report.dy,
                report.wheel,
            )
        };

        Self::map_status(status)
    }
}
