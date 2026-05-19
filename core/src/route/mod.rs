use crate::ffi::bindings::{
    VP_HID_ROUTE_BLE, VP_HID_ROUTE_DONGLE_2G4, VP_HID_ROUTE_NONE, VP_HID_ROUTE_USB,
    VP_USB_STATE_ATTACHED, VP_USB_STATE_CONFIGURED, VP_USB_STATE_ERROR, VP_USB_STATE_SUSPENDED,
    vp_hid_route_t, vp_usb_state_t,
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
            x if x == VP_HID_ROUTE_BLE as u8 => Self::Ble,
            x if x == VP_HID_ROUTE_DONGLE_2G4 as u8 => Self::Dongle2G4,
            x if x == VP_HID_ROUTE_USB as u8 => Self::Usb,
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
            x if x == VP_USB_STATE_ATTACHED as u8 => Self::Attached,
            x if x == VP_USB_STATE_CONFIGURED as u8 => Self::Configured,
            x if x == VP_USB_STATE_SUSPENDED as u8 => Self::Suspended,
            x if x == VP_USB_STATE_ERROR as u8 => Self::Error,
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

    fn preferred_wireless_route(&self) -> HidRoute {
        // 2.4G 尚未实现，无线固定选择 BLE
        if self.is_ble_input_ready() {
            HidRoute::Ble
        } else {
            HidRoute::None
        }
    }

    pub fn preferred_mouse_route(&self) -> HidRoute {
        if self.is_usb_configured() {
            HidRoute::Usb
        } else {
            self.preferred_wireless_route()
        }
    }

    pub fn preferred_custom_route(&self) -> HidRoute {
        if self.is_usb_configured() {
            HidRoute::Usb
        } else {
            self.preferred_wireless_route()
        }
    }

    pub fn has_mouse_route(&self) -> bool {
        self.preferred_mouse_route() != HidRoute::None
    }

    pub fn has_custom_route(&self) -> bool {
        self.preferred_custom_route() != HidRoute::None
    }

    pub fn has_wireless_connection(&self) -> bool {
        self.is_ble_input_ready()
    }
}

impl Default for HidRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn hid_route_as_ffi() {
        assert_eq!(HidRoute::None.as_ffi(), 0);
        assert_eq!(HidRoute::Ble.as_ffi(), 1);
        assert_eq!(HidRoute::Dongle2G4.as_ffi(), 2);
        assert_eq!(HidRoute::Usb.as_ffi(), 3);
    }

    #[test]
    fn hid_route_from_ffi() {
        assert_eq!(HidRoute::from(0), HidRoute::None);
        assert_eq!(HidRoute::from(1), HidRoute::Ble);
        assert_eq!(HidRoute::from(2), HidRoute::Dongle2G4);
        assert_eq!(HidRoute::from(3), HidRoute::Usb);
        assert_eq!(HidRoute::from(99), HidRoute::None);
    }

    #[test]
    fn usb_state_from_ffi() {
        assert_eq!(UsbState::from(0), UsbState::Detached);
        assert_eq!(UsbState::from(1), UsbState::Attached);
        assert_eq!(UsbState::from(2), UsbState::Configured);
        assert_eq!(UsbState::from(3), UsbState::Suspended);
        assert_eq!(UsbState::from(4), UsbState::Error);
        assert_eq!(UsbState::from(99), UsbState::Detached);
    }

    #[test]
    fn router_default_state() {
        let r = HidRouter::new();
        assert!(!r.is_ble_connected());
        assert!(!r.is_ble_input_ready());
        assert_eq!(r.usb_state(), UsbState::Detached);
        assert!(!r.is_usb_configured());
        assert_eq!(r.preferred_mouse_route(), HidRoute::None);
    }

    #[test]
    fn usb_configured_preferred_over_ble() {
        let mut r = HidRouter::new();
        r.set_ble_connected(true);
        r.set_ble_input_ready(true);
        r.set_usb_state(UsbState::Configured);
        assert_eq!(r.preferred_mouse_route(), HidRoute::Usb);
        assert_eq!(r.preferred_custom_route(), HidRoute::Usb);
        assert!(r.is_usb_configured());
    }

    #[test]
    fn ble_ready_when_no_usb() {
        let mut r = HidRouter::new();
        r.set_ble_connected(true);
        r.set_ble_input_ready(true);
        assert_eq!(r.preferred_mouse_route(), HidRoute::Ble);
        assert!(r.has_mouse_route());
        assert!(r.has_wireless_connection());
    }

    #[test]
    fn ble_connected_not_ready() {
        let mut r = HidRouter::new();
        r.set_ble_connected(true);
        assert_eq!(r.preferred_mouse_route(), HidRoute::None);
        assert!(!r.has_mouse_route());
    }

    #[test]
    fn ble_disconnect_resets_input_ready() {
        let mut r = HidRouter::new();
        r.set_ble_connected(true);
        r.set_ble_input_ready(true);
        assert!(r.is_ble_input_ready());
        r.set_ble_connected(false);
        assert!(!r.is_ble_connected());
        assert!(!r.is_ble_input_ready());
    }

    #[test]
    fn set_ble_input_ready_ignored_when_disconnected() {
        let mut r = HidRouter::new();
        r.set_ble_input_ready(true);
        assert!(!r.is_ble_input_ready());
    }

    #[test]
    fn dongle_connected() {
        let mut r = HidRouter::new();
        assert!(!r.is_ble_connected());
        r.set_dongle_connected(true);
        assert_eq!(r.preferred_mouse_route(), HidRoute::None);
    }

    #[test]
    fn has_custom_route_when_usb() {
        let mut r = HidRouter::new();
        assert!(!r.has_custom_route());
        r.set_usb_state(UsbState::Configured);
        assert!(r.has_custom_route());
    }

    #[test]
    fn has_wireless_connection_when_ble_ready() {
        let mut r = HidRouter::new();
        assert!(!r.has_wireless_connection());
        r.set_ble_connected(true);
        r.set_ble_input_ready(true);
        assert!(r.has_wireless_connection());
    }

    #[test]
    fn default_equals_new() {
        assert_eq!(
            HidRouter::default().usb_state(),
            HidRouter::new().usb_state()
        );
    }
}
