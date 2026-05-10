use crate::route::{HidRouter, UsbState};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PowerRequest {
    pub target: PowerState,
}

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

    pub fn config(&self) -> PowerConfig {
        self.config
    }

    pub fn poll(
        &mut self,
        now_ms: u32,
        last_activity_ms: u32,
        config_dirty: bool,
        router: &HidRouter,
    ) -> Option<PowerRequest> {
        if router.usb_state() == UsbState::Configured {
            self.state = PowerState::Active;
            return None;
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

        self.request_state(target)
    }

    pub fn next_recheck_delay_ms(
        &self,
        now_ms: u32,
        last_activity_ms: u32,
        config_dirty: bool,
        router: &HidRouter,
    ) -> Option<u32> {
        if router.usb_state() == UsbState::Configured {
            return None;
        }

        let idle_ms = now_ms.wrapping_sub(last_activity_ms);
        let wireless_connected = router.has_wireless_connection();
        let usb_allows_sleep = router.usb_state() == UsbState::Detached;

        if wireless_connected {
            if self.state == PowerState::Suspend {
                return None;
            }
            return Some(remaining_delay_ms(self.config.suspend_timeout_ms, idle_ms));
        }

        if usb_allows_sleep && !config_dirty {
            if self.state == PowerState::Sleep {
                return None;
            }
            return Some(remaining_delay_ms(
                self.config.disconnect_sleep_timeout_ms,
                idle_ms,
            ));
        }

        None
    }

    fn request_state(&mut self, target: PowerState) -> Option<PowerRequest> {
        if self.state == target {
            return None;
        }

        match target {
            PowerState::Active => {
                self.state = PowerState::Active;
                None
            }
            PowerState::Suspend | PowerState::Sleep => Some(PowerRequest { target }),
        }
    }

    pub fn apply_request_result(&mut self, target: PowerState, accepted: bool) {
        if accepted {
            self.state = target;
        } else {
            self.state = PowerState::Active;
        }
    }
}

fn remaining_delay_ms(timeout_ms: u32, elapsed_ms: u32) -> u32 {
    timeout_ms.saturating_sub(elapsed_ms).max(1)
}

impl Default for PowerManager {
    fn default() -> Self {
        Self::new()
    }
}
