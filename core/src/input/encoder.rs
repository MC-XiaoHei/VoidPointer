#[derive(Debug, Clone, Default)]
pub struct RotaryEncoder {
    prev_state: u8,
    accum: i8,
}

impl RotaryEncoder {
    pub fn new() -> Self {
        Self {
            prev_state: 0,
            accum: 0,
        }
    }

    pub fn sync(&mut self, enc_a: bool, enc_b: bool) {
        self.prev_state = ((enc_a as u8) << 1) | (enc_b as u8);
        self.accum = 0;
    }

    pub fn update(&mut self, enc_a: bool, enc_b: bool) -> i8 {
        let current_state = ((enc_a as u8) << 1) | (enc_b as u8);
        let state_transition = (self.prev_state << 2) | current_state;

        let delta = match state_transition {
            0b0010 | 0b1011 | 0b1101 | 0b0100 => 1,
            0b0001 | 0b0111 | 0b1110 | 0b1000 => -1,
            _ => 0,
        };

        self.prev_state = current_state;
        self.accum += delta;

        if self.accum >= 4 {
            self.accum -= 4;
            1
        } else if self.accum <= -4 {
            self.accum += 4;
            -1
        } else {
            0
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn new_is_default() {
        let e = RotaryEncoder::new();
        assert_eq!(e.accum, 0);
    }

    #[test]
    fn sync_resets_accum() {
        let mut e = RotaryEncoder::new();
        e.update(true, false);
        e.sync(false, false);
        assert_eq!(e.accum, 0);
    }

    #[test]
    fn full_step_forward() {
        let mut e = RotaryEncoder::new();
        // 00 → 10 → 11 → 01 → 00
        assert_eq!(e.update(false, false), 0);
        assert_eq!(e.update(true, false), 0);
        assert_eq!(e.update(true, true), 0);
        assert_eq!(e.update(false, true), 0);
        assert_eq!(e.update(false, false), 1);
    }

    #[test]
    fn full_step_backward() {
        let mut e = RotaryEncoder::new();
        // 00 → 01 → 11 → 10 → 00
        assert_eq!(e.update(false, false), 0);
        assert_eq!(e.update(false, true), 0);
        assert_eq!(e.update(true, true), 0);
        assert_eq!(e.update(true, false), 0);
        assert_eq!(e.update(false, false), -1);
    }

    #[test]
    fn partial_then_reverse() {
        let mut e = RotaryEncoder::new();
        e.update(false, true);
        e.update(true, true);
        e.update(true, false);
        e.update(true, true);
        let result = e.update(false, true);
        assert_eq!(result, 0);
    }

    #[test]
    fn sync_clears_partial() {
        let mut e = RotaryEncoder::new();
        e.update(false, true);
        e.update(true, true);
        e.sync(false, false);
        assert_eq!(e.update(false, true), 0);
    }

    #[test]
    fn two_full_steps_forward() {
        let mut e = RotaryEncoder::new();
        for _ in 0..2 {
            assert_eq!(e.update(true, false), 0);
            assert_eq!(e.update(true, true), 0);
            assert_eq!(e.update(false, true), 0);
            assert_eq!(e.update(false, false), 1);
        }
    }

    #[test]
    fn bounce_on_one_phase() {
        let mut e = RotaryEncoder::new();
        e.update(true, false);
        e.update(true, true);
        e.update(true, false);
        e.update(true, true);
        e.update(true, false);
        let r = e.update(true, true);
        assert_eq!(r, 0);
    }
}
