use crate::ffi::bindings::{
    VP_EXTI_EDGE_FALLING, VP_EXTI_EDGE_RISING, VP_INPUT_ACTION, VP_INPUT_ENCODER_A,
    VP_INPUT_ENCODER_B, VP_INPUT_LEFT, VP_INPUT_MIDDLE, VP_INPUT_RIGHT, VP_STATUS_OK,
    c_vp_debounce_timer_start, c_vp_debounce_timer_stop, c_vp_exti_set_edge, c_vp_gpio_read,
};
use crate::input::encoder::RotaryEncoder;
use log::info;

const DEBOUNCE_STABLE_TICKS: u8 = 5;
const DEBOUNCED_TWO_STATE_INPUTS: usize = 4;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InputStatus {
    pub left: bool,
    pub right: bool,
    pub middle: bool,
    pub light: bool,
    pub action: bool,
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

    fn sync_from_gpio(&mut self) {
        let active = read_active_low_input(self.input_id);
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

    fn sample(&mut self) -> DebounceTickOutcome {
        if !self.debouncing {
            return DebounceTickOutcome::Idle;
        }

        self.track_candidate(read_active_low_input(self.input_id));

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
    two_state_inputs: [DebouncedTwoStateInput; DEBOUNCED_TWO_STATE_INPUTS],
    pending_wheel: i8,
}

impl InputManager {
    pub fn new() -> Self {
        Self {
            encoder: RotaryEncoder::new(),
            two_state_inputs: [
                DebouncedTwoStateInput::new(VP_INPUT_LEFT as u8),
                DebouncedTwoStateInput::new(VP_INPUT_RIGHT as u8),
                DebouncedTwoStateInput::new(VP_INPUT_MIDDLE as u8),
                DebouncedTwoStateInput::new(VP_INPUT_ACTION as u8),
            ],
            pending_wheel: 0,
        }
    }

    pub fn sync_snapshot(&mut self) -> InputStatus {
        for input in &mut self.two_state_inputs {
            input.sync_from_gpio();
            arm_next_level_interrupt(input.input_id, input.stable_active);
        }

        let enc_a = read_active_low_input(VP_INPUT_ENCODER_A as u8);
        let enc_b = read_active_low_input(VP_INPUT_ENCODER_B as u8);
        self.encoder.sync(enc_a, enc_b);
        self.pending_wheel = 0;

        self.current_status(0)
    }

    pub fn on_button_exti(&mut self, button_id: u8, active: bool) -> bool {
        let Some(input_id) = button_id_to_input_id(button_id) else {
            return false;
        };
        let Some(input) = self.two_state_input_mut(input_id) else {
            return false;
        };

        input.begin_debounce(active);
        start_debounce_timer()
    }

    pub fn on_debounce_tick(&mut self) -> bool {
        let mut changed = false;
        let mut any_debouncing = false;

        for input in &mut self.two_state_inputs {
            match input.sample() {
                DebounceTickOutcome::Idle => {}
                DebounceTickOutcome::StillDebouncing => {
                    any_debouncing = true;
                }
                DebounceTickOutcome::Stabilized(transition) => {
                    publish_stable_transition(transition);
                    changed |= transition.changed;
                }
            }
        }

        if !any_debouncing {
            stop_debounce_timer();
        }

        changed
    }

    pub fn on_encoder_exti(&mut self, enc_a: bool, enc_b: bool) -> bool {
        let delta = self.encoder.update(enc_a, enc_b);
        self.pending_wheel = self.pending_wheel.saturating_add(delta);
        delta != 0
    }

    pub fn get_current_input(&mut self) -> InputStatus {
        // Event-time encoder resync. The primary wheel path is EncoderExti;
        // this read normally produces zero after the queued EXTI state has been
        // applied, but keeps the encoder state coherent across missed edges.
        let enc_a = read_active_low_input(VP_INPUT_ENCODER_A as u8);
        let enc_b = read_active_low_input(VP_INPUT_ENCODER_B as u8);
        let polled_wheel = self.encoder.update(enc_a, enc_b);
        self.pending_wheel = self.pending_wheel.saturating_add(polled_wheel);

        let wheel_delta = self.pending_wheel;
        self.pending_wheel = 0;
        self.current_status(wheel_delta)
    }

    fn current_status(&self, wheel_delta: i8) -> InputStatus {
        InputStatus {
            left: self.stable_active_level(VP_INPUT_LEFT as u8),
            right: self.stable_active_level(VP_INPUT_RIGHT as u8),
            middle: self.stable_active_level(VP_INPUT_MIDDLE as u8),
            light: false,
            action: self.stable_active_level(VP_INPUT_ACTION as u8),
            wheel_delta,
        }
    }

    fn stable_active_level(&self, input_id: u8) -> bool {
        self.two_state_inputs
            .iter()
            .find(|input| input.input_id == input_id)
            .map(|input| input.stable_active)
            .unwrap_or(false)
    }

    fn two_state_input_mut(&mut self, input_id: u8) -> Option<&mut DebouncedTwoStateInput> {
        self.two_state_inputs
            .iter_mut()
            .find(|input| input.input_id == input_id)
    }
}

fn publish_stable_transition(transition: StableTransition) {
    arm_next_level_interrupt(transition.input_id, transition.active);

    if transition.changed {
        log_button_change(transition.input_id, transition.active);
    }
}

fn arm_next_level_interrupt(input_id: u8, active: bool) {
    let edge = next_edge_for_active_low_state(active);
    let _ = unsafe { c_vp_exti_set_edge(input_id, edge as u8) };
}

fn next_edge_for_active_low_state(active: bool) -> u32 {
    if active {
        // Active-low input is currently low; wait for physical high/release.
        VP_EXTI_EDGE_RISING
    } else {
        // Active-low input is currently high; wait for physical low/press.
        VP_EXTI_EDGE_FALLING
    }
}

fn read_active_low_input(input_id: u8) -> bool {
    unsafe { c_vp_gpio_read(input_id) != 0 }
}

fn start_debounce_timer() -> bool {
    unsafe { c_vp_debounce_timer_start() == VP_STATUS_OK as u8 }
}

fn stop_debounce_timer() {
    let _ = unsafe { c_vp_debounce_timer_stop() };
}

fn log_button_change(input_id: u8, pressed: bool) {
    if input_id == VP_INPUT_ACTION as u8 {
        if pressed {
            info!("Action pressed");
        } else {
            info!("Action released");
        }
    }
}

fn button_id_to_input_id(button_id: u8) -> Option<u8> {
    match button_id {
        0 => Some(VP_INPUT_LEFT as u8),
        1 => Some(VP_INPUT_RIGHT as u8),
        2 => Some(VP_INPUT_MIDDLE as u8),
        3 => Some(VP_INPUT_ACTION as u8),
        _ => None,
    }
}
