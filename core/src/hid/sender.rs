use crate::bindings::*;

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
    fn map_status(status: hid_send_status_t) -> HidSendStatus {
        match status {
            hid_send_status_t_HID_SEND_OK => HidSendStatus::Sent,
            hid_send_status_t_HID_SEND_RETRY => HidSendStatus::RetryLater,
            hid_send_status_t_HID_SEND_FATAL => HidSendStatus::Fatal,
            _ => HidSendStatus::Fatal,
        }
    }
}

impl HidSender for BleHidSender {
    fn send_mouse_report(&mut self, report: MouseReport) -> HidSendStatus {
        let status = unsafe {
            c_send_ble_hid_mouse_report(report.buttons, report.dx, report.dy, report.wheel)
        };

        Self::map_status(status)
    }
}
