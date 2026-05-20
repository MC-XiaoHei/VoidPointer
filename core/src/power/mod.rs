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

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

        let target = if wireless_connected {
            if idle_ms >= self.config.suspend_timeout_ms {
                PowerState::Suspend
            } else {
                PowerState::Active
            }
        } else if idle_ms >= self.config.disconnect_sleep_timeout_ms
            && usb_allows_sleep
            && !config_dirty
        {
            PowerState::Sleep
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

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn remaining_not_expired() {
        assert_eq!(remaining_delay_ms(100, 30), 70);
    }

    #[test]
    fn remaining_exactly_expired() {
        assert_eq!(remaining_delay_ms(100, 100), 1);
    }

    #[test]
    fn remaining_over_expired() {
        assert_eq!(remaining_delay_ms(100, 200), 1);
    }

    #[test]
    fn remaining_zero_timeout() {
        assert_eq!(remaining_delay_ms(0, 0), 1);
    }

    #[test]
    fn pm_new_state_is_active() {
        let pm = PowerManager::new();
        assert_eq!(pm.state(), PowerState::Active);
        assert_eq!(pm.config(), PowerConfig::default());
    }

    #[test]
    fn pm_apply_request_accepted() {
        let mut pm = PowerManager::new();
        pm.apply_request_result(PowerState::Suspend, true);
        assert_eq!(pm.state(), PowerState::Suspend);
    }

    #[test]
    fn pm_apply_request_rejected() {
        let mut pm = PowerManager::new();
        pm.apply_request_result(PowerState::Sleep, false);
        assert_eq!(pm.state(), PowerState::Active);
    }

    #[test]
    fn pm_poll_usb_configured_keeps_active() {
        let mut pm = PowerManager::new();
        let mut router = crate::route::HidRouter::new();
        router.set_usb_state(crate::route::UsbState::Configured);
        let result = pm.poll(100, 0, false, &router);
        assert!(result.is_none());
        assert_eq!(pm.state(), PowerState::Active);
    }

    #[test]
    fn pm_poll_sleep_after_disconnect() {
        let mut pm = PowerManager::new();
        let router = crate::route::HidRouter::new();
        let result = pm.poll(120000, 0, false, &router);
        assert_eq!(
            result,
            Some(PowerRequest {
                target: PowerState::Sleep
            })
        );
    }

    #[test]
    fn pm_poll_config_dirty_blocks_sleep() {
        let mut pm = PowerManager::new();
        let router = crate::route::HidRouter::new();
        let result = pm.poll(120000, 0, true, &router);
        assert!(result.is_none());
        assert_eq!(pm.state(), PowerState::Active);
    }

    #[test]
    fn pm_poll_noop_when_already_active() {
        let mut pm = PowerManager::new();
        let router = crate::route::HidRouter::new();
        let result = pm.poll(100, 50, false, &router);
        assert!(result.is_none());
    }

    #[test]
    fn pm_poll_suspend_after_timeout() {
        let mut pm = PowerManager::new();
        let mut router = crate::route::HidRouter::new();
        router.set_ble_connected(true);
        router.set_ble_input_ready(true);

        let result = pm.poll(10000, 0, false, &router);
        assert_eq!(
            result,
            Some(PowerRequest {
                target: PowerState::Suspend
            })
        );
        pm.apply_request_result(PowerState::Suspend, true);

        let result2 = pm.poll(11000, 0, false, &router);
        assert_eq!(result2, None);
    }

    #[test]
    fn pm_next_recheck_delay() {
        let pm = PowerManager::new();
        let mut router = crate::route::HidRouter::new();
        router.set_ble_connected(true);
        router.set_ble_input_ready(true);
        let delay = pm.next_recheck_delay_ms(100, 0, false, &router);
        assert!(delay.is_some());
        assert!(delay.unwrap() <= 5000);
    }

    #[test]
    fn pm_next_recheck_usb_configured_returns_none() {
        let pm = PowerManager::new();
        let mut router = crate::route::HidRouter::new();
        router.set_usb_state(crate::route::UsbState::Configured);
        assert!(pm.next_recheck_delay_ms(100, 0, false, &router).is_none());
    }

    #[test]
    fn pm_next_recheck_already_suspend_returns_none() {
        let mut pm = PowerManager::new();
        pm.apply_request_result(PowerState::Suspend, true);
        let mut router = crate::route::HidRouter::new();
        router.set_ble_connected(true);
        router.set_ble_input_ready(true);
        assert!(pm.next_recheck_delay_ms(100, 0, false, &router).is_none());
    }

    #[test]
    fn pm_next_recheck_already_sleep_returns_none() {
        let mut pm = PowerManager::new();
        pm.apply_request_result(PowerState::Sleep, true);
        let router = crate::route::HidRouter::new();
        assert!(pm.next_recheck_delay_ms(100, 0, false, &router).is_none());
    }

    #[test]
    fn pm_next_recheck_sleep_path_returns_delay() {
        let pm = PowerManager::new();
        let router = crate::route::HidRouter::new();
        let delay = pm.next_recheck_delay_ms(100, 0, false, &router);
        assert!(delay.is_some());
        assert!(delay.unwrap() <= 60000);
    }

    #[test]
    fn pm_next_recheck_config_dirty_blocks_sleep() {
        let pm = PowerManager::new();
        let router = crate::route::HidRouter::new();
        assert!(pm.next_recheck_delay_ms(100, 0, true, &router).is_none());
    }

    #[test]
    fn pm_poll_ble_connected_stays_active_when_not_idle_enough() {
        let mut pm = PowerManager::new();
        let mut router = crate::route::HidRouter::new();
        router.set_ble_connected(true);
        router.set_ble_input_ready(true);
        // idle_ms = 50，远小于 suspend_timeout_ms (5000)
        let result = pm.poll(100, 50, false, &router);
        assert!(result.is_none());
        assert_eq!(pm.state(), PowerState::Active);
    }

    #[test]
    fn pm_request_state_same_state_returns_none() {
        let mut pm = PowerManager::new();
        let result = pm.poll(100, 50, false, &crate::route::HidRouter::new());
        assert!(result.is_none());
        assert_eq!(pm.state(), PowerState::Active);
    }

    #[test]
    fn pm_request_state_returns_request_for_suspend() {
        let mut pm = PowerManager::new();
        let mut router = crate::route::HidRouter::new();
        router.set_ble_connected(true);
        router.set_ble_input_ready(true);
        let result = pm.poll(10000, 0, false, &router);
        assert_eq!(
            result,
            Some(PowerRequest {
                target: PowerState::Suspend
            })
        );
    }

    #[test]
    fn pm_request_state_returns_request_for_sleep() {
        let mut pm = PowerManager::new();
        let result = pm.poll(120000, 0, false, &crate::route::HidRouter::new());
        assert_eq!(
            result,
            Some(PowerRequest {
                target: PowerState::Sleep
            })
        );
    }

    #[test]
    fn pm_apply_request_active_sets_state() {
        let mut pm = PowerManager::new();
        pm.apply_request_result(PowerState::Active, true);
        assert_eq!(pm.state(), PowerState::Active);
    }

    #[test]
    fn pm_request_state_transition_from_suspend_to_active() {
        // 覆盖 request_state 中 PowerState::Active 分支：从非 Active 态切换而来
        let mut pm = PowerManager::new();
        pm.apply_request_result(PowerState::Suspend, true);
        assert_eq!(pm.state(), PowerState::Suspend);

        let mut router = crate::route::HidRouter::new();
        router.set_ble_connected(true);
        router.set_ble_input_ready(true);
        // idle_ms=50 < suspend_timeout_ms=5000 => target = Active
        let result = pm.poll(100, 50, false, &router);

        assert!(result.is_none());
        assert_eq!(pm.state(), PowerState::Active);
    }
}
