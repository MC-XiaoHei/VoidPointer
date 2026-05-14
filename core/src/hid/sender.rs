use crate::ffi::bindings::*;
use crate::runtime::commands::map_hid_status;

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

        map_hid_status(status)
    }
}
