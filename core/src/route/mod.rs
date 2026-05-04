/**
 * HID 路由策略只回答两个问题：现在该走哪条路，由这条路发出的报告是否真的能成功送达
 *
 * 对 BLE 来说，链路连上不等于输入路径已经可用，因此必须把 connected 和 input-ready 分开建模
 */
use crate::ffi::bindings::{
    VP_HID_ROUTE_BLE, VP_HID_ROUTE_DONGLE_2G4, VP_HID_ROUTE_NONE, VP_HID_ROUTE_USB, vp_hid_route_t,
    vp_usb_state_t,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HidRoute {
    None,
    Ble,
    Dongle2G4,
    Usb,
}

impl HidRoute {
    pub fn as_ffi(self) -> vp_hid_route_t {
        match self {
            Self::None => VP_HID_ROUTE_NONE as u8,
            Self::Ble => VP_HID_ROUTE_BLE as u8,
            Self::Dongle2G4 => VP_HID_ROUTE_DONGLE_2G4 as u8,
            Self::Usb => VP_HID_ROUTE_USB as u8,
        }
    }
}

impl From<vp_hid_route_t> for HidRoute {
    fn from(value: vp_hid_route_t) -> Self {
        match value {
            1 => Self::Ble,
            2 => Self::Dongle2G4,
            3 => Self::Usb,
            _ => Self::None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UsbState {
    Detached,
    Attached,
    Configured,
    Suspended,
    Error,
}

impl From<vp_usb_state_t> for UsbState {
    fn from(value: vp_usb_state_t) -> Self {
        match value {
            1 => Self::Attached,
            2 => Self::Configured,
            3 => Self::Suspended,
            4 => Self::Error,
            _ => Self::Detached,
        }
    }
}

pub struct HidRouter {
    /// 只表示 BLE 链路存在，还不表示输入通知已经可用
    ble_connected: bool,
    /// 只有 secure 与 notify 路径就绪后才允许参与路由选择
    ble_input_ready: bool,
    dongle_connected: bool,
    usb_state: UsbState,
}

impl HidRouter {
    pub fn new() -> Self {
        Self {
            ble_connected: false,
            ble_input_ready: false,
            dongle_connected: false,
            usb_state: UsbState::Detached,
        }
    }

    pub fn set_ble_connected(&mut self, connected: bool) {
        self.ble_connected = connected;
        if !connected {
            self.ble_input_ready = false;
        }
    }

    pub fn set_ble_input_ready(&mut self, ready: bool) {
        self.ble_input_ready = if self.ble_connected { ready } else { false };
    }

    pub fn set_dongle_connected(&mut self, connected: bool) {
        self.dongle_connected = connected;
    }

    pub fn set_usb_state(&mut self, state: UsbState) {
        self.usb_state = state;
    }

    pub fn is_ble_connected(&self) -> bool {
        self.ble_connected
    }

    pub fn is_ble_input_ready(&self) -> bool {
        self.ble_connected && self.ble_input_ready
    }

    pub fn usb_state(&self) -> UsbState {
        self.usb_state
    }

    pub fn is_usb_configured(&self) -> bool {
        self.usb_state == UsbState::Configured
    }

    pub fn preferred_mouse_route(&self) -> HidRoute {
        if self.is_usb_configured() {
            HidRoute::Usb
        } else if self.is_ble_input_ready() {
            HidRoute::Ble
        } else if self.dongle_connected {
            HidRoute::Dongle2G4
        } else {
            HidRoute::None
        }
    }

    pub fn preferred_custom_route(&self) -> HidRoute {
        if self.is_usb_configured() {
            HidRoute::Usb
        } else if self.is_ble_input_ready() {
            HidRoute::Ble
        } else if self.dongle_connected {
            HidRoute::Dongle2G4
        } else {
            HidRoute::None
        }
    }

    pub fn has_mouse_route(&self) -> bool {
        self.preferred_mouse_route() != HidRoute::None
    }

    pub fn has_custom_route(&self) -> bool {
        self.preferred_custom_route() != HidRoute::None
    }

    pub fn has_wireless_connection(&self) -> bool {
        self.ble_connected || self.dongle_connected
    }
}

impl Default for HidRouter {
    fn default() -> Self {
        Self::new()
    }
}
