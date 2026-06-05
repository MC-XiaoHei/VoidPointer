use crate::ffi::bindings::{
    VP_STATUS_OK, c_vp_debounce_timer_start, c_vp_debounce_timer_stop, c_vp_gpio_read,
};
use crate::input::config::{ButtonFunction, ButtonMapping, ButtonProfile, PHYSICAL_BUTTON_COUNT};

const DEBOUNCE_STABLE_TICKS: u8 = 5;

/// 6 个物理按键：0=Context, 1=Action, 2=Up, 3=Down, 4=Primary, 5=Secondary
const BUTTON_COUNT: usize = PHYSICAL_BUTTON_COUNT;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InputStatus {
    pub left: bool,
    pub right: bool,
    pub middle: bool,
    pub action: bool,
    pub laser: bool,
    pub wheel_delta: i8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct StableTransition {
    input_id: u8,
    active: bool,
    changed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DebouncedTwoStateInput {
    input_id: u8,
    stable_active: bool,
    candidate_active: bool,
    stable_ticks: u8,
    debouncing: bool,
}

impl DebouncedTwoStateInput {
    const fn new(input_id: u8) -> Self {
        Self {
            input_id,
            stable_active: false,
            candidate_active: false,
            stable_ticks: 0,
            debouncing: false,
        }
    }

    fn sync_from_gpio(&mut self, active: bool) {
        self.stable_active = active;
        self.candidate_active = active;
        self.stable_ticks = 0;
        self.debouncing = false;
    }

    fn begin_debounce(&mut self, observed_active: bool) {
        self.candidate_active = observed_active;
        self.stable_ticks = 0;
        self.debouncing = true;
    }

    fn sample(&mut self, observed: bool) -> DebounceTickOutcome {
        if !self.debouncing {
            if observed == self.stable_active {
                return DebounceTickOutcome::Idle;
            }
            self.begin_debounce(observed);
        }

        self.track_candidate(observed);

        if !self.candidate_is_stable() {
            return DebounceTickOutcome::StillDebouncing;
        }

        self.accept_candidate()
    }

    fn track_candidate(&mut self, observed: bool) {
        if observed == self.candidate_active {
            self.stable_ticks = self.stable_ticks.saturating_add(1);
        } else {
            self.candidate_active = observed;
            self.stable_ticks = 0;
        }
    }

    fn candidate_is_stable(&self) -> bool {
        self.stable_ticks >= DEBOUNCE_STABLE_TICKS
    }

    fn accept_candidate(&mut self) -> DebounceTickOutcome {
        self.debouncing = false;
        if self.stable_active != self.candidate_active {
            self.stable_active = self.candidate_active;
            DebounceTickOutcome::Stabilized(StableTransition {
                input_id: self.input_id,
                active: self.stable_active,
                changed: true,
            })
        } else {
            DebounceTickOutcome::Stabilized(StableTransition {
                input_id: self.input_id,
                active: self.stable_active,
                changed: false,
            })
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DebounceTickOutcome {
    Idle,
    StillDebouncing,
    Stabilized(StableTransition),
}

pub struct InputManager {
    two_state_inputs: [DebouncedTwoStateInput; BUTTON_COUNT],
    active_mapping: [ButtonMapping; PHYSICAL_BUTTON_COUNT],
    pending_wheel: i8,
}

impl InputManager {
    pub fn new() -> Self {
        Self {
            two_state_inputs: {
                const INIT: DebouncedTwoStateInput = DebouncedTwoStateInput::new(0);
                let mut arr = [INIT; BUTTON_COUNT];
                let mut i = 0;
                while i < BUTTON_COUNT {
                    arr[i].input_id = i as u8;
                    i += 1;
                }
                arr
            },
            active_mapping: [ButtonMapping::new([ButtonFunction::None; 4]); PHYSICAL_BUTTON_COUNT],
            pending_wheel: 0,
        }
    }

    pub fn set_profile(&mut self, profile: &ButtonProfile) {
        self.active_mapping[0] = profile.context;
        self.active_mapping[1] = profile.action;
        self.active_mapping[2] = profile.up;
        self.active_mapping[3] = profile.down;
        self.active_mapping[4] = profile.primary;
        self.active_mapping[5] = profile.secondary;
    }

    #[cfg_attr(coverage, coverage(off))]
    pub fn sync_snapshot(&mut self) -> InputStatus {
        // 初始同步：读取所有 GPIO 并构建初始状态
        for input in &mut self.two_state_inputs {
            let level = read_active_low_input(input.input_id);
            input.sync_from_gpio(level);
        }
        self.pending_wheel = 0;
        InputStatus::default()
    }

    #[cfg_attr(coverage, coverage(off))]
    pub fn enable_interrupts(&self) {}

    #[cfg_attr(coverage, coverage(off))]
    pub fn on_button_exti(&mut self, button_id: u8, active: bool) -> bool {
        if button_id as usize >= BUTTON_COUNT {
            return false;
        }
        let input = &mut self.two_state_inputs[button_id as usize];
        input.begin_debounce(active);
        start_debounce_timer()
    }

    #[cfg_attr(coverage, coverage(off))]
    pub fn on_debounce_tick(&mut self) -> bool {
        let mut changed = false;
        let mut any_debouncing = false;
        for input in &mut self.two_state_inputs {
            match input.sample(read_active_low_input(input.input_id)) {
                DebounceTickOutcome::Idle => {}
                DebounceTickOutcome::StillDebouncing => {
                    any_debouncing = true;
                }
                DebounceTickOutcome::Stabilized(t) => {
                    changed |= t.changed;
                }
            }
        }
        if !any_debouncing {
            stop_debounce_timer();
        }
        changed
    }

    /// 将 debounce 后的物理状态按当前映射转换为逻辑状态
    #[cfg_attr(coverage, coverage(off))]
    pub fn get_current_input(&mut self) -> InputStatus {
        let mut status = InputStatus::default();

        for (phys_idx, input) in self.two_state_inputs.iter().enumerate() {
            if !input.stable_active {
                continue;
            }

            let mapping = &self.active_mapping[phys_idx];
            for &fn_id in &mapping.fns {
                match fn_id {
                    ButtonFunction::None => {}
                    ButtonFunction::Left => status.left = true,
                    ButtonFunction::Right => status.right = true,
                    ButtonFunction::Middle => status.middle = true,
                    ButtonFunction::Action => status.action = true,
                    ButtonFunction::Laser => status.laser = true,
                    ButtonFunction::ScrollUp => {
                        self.pending_wheel = self.pending_wheel.saturating_add(1);
                    }
                    ButtonFunction::ScrollDown => {
                        self.pending_wheel = self.pending_wheel.saturating_sub(1);
                    }
                }
            }
        }

        status.wheel_delta = self.pending_wheel;
        self.pending_wheel = 0;
        status
    }
}

#[cfg_attr(coverage, coverage(off))]
fn read_active_low_input(input_id: u8) -> bool {
    unsafe { c_vp_gpio_read(input_id) != 0 }
}

#[cfg_attr(coverage, coverage(off))]
fn start_debounce_timer() -> bool {
    unsafe { c_vp_debounce_timer_start() == VP_STATUS_OK as u8 }
}

#[cfg_attr(coverage, coverage(off))]
fn stop_debounce_timer() {
    let _ = unsafe { c_vp_debounce_timer_stop() };
}

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use super::*;
    use crate::input::config::ButtonProfile;

    #[test]
    fn default_profile_maps_primary_to_left() {
        let mut m = InputManager::new();
        m.set_profile(&ButtonProfile::default());

        // 模拟 Primary (idx=4) 按下
        m.two_state_inputs[4].stable_active = true;

        let status = m.get_current_input();
        assert!(status.left);
        assert!(!status.right);
        assert!(!status.middle);
        assert!(!status.action);
        assert!(!status.laser);
        assert_eq!(status.wheel_delta, 0);
    }

    #[test]
    fn scroll_up_down_produces_wheel() {
        let mut m = InputManager::new();
        // 自定义映射：Up=ScrollUp, Down=ScrollDown
        let mut profile = ButtonProfile::default();
        profile.up = ButtonMapping::new([
            ButtonFunction::ScrollUp,
            ButtonFunction::None,
            ButtonFunction::None,
            ButtonFunction::None,
        ]);
        profile.down = ButtonMapping::new([
            ButtonFunction::ScrollDown,
            ButtonFunction::None,
            ButtonFunction::None,
            ButtonFunction::None,
        ]);
        m.set_profile(&profile);

        m.two_state_inputs[2].stable_active = true; // Up
        m.two_state_inputs[3].stable_active = true; // Down

        let status = m.get_current_input();
        assert_eq!(status.wheel_delta, 0); // +1 -1 = 0
    }

    #[test]
    fn multi_mapping_triggers_all() {
        let mut m = InputManager::new();
        // Context = [Middle, Action]
        let mut profile = ButtonProfile::default();
        profile.context = ButtonMapping::new([
            ButtonFunction::Middle,
            ButtonFunction::Action,
            ButtonFunction::None,
            ButtonFunction::None,
        ]);
        m.set_profile(&profile);

        m.two_state_inputs[0].stable_active = true; // Context

        let status = m.get_current_input();
        assert!(status.middle);
        assert!(status.action);
    }

    #[test]
    fn no_mapping_produces_empty_status() {
        let mut m = InputManager::new();
        // active_mapping 初始全 None
        let status = m.get_current_input();
        assert!(!status.left);
        assert!(!status.right);
        assert!(!status.middle);
        assert!(!status.action);
        assert!(!status.laser);
        assert_eq!(status.wheel_delta, 0);
    }
}
