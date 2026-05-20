use crate::ffi::bindings::{
    VP_EXTI_EDGE_BOTH, VP_EXTI_EDGE_FALLING, VP_EXTI_EDGE_RISING, VP_INPUT_ACTION,
    VP_INPUT_ENCODER_A, VP_INPUT_ENCODER_B, VP_INPUT_LASER, VP_INPUT_LEFT, VP_INPUT_MIDDLE,
    VP_INPUT_RIGHT, VP_STATUS_OK, c_vp_debounce_timer_start, c_vp_debounce_timer_stop,
    c_vp_exti_set_edge, c_vp_gpio_read,
};
use crate::input::encoder::RotaryEncoder;

const DEBOUNCE_STABLE_TICKS: u8 = 5;

const BUTTON_IDS: [u8; 5] = [
    VP_INPUT_LEFT as u8,
    VP_INPUT_RIGHT as u8,
    VP_INPUT_MIDDLE as u8,
    VP_INPUT_ACTION as u8,
    VP_INPUT_LASER as u8,
];
const BUTTON_COUNT: usize = BUTTON_IDS.len();

fn button_id_to_input_id(button_id: u8) -> Option<u8> {
    BUTTON_IDS.get(button_id as usize).copied()
}

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
            return DebounceTickOutcome::Idle;
        }

        self.track_candidate(observed);

        if !self.candidate_is_stable() {
            return DebounceTickOutcome::StillDebouncing;
        }

        DebounceTickOutcome::Stabilized(self.accept_candidate())
    }

    fn track_candidate(&mut self, observed_active: bool) {
        if observed_active == self.candidate_active {
            self.stable_ticks = self.stable_ticks.saturating_add(1);
        } else {
            self.candidate_active = observed_active;
            self.stable_ticks = 1;
        }
    }

    fn candidate_is_stable(&self) -> bool {
        self.stable_ticks >= DEBOUNCE_STABLE_TICKS
    }

    fn accept_candidate(&mut self) -> StableTransition {
        let transition = StableTransition {
            input_id: self.input_id,
            active: self.candidate_active,
            changed: self.stable_active != self.candidate_active,
        };

        self.stable_active = self.candidate_active;
        self.stable_ticks = 0;
        self.debouncing = false;

        transition
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DebounceTickOutcome {
    Idle,
    StillDebouncing,
    Stabilized(StableTransition),
}

pub struct InputManager {
    encoder: RotaryEncoder,
    two_state_inputs: [DebouncedTwoStateInput; BUTTON_COUNT],
    pending_wheel: i8,
}

impl InputManager {
    pub fn new() -> Self {
        Self {
            encoder: RotaryEncoder::new(),
            two_state_inputs: {
                const INIT: DebouncedTwoStateInput = DebouncedTwoStateInput::new(0);
                let mut arr = [INIT; BUTTON_COUNT];
                let mut i = 0;
                while i < BUTTON_COUNT {
                    arr[i].input_id = BUTTON_IDS[i];
                    i += 1;
                }
                arr
            },
            pending_wheel: 0,
        }
    }

    #[cfg_attr(coverage, coverage(off))]
    pub fn sync_snapshot(&mut self) -> InputStatus {
        let levels: [_; BUTTON_COUNT] = BUTTON_IDS.map(|id| read_active_low_input(id));
        let enc_a = read_active_low_input(VP_INPUT_ENCODER_A as u8);
        let enc_b = read_active_low_input(VP_INPUT_ENCODER_B as u8);
        self.sync_with_gpio_values(&levels, enc_a, enc_b)
    }

    fn sync_with_gpio_values(&mut self, levels: &[bool], enc_a: bool, enc_b: bool) -> InputStatus {
        for (input, &level) in self.two_state_inputs.iter_mut().zip(levels.iter()) {
            input.sync_from_gpio(level);
        }
        self.apply_snapshot_values(enc_a, enc_b)
    }

    fn apply_snapshot_values(&mut self, enc_a: bool, enc_b: bool) -> InputStatus {
        self.encoder.sync(enc_a, enc_b);
        self.pending_wheel = 0;
        self.current_status(0)
    }

    #[cfg_attr(coverage, coverage(off))]
    pub fn enable_interrupts(&self) {
        for input in &self.two_state_inputs {
            arm_next_level_interrupt(input.input_id, input.stable_active);
        }
        let _ = unsafe { c_vp_exti_set_edge(VP_INPUT_ENCODER_A as u8, VP_EXTI_EDGE_BOTH as u8) };
        let _ = unsafe { c_vp_exti_set_edge(VP_INPUT_ENCODER_B as u8, VP_EXTI_EDGE_BOTH as u8) };
    }

    #[cfg_attr(coverage, coverage(off))]
    pub fn on_button_exti(&mut self, button_id: u8, active: bool) -> bool {
        if !self.try_begin_debounce(button_id, active) {
            return false;
        }
        start_debounce_timer()
    }

    #[cfg_attr(coverage, coverage(off))]
    fn try_begin_debounce(&mut self, button_id: u8, active: bool) -> bool {
        let Some(input_id) = button_id_to_input_id(button_id) else {
            return false;
        };
        let Some(input) = self.two_state_input_mut(input_id) else {
            return false;
        };
        input.begin_debounce(active);
        true
    }

    #[cfg_attr(coverage, coverage(off))]
    pub fn on_debounce_tick(&mut self) -> bool {
        let levels: [_; BUTTON_COUNT] = BUTTON_IDS.map(|id| read_active_low_input(id));
        let (changed, any_debouncing, transitions, count) = self.process_debounce_levels(&levels);
        for t in &transitions[..count] {
            arm_next_level_interrupt(t.input_id, t.active);
            if t.changed {
                log_button_change(t.input_id, t.active);
            }
        }
        if !any_debouncing {
            stop_debounce_timer();
        }
        changed
    }

    fn process_debounce_levels(
        &mut self,
        levels: &[bool],
    ) -> (bool, bool, [StableTransition; BUTTON_COUNT], usize) {
        let mut changed = false;
        let mut any_debouncing = false;
        let mut transitions = [StableTransition {
            input_id: 0,
            active: false,
            changed: false,
        }; BUTTON_COUNT];
        let mut count = 0;
        for (input, &level) in self.two_state_inputs.iter_mut().zip(levels.iter()) {
            match input.sample(level) {
                DebounceTickOutcome::Idle => {}
                DebounceTickOutcome::StillDebouncing => {
                    any_debouncing = true;
                }
                DebounceTickOutcome::Stabilized(transition) => {
                    changed |= transition.changed;
                    transitions[count] = transition;
                    count += 1;
                }
            }
        }
        (changed, any_debouncing, transitions, count)
    }

    pub fn on_encoder_exti(&mut self, enc_a: bool, enc_b: bool) -> bool {
        let delta = self.encoder.update(enc_a, enc_b);
        self.pending_wheel = self.pending_wheel.saturating_add(delta);
        delta != 0
    }

    #[cfg_attr(coverage, coverage(off))]
    pub fn get_current_input(&mut self) -> InputStatus {
        let enc_a = read_active_low_input(VP_INPUT_ENCODER_A as u8);
        let enc_b = read_active_low_input(VP_INPUT_ENCODER_B as u8);
        self.poll_encoder(enc_a, enc_b)
    }

    fn poll_encoder(&mut self, enc_a: bool, enc_b: bool) -> InputStatus {
        let polled_wheel = self.encoder.update(enc_a, enc_b);
        self.pending_wheel = self.pending_wheel.saturating_add(polled_wheel);
        let wheel_delta = self.pending_wheel;
        self.pending_wheel = 0;
        self.current_status(wheel_delta)
    }

    fn current_status(&self, wheel_delta: i8) -> InputStatus {
        fn stable_at(inputs: &[DebouncedTwoStateInput; BUTTON_COUNT], id: u8) -> bool {
            inputs
                .iter()
                .find(|input| input.input_id == id)
                .map_or(false, |input| input.stable_active)
        }
        InputStatus {
            left: stable_at(&self.two_state_inputs, VP_INPUT_LEFT as u8),
            right: stable_at(&self.two_state_inputs, VP_INPUT_RIGHT as u8),
            middle: stable_at(&self.two_state_inputs, VP_INPUT_MIDDLE as u8),
            action: stable_at(&self.two_state_inputs, VP_INPUT_ACTION as u8),
            laser: stable_at(&self.two_state_inputs, VP_INPUT_LASER as u8),
            wheel_delta,
        }
    }

    fn two_state_input_mut(&mut self, input_id: u8) -> Option<&mut DebouncedTwoStateInput> {
        self.two_state_inputs
            .iter_mut()
            .find(|input| input.input_id == input_id)
    }
}

#[cfg_attr(coverage, coverage(off))]
fn arm_next_level_interrupt(input_id: u8, active: bool) {
    let edge = next_edge_for_active_low_state(active);
    let _ = unsafe { c_vp_exti_set_edge(input_id, edge as u8) };
}

fn next_edge_for_active_low_state(active: bool) -> u32 {
    if active {
        VP_EXTI_EDGE_RISING
    } else {
        VP_EXTI_EDGE_FALLING
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

// TODO: 后续移除，现有测试价值低，暂不测
#[cfg_attr(coverage, coverage(off))]
fn log_button_change(input_id: u8, pressed: bool) {
    let name = match input_id as u32 {
        VP_INPUT_LEFT => "left",
        VP_INPUT_RIGHT => "right",
        VP_INPUT_MIDDLE => "middle",
        VP_INPUT_ACTION => "action",
        VP_INPUT_LASER => "laser",
        _ => return,
    };
    log::debug!("button state changed;button={},pressed={}", name, pressed);
}

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn button_id_to_input_id_valid() {
        for id in 0..BUTTON_IDS.len() as u8 {
            assert_eq!(button_id_to_input_id(id), Some(BUTTON_IDS[id as usize]));
        }
    }

    #[test]
    fn button_id_to_input_id_out_of_range() {
        assert_eq!(button_id_to_input_id(99), None);
    }

    #[test]
    fn next_edge_for_active_low_active_returns_rising() {
        assert_eq!(next_edge_for_active_low_state(true), VP_EXTI_EDGE_RISING);
    }

    #[test]
    fn next_edge_for_active_low_inactive_returns_falling() {
        assert_eq!(next_edge_for_active_low_state(false), VP_EXTI_EDGE_FALLING);
    }

    #[test]
    fn debounce_new_is_idle() {
        let mut input = DebouncedTwoStateInput::new(0);
        assert_eq!(input.sample(true), DebounceTickOutcome::Idle);
    }

    #[test]
    fn debounce_stabilizes_after_enough_ticks() {
        let mut input = DebouncedTwoStateInput::new(0);
        input.begin_debounce(true);
        for _ in 0..DEBOUNCE_STABLE_TICKS - 1 {
            assert_eq!(input.sample(true), DebounceTickOutcome::StillDebouncing);
        }
        assert_eq!(
            input.sample(true),
            DebounceTickOutcome::Stabilized(StableTransition {
                input_id: 0,
                active: true,
                changed: true,
            })
        );
    }

    #[test]
    fn debounce_resets_on_transient_level_change() {
        let mut input = DebouncedTwoStateInput::new(0);
        input.begin_debounce(true);
        assert!(matches!(
            input.sample(true),
            DebounceTickOutcome::StillDebouncing
        ));
        assert!(matches!(
            input.sample(true),
            DebounceTickOutcome::StillDebouncing
        ));
        assert!(matches!(
            input.sample(false),
            DebounceTickOutcome::StillDebouncing
        ));
        for _ in 0..4 {
            assert!(matches!(
                input.sample(true),
                DebounceTickOutcome::StillDebouncing
            ));
        }
        assert_eq!(
            input.sample(true),
            DebounceTickOutcome::Stabilized(StableTransition {
                input_id: 0,
                active: true,
                changed: true,
            })
        );
    }

    #[test]
    fn debounce_no_change_when_same_state() {
        let mut input = DebouncedTwoStateInput::new(0);
        input.sync_from_gpio(true);
        input.begin_debounce(true);
        for _ in 0..DEBOUNCE_STABLE_TICKS - 1 {
            assert!(matches!(
                input.sample(true),
                DebounceTickOutcome::StillDebouncing
            ));
        }
        assert_eq!(
            input.sample(true),
            DebounceTickOutcome::Stabilized(StableTransition {
                input_id: 0,
                active: true,
                changed: false,
            })
        );
    }

    #[test]
    fn begin_debounce_sets_candidate() {
        let mut input = DebouncedTwoStateInput::new(0);
        input.sync_from_gpio(false);
        input.begin_debounce(true);
        assert!(input.debouncing);
        assert!(input.candidate_active);
        assert!(!input.stable_active);
        assert_eq!(input.stable_ticks, 0);
    }

    #[test]
    fn track_candidate_same_increments_ticks() {
        let mut input = DebouncedTwoStateInput::new(0);
        input.candidate_active = true;
        input.stable_ticks = 1;
        input.track_candidate(true);
        assert_eq!(input.stable_ticks, 2);
    }

    #[test]
    fn track_candidate_different_resets_ticks() {
        let mut input = DebouncedTwoStateInput::new(0);
        input.candidate_active = true;
        input.stable_ticks = 3;
        input.track_candidate(false);
        assert!(!input.candidate_active);
        assert_eq!(input.stable_ticks, 1);
    }

    #[test]
    fn candidate_not_stable_below_threshold() {
        let mut input = DebouncedTwoStateInput::new(0);
        input.stable_ticks = DEBOUNCE_STABLE_TICKS - 1;
        assert!(!input.candidate_is_stable());
    }

    #[test]
    fn candidate_stable_at_threshold() {
        let mut input = DebouncedTwoStateInput::new(0);
        input.stable_ticks = DEBOUNCE_STABLE_TICKS;
        assert!(input.candidate_is_stable());
    }

    #[test]
    fn accept_candidate_returns_transition() {
        let mut input = DebouncedTwoStateInput::new(0);
        input.sync_from_gpio(false);
        input.candidate_active = true;
        input.stable_ticks = DEBOUNCE_STABLE_TICKS;
        input.debouncing = true;

        let t = input.accept_candidate();
        assert_eq!(t.input_id, 0);
        assert!(t.active);
        assert!(t.changed);
        assert!(input.stable_active);
        assert_eq!(input.stable_ticks, 0);
        assert!(!input.debouncing);
    }

    #[test]
    fn try_begin_debounce_valid_button() {
        let mut m = InputManager::new();
        assert!(m.try_begin_debounce(0, true));
    }

    #[test]
    fn try_begin_debounce_invalid_button() {
        let mut m = InputManager::new();
        assert!(!m.try_begin_debounce(99, true));
    }

    #[test]
    fn input_manager_new_has_default_state() {
        let m = InputManager::new();
        assert_eq!(m.pending_wheel, 0);
        for input in &m.two_state_inputs {
            assert!(!input.stable_active);
            assert!(!input.debouncing);
        }
    }

    #[test]
    fn two_state_input_mut_finds_existing() {
        let mut m = InputManager::new();
        let input_id = BUTTON_IDS[0];
        let found = m.two_state_input_mut(input_id);
        assert!(found.is_some());
        assert_eq!(found.unwrap().input_id, input_id);
    }

    #[test]
    fn two_state_input_mut_not_found() {
        let mut m = InputManager::new();
        assert!(m.two_state_input_mut(99).is_none());
    }

    #[test]
    fn current_status_reflects_stable_state() {
        let mut m = InputManager::new();
        if let Some(input) = m.two_state_input_mut(BUTTON_IDS[0]) {
            input.stable_active = true;
        }
        let status = m.current_status(5);
        assert!(status.left);
        assert_eq!(status.wheel_delta, 5);
    }

    #[test]
    fn sync_with_gpio_values_updates_all_inputs() {
        let mut m = InputManager::new();
        let levels = [true; BUTTON_COUNT];
        let status = m.sync_with_gpio_values(&levels, false, false);
        assert!(status.left);
        assert_eq!(status.wheel_delta, 0);
    }

    #[test]
    fn apply_snapshot_values_syncs_encoder() {
        let mut m = InputManager::new();
        let status = m.apply_snapshot_values(true, false);
        assert_eq!(status.wheel_delta, 0);
    }

    #[test]
    fn process_debounce_levels_idle_when_not_debouncing() {
        let mut m = InputManager::new();
        let levels = [false; BUTTON_COUNT];
        let (changed, any_debouncing, _transitions, count) = m.process_debounce_levels(&levels);
        assert!(!changed);
        assert!(!any_debouncing);
        assert_eq!(count, 0);
    }

    #[test]
    fn process_debounce_levels_still_debouncing_below_threshold() {
        let mut m = InputManager::new();
        if let Some(input) = m.two_state_input_mut(BUTTON_IDS[0]) {
            input.begin_debounce(true);
        }
        let levels = [true; BUTTON_COUNT];
        let (changed, any_debouncing, _transitions, count) = m.process_debounce_levels(&levels);
        assert!(!changed);
        assert!(any_debouncing);
        assert_eq!(count, 0);
    }

    #[test]
    fn process_debounce_levels_stabilized_after_enough_ticks() {
        let mut m = InputManager::new();
        if let Some(input) = m.two_state_input_mut(BUTTON_IDS[0]) {
            input.begin_debounce(true);
        }
        let levels = [true; BUTTON_COUNT];
        for _ in 0..DEBOUNCE_STABLE_TICKS - 1 {
            let (_, any_debouncing, _, _) = m.process_debounce_levels(&levels);
            assert!(any_debouncing);
        }
        let (changed, any_debouncing, transitions, count) = m.process_debounce_levels(&levels);
        assert!(changed);
        assert!(!any_debouncing);
        assert_eq!(count, 1);
        assert_eq!(transitions[0].input_id, BUTTON_IDS[0]);
        assert!(transitions[0].active);
    }

    #[test]
    fn poll_encoder_returns_wheel() {
        let mut m = InputManager::new();
        m.pending_wheel = 3;
        let status = m.poll_encoder(false, false);
        assert_eq!(status.wheel_delta, 3);
    }

    #[test]
    fn on_encoder_exti_accumulates_delta() {
        let mut m = InputManager::new();
        let result = m.on_encoder_exti(true, false);
        assert!(!result);
        assert_eq!(m.pending_wheel, 0);
    }
}
