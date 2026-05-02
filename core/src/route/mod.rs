use crate::ffi::bindings::{vp_hid_route_t, vp_usb_state_t};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HidRoute {
    None,
    Ble,
    Dongle2G4,
    Usb,
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
    ble_connected: bool,
    dongle_connected: bool,
    usb_state: UsbState,
}

impl HidRouter {
    pub fn new() -> Self {
        Self {
            ble_connected: false,
            dongle_connected: false,
            usb_state: UsbState::Detached,
        }
    }

    pub fn set_ble_connected(&mut self, connected: bool) {
        self.ble_connected = connected;
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

    pub fn usb_state(&self) -> UsbState {
        self.usb_state
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
