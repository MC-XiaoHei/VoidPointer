use crate::ffi::board_map::BoardSignal;
use crate::led::TICK_MS;
use crate::led::patterns::CHARGING;
use crate::led::patterns::LOW_BATTERY;
use crate::led::stop_playback;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersistentState {
    Charging,
    LowBattery,
}

pub struct LedManager {
    persistent: Option<PersistentState>,
    transient_end_ms: Option<u32>,
    last_frame_ms: Option<u32>,
    tick_scheduled: bool,
}

impl LedManager {
    pub fn new() -> Self {
        Self {
            persistent: None,
            transient_end_ms: None,
            last_frame_ms: None,
            tick_scheduled: false,
        }
    }

    pub fn clear_tick_scheduled(&mut self) {
        self.tick_scheduled = false;
    }

    pub fn set_persistent(&mut self, state: Option<PersistentState>) {
        if self.persistent == state {
            return;
        }
        self.persistent = state;
        if self.transient_end_ms.is_none() {
            self.apply_persistent();
        }
    }

    pub fn begin_transient(&mut self, duration_ms: u32, now: u32) {
        self.transient_end_ms = Some(now.wrapping_add(duration_ms));
        self.last_frame_ms = Some(now);
        self.tick_scheduled = false;
    }

    /// 返回 true 表示瞬态未结束，需要继续调度
    pub fn poll(&mut self, now: u32) -> bool {
        let Some(end) = self.transient_end_ms else {
            return false;
        };

        if now.wrapping_sub(end) >= 0x8000_0000 {
            if self.last_frame_ms.is_none() {
                self.last_frame_ms = Some(now);
                self.tick_scheduled = false;
            }
            let need_schedule = !self.tick_scheduled
                && self
                    .last_frame_ms
                    .map_or(true, |last| now.wrapping_sub(last) >= TICK_MS as u32);
            if need_schedule {
                self.last_frame_ms = Some(now);
                self.tick_scheduled = true;
                return true;
            }
            return false;
        }

        self.transient_end_ms = None;
        self.last_frame_ms = None;
        self.tick_scheduled = false;

        // 有持续态则立即切换；无持续态时不调 stop_playback()，
        // 让当前 DMA 播完末尾 0 自动灭
        if self.persistent.is_some() {
            self.apply_persistent();
        }

        false
    }

    fn apply_persistent(&self) {
        match self.persistent {
            Some(PersistentState::Charging) => CHARGING.play(BoardSignal::STATUS_LED),
            Some(PersistentState::LowBattery) => LOW_BATTERY.play(BoardSignal::STATUS_LED),
            None => stop_playback(),
        }
    }
}

impl Default for LedManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_is_idle() {
        let m = LedManager::new();
        assert_eq!(m.persistent, None);
    }

    #[test]
    fn set_persistent_chargin_applies_immediately() {
        let mut m = LedManager::new();
        m.set_persistent(Some(PersistentState::Charging));
        assert_eq!(m.persistent, Some(PersistentState::Charging));
    }

    #[test]
    fn clear_persistent_stops() {
        let mut m = LedManager::new();
        m.set_persistent(Some(PersistentState::Charging));
        m.set_persistent(None);
        assert_eq!(m.persistent, None);
    }

    #[test]
    fn transient_suppresses_persistent_until_done() {
        let mut m = LedManager::new();
        m.set_persistent(Some(PersistentState::Charging));
        m.begin_transient(100, 0);
        m.set_persistent(Some(PersistentState::LowBattery));
        m.poll(50);
        assert_eq!(m.persistent, Some(PersistentState::LowBattery));
        m.poll(150);
        assert!(m.transient_end_ms.is_none());
    }

    #[test]
    fn transient_ends_triggers_persistent_apply() {
        let mut m = LedManager::new();
        m.set_persistent(Some(PersistentState::LowBattery));
        m.begin_transient(200, 100);
        m.poll(100);
        m.poll(350);
        assert!(m.transient_end_ms.is_none());
    }
}
