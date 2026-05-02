use crate::ffi::bindings::{
    c_vp_power_enter_sleep, c_vp_power_enter_suspend, c_vp_power_prepare_sleep,
    c_vp_power_prepare_suspend,
};
use crate::route::{HidRouter, UsbState};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PowerState {
    Active,
    Suspend,
    Sleep,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PowerConfig {
    pub suspend_timeout_ms: u32,
    pub disconnect_sleep_timeout_ms: u32,
}

impl Default for PowerConfig {
    fn default() -> Self {
        Self {
            suspend_timeout_ms: 5_000,
            disconnect_sleep_timeout_ms: 60_000,
        }
    }
}

pub struct PowerManager {
    state: PowerState,
    config: PowerConfig,
}

impl PowerManager {
    pub fn new() -> Self {
        Self {
            state: PowerState::Active,
            config: PowerConfig::default(),
        }
    }

    pub fn state(&self) -> PowerState {
        self.state
    }

    pub fn poll(
        &mut self,
        now_ms: u32,
        last_activity_ms: u32,
        config_dirty: bool,
        router: &HidRouter,
    ) {
        if router.usb_state() == UsbState::Configured {
            self.state = PowerState::Active;
            return;
        }

        let idle_ms = now_ms.wrapping_sub(last_activity_ms);
        let wireless_connected = router.has_wireless_connection();
        let usb_allows_sleep = router.usb_state() == UsbState::Detached;

        let target = if !wireless_connected
            && usb_allows_sleep
            && !config_dirty
            && idle_ms >= self.config.disconnect_sleep_timeout_ms
        {
            PowerState::Sleep
        } else if wireless_connected && idle_ms >= self.config.suspend_timeout_ms {
            PowerState::Suspend
        } else {
            PowerState::Active
        };

        self.transition(target);
    }

    fn transition(&mut self, target: PowerState) {
        if self.state == target {
            return;
        }

        match target {
            PowerState::Active => {
                self.state = PowerState::Active;
            }
            PowerState::Suspend => unsafe {
                let _ = c_vp_power_prepare_suspend();
                let _ = c_vp_power_enter_suspend();
                self.state = PowerState::Suspend;
            },
            PowerState::Sleep => unsafe {
                let _ = c_vp_power_prepare_sleep();
                let _ = c_vp_power_enter_sleep();
                self.state = PowerState::Sleep;
            },
        }
    }
}

impl Default for PowerManager {
    fn default() -> Self {
        Self::new()
    }
}
