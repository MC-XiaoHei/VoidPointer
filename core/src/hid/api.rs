use crate::ffi::bindings::*;
use crate::hid::types::{CustomReport, HidSendStatus, MouseReport};

/// 封装 HID 发送的 FFI 调用，调用方需保证在 bottom-half 上下文中使用
pub struct HidApi;

impl HidApi {
    pub fn send_mouse(route: u8, report: MouseReport) -> HidSendStatus {
        let status = unsafe {
            c_vp_hid_send_mouse(
                route,
                report.buttons.pack(),
                report.dx,
                report.dy,
                report.wheel,
            )
        };
        Self::map_status(status)
    }

    pub fn send_vendor(route: u8, report: &CustomReport) -> HidSendStatus {
        let status = unsafe { c_vp_hid_send_vendor(route, report.data.as_ptr(), report.len) };
        Self::map_status(status)
    }

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
