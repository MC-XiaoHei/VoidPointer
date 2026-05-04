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
        // 启动或重同步时直接对齐相位，避免把历史毛刺累计成滚轮步进
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

        // 一格机械步进通常会经过 4 个合法相位变化，这里只在累计满一格后上报
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
