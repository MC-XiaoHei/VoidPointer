use crate::ffi::bindings::{
    VP_EXTI_EDGE_FALLING, VP_EXTI_EDGE_RISING, VP_INPUT_ACTION, VP_INPUT_ENCODER_A,
    VP_INPUT_ENCODER_B, VP_INPUT_LEFT, VP_INPUT_MIDDLE, VP_INPUT_RIGHT, VP_STATUS_OK,
    c_vp_debounce_timer_start, c_vp_debounce_timer_stop, c_vp_exti_set_edge, c_vp_gpio_read,
};
use crate::input::encoder::RotaryEncoder;
use log::info;

const DEBOUNCE_STABLE_TICKS: u8 = 5;
const DEBOUNCED_BUTTONS: usize = 4;

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
struct DebouncedButton {
    input_id: u8,
    stable_level: bool,
    candidate_level: bool,
    stable_ticks: u8,
    active: bool,
}

impl DebouncedButton {
    const fn new(input_id: u8) -> Self {
        Self {
            input_id,
            stable_level: false,
            candidate_level: false,
            stable_ticks: 0,
            active: false,
        }
    }

    fn sync(&mut self) {
        let level = unsafe { c_vp_gpio_read(self.input_id) } != 0;
        self.stable_level = level;
        self.candidate_level = level;
        self.stable_ticks = 0;
        self.active = false;
    }

    fn begin(&mut self, level: bool) {
        self.candidate_level = level;
        self.stable_ticks = 0;
        self.active = true;
    }

    fn sample(&mut self) -> DebounceSampleResult {
        if !self.active {
            return DebounceSampleResult::Idle;
        }

        let sample = unsafe { c_vp_gpio_read(self.input_id) } != 0;
        if sample == self.candidate_level {
            self.stable_ticks = self.stable_ticks.saturating_add(1);
        } else {
            self.candidate_level = sample;
            self.stable_ticks = 1;
        }

        if self.stable_ticks < DEBOUNCE_STABLE_TICKS {
            return DebounceSampleResult::Active;
        }

        let changed = self.stable_level != self.candidate_level;
        self.stable_level = self.candidate_level;
        self.active = false;
        self.stable_ticks = 0;

        DebounceSampleResult::Stable {
            level: self.stable_level,
            changed,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DebounceSampleResult {
    Idle,
    Active,
    Stable { level: bool, changed: bool },
}

pub struct InputManager {
    encoder: RotaryEncoder,
    buttons: [DebouncedButton; DEBOUNCED_BUTTONS],
    pending_wheel: i8,
}

impl InputManager {
    pub fn new() -> Self {
        Self {
            encoder: RotaryEncoder::new(),
            buttons: [
                DebouncedButton::new(VP_INPUT_LEFT as u8),
                DebouncedButton::new(VP_INPUT_RIGHT as u8),
                DebouncedButton::new(VP_INPUT_MIDDLE as u8),
                DebouncedButton::new(VP_INPUT_ACTION as u8),
            ],
            pending_wheel: 0,
        }
    }

    pub fn sync_snapshot(&mut self) -> InputStatus {
        for button in &mut self.buttons {
            button.sync();
        }

        let enc_a = unsafe { c_vp_gpio_read(VP_INPUT_ENCODER_A as u8) } != 0;
        let enc_b = unsafe { c_vp_gpio_read(VP_INPUT_ENCODER_B as u8) } != 0;
        self.encoder.sync(enc_a, enc_b);
        self.pending_wheel = 0;

        self.current_status(0)
    }

    pub fn on_button_exti(&mut self, button_id: u8, level: bool) -> bool {
        let Some(input_id) = button_id_to_input_id(button_id) else {
            return false;
        };
        let Some(button) = self.button_mut(input_id) else {
            return false;
        };

        button.begin(level);
        unsafe { c_vp_debounce_timer_start() == VP_STATUS_OK as u8 }
    }

    pub fn on_debounce_tick(&mut self) -> bool {
        let mut changed = false;
        let mut any_active = false;

        for button in &mut self.buttons {
            match button.sample() {
                DebounceSampleResult::Idle => {}
                DebounceSampleResult::Active => {
                    any_active = true;
                }
                DebounceSampleResult::Stable {
                    level,
                    changed: button_changed,
                } => {
                    if button_changed {
                        log_button_change(button.input_id, level);
                    }
                    rearm_button_exti(button.input_id, level);
                    changed |= button_changed;
                }
            }
        }

        if !any_active {
            let _ = unsafe { c_vp_debounce_timer_stop() };
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
        let enc_a = unsafe { c_vp_gpio_read(VP_INPUT_ENCODER_A as u8) } != 0;
        let enc_b = unsafe { c_vp_gpio_read(VP_INPUT_ENCODER_B as u8) } != 0;
        let polled_wheel = self.encoder.update(enc_a, enc_b);
        self.pending_wheel = self.pending_wheel.saturating_add(polled_wheel);

        let wheel_delta = self.pending_wheel;
        self.pending_wheel = 0;
        self.current_status(wheel_delta)
    }

    fn current_status(&self, wheel_delta: i8) -> InputStatus {
        InputStatus {
            left: self.button_level(VP_INPUT_LEFT as u8),
            right: self.button_level(VP_INPUT_RIGHT as u8),
            middle: self.button_level(VP_INPUT_MIDDLE as u8),
            light: false,
            action: self.button_level(VP_INPUT_ACTION as u8),
            wheel_delta,
        }
    }

    fn button_level(&self, input_id: u8) -> bool {
        self.buttons
            .iter()
            .find(|button| button.input_id == input_id)
            .map(|button| button.stable_level)
            .unwrap_or(false)
    }

    fn button_mut(&mut self, input_id: u8) -> Option<&mut DebouncedButton> {
        self.buttons
            .iter_mut()
            .find(|button| button.input_id == input_id)
    }
}

fn rearm_button_exti(input_id: u8, active_level: bool) {
    let edge = if active_level {
        // Active-low input is currently low; wait for physical high/release.
        VP_EXTI_EDGE_RISING
    } else {
        // Active-low input is currently high; wait for physical low/press.
        VP_EXTI_EDGE_FALLING
    };
    let _ = unsafe { c_vp_exti_set_edge(input_id, edge as u8) };
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
