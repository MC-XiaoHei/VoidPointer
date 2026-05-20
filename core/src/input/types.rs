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
        // EXTI 只负责报出候选电平，是否采信交给后续定时采样确认
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

    pub fn sync_snapshot(&mut self) -> InputStatus {
        for input in &mut self.two_state_inputs {
            input.sync_from_gpio(read_active_low_input(input.input_id));
        }

        let enc_a = read_active_low_input(VP_INPUT_ENCODER_A as u8);
        let enc_b = read_active_low_input(VP_INPUT_ENCODER_B as u8);
        self.encoder.sync(enc_a, enc_b);
        self.pending_wheel = 0;

        self.current_status(0)
    }

    pub fn enable_interrupts(&self) {
        // 两态输入只监听下一条会改变稳定状态的边沿，避免在当前电平上空转
        for input in &self.two_state_inputs {
            arm_next_level_interrupt(input.input_id, input.stable_active);
        }

        let _ = unsafe { c_vp_exti_set_edge(VP_INPUT_ENCODER_A as u8, VP_EXTI_EDGE_BOTH as u8) };
        let _ = unsafe { c_vp_exti_set_edge(VP_INPUT_ENCODER_B as u8, VP_EXTI_EDGE_BOTH as u8) };
    }

    pub fn on_button_exti(&mut self, button_id: u8, active: bool) -> bool {
        let Some(input_id) = button_id_to_input_id(button_id) else {
            return false;
        };
        let Some(input) = self.two_state_input_mut(input_id) else {
            return false;
        };

        // 中断里只启动去抖窗口，不在这里直接改 stable 状态
        input.begin_debounce(active);
        start_debounce_timer()
    }

    pub fn on_debounce_tick(&mut self) -> bool {
        let mut changed = false;
        let mut any_debouncing = false;

        for input in &mut self.two_state_inputs {
            let level = read_active_low_input(input.input_id);
            match input.sample(level) {
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
        // 编码器相位变化在中断里先累计，是否形成完整步进由解码器决定
        let delta = self.encoder.update(enc_a, enc_b);
        self.pending_wheel = self.pending_wheel.saturating_add(delta);
        delta != 0
    }

    pub fn get_current_input(&mut self) -> InputStatus {
        // 轮询时再对齐一次编码器状态，避免漏边沿后长期漂移
        let enc_a = read_active_low_input(VP_INPUT_ENCODER_A as u8);
        let enc_b = read_active_low_input(VP_INPUT_ENCODER_B as u8);
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

fn publish_stable_transition(transition: StableTransition) {
    // 状态稳定后立刻挂到下一条相反电平边沿，避免同一状态反复触发
    arm_next_level_interrupt(transition.input_id, transition.active);

    if transition.changed {
        log_button_change(transition.input_id, transition.active);
    }
}

fn arm_next_level_interrupt(input_id: u8, active: bool) {
    // 低有效输入在 active 时要等释放沿，在 inactive 时要等按下沿
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
        // 2 true
        assert!(matches!(
            input.sample(true),
            DebounceTickOutcome::StillDebouncing
        ));
        assert!(matches!(
            input.sample(true),
            DebounceTickOutcome::StillDebouncing
        ));
        // 突然变成 false → 重置计数
        assert!(matches!(
            input.sample(false),
            DebounceTickOutcome::StillDebouncing
        ));
        // 第一个 true 把候选拉回 true（ticks=1），再来 4 个 true 后稳定
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
        // 前 4 个 true → StillDebouncing，第 5 个 → Stabilized
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
}
