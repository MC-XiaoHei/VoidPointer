use crate::attitude::types::AttitudeData;
use crate::motion::config::MotionConfig;
use crate::motion::resolver::TiltMotionSolver;
use crate::motion::state::MotionState;

#[derive(Clone, Copy)]
pub struct TriggerButtons {
    pub action: bool,
    pub middle: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SessionState {
    Idle,
    Calibrating,
    Active,
}

pub struct MotionSession {
    state: SessionState,
    solver: TiltMotionSolver,
    cfg: MotionConfig,
    current_output: MotionState,
    last_sample_ts: Option<u32>,
}

impl MotionSession {
    pub fn new(cfg: MotionConfig) -> Self {
        Self {
            state: SessionState::Idle,
            solver: TiltMotionSolver::new(cfg),
            cfg,
            current_output: MotionState::default(),
            last_sample_ts: None,
        }
    }

    pub fn reconfigure(&mut self, cfg: MotionConfig) {
        self.cfg = cfg;
        self.solver = TiltMotionSolver::new(cfg);
        self.reset();
    }

    /// 必须先于 `update_attitude` 调用
    pub fn update_trigger(&mut self, buttons: TriggerButtons) -> bool {
        let active = buttons.action || (buttons.middle && self.cfg.middle_triggers_motion);

        match (self.state, active) {
            (SessionState::Idle, true) => {
                self.state = SessionState::Calibrating;
                self.current_output = MotionState::default();
            }
            (SessionState::Calibrating | SessionState::Active, false) => {
                self.state = SessionState::Idle;
                self.current_output = MotionState::default();
                self.last_sample_ts = None;
            }
            _ => {}
        }

        self.state != SessionState::Idle
    }

    pub fn should_process_sample(&self, latest_ts: u32, latest_valid: bool) -> bool {
        self.state != SessionState::Idle && latest_valid && self.last_sample_ts != Some(latest_ts)
    }

    pub fn update_attitude(&mut self, attitude: &AttitudeData, timestamp: u32) -> MotionState {
        self.last_sample_ts = Some(timestamp);

        if !attitude.is_valid() {
            self.current_output = MotionState::default();
            return self.current_output;
        }

        match self.state {
            SessionState::Calibrating => {
                self.solver.calibrate(*attitude);
                self.current_output = MotionState::default();
                self.state = SessionState::Active;
            }
            SessionState::Active => {
                self.current_output = self.solver.update(*attitude);
            }
            SessionState::Idle => {}
        }

        self.current_output
    }

    pub fn is_active(&self) -> bool {
        self.state != SessionState::Idle
    }

    pub fn output(&self) -> MotionState {
        self.current_output
    }

    pub fn reset(&mut self) {
        self.state = SessionState::Idle;
        self.current_output = MotionState::default();
        self.last_sample_ts = None;
    }
}

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use super::*;

    fn attitude(roll: f32, pitch: f32, yaw: f32) -> AttitudeData {
        AttitudeData {
            roll,
            pitch,
            yaw,
            w: 1.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    #[test]
    fn new_is_idle() {
        let s = MotionSession::new(MotionConfig::default());
        assert!(!s.is_active());
        assert_eq!(s.output().vx, 0.0);
        assert_eq!(s.output().vy, 0.0);
    }

    #[test]
    fn trigger_activates_on_action() {
        let mut s = MotionSession::new(MotionConfig::default());
        assert!(s.update_trigger(TriggerButtons {
            action: true,
            middle: false
        }));
        assert!(s.is_active());
    }

    #[test]
    fn trigger_activates_on_middle() {
        let cfg = MotionConfig {
            middle_triggers_motion: true,
            ..MotionConfig::default()
        };
        let mut s = MotionSession::new(cfg);
        assert!(s.update_trigger(TriggerButtons {
            action: false,
            middle: true
        }));
        assert!(s.is_active());
    }

    #[test]
    fn trigger_activates_on_both() {
        let cfg = MotionConfig {
            middle_triggers_motion: true,
            ..MotionConfig::default()
        };
        let mut s = MotionSession::new(cfg);
        assert!(s.update_trigger(TriggerButtons {
            action: true,
            middle: true
        }));
        assert!(s.is_active());
    }

    #[test]
    fn middle_does_not_trigger_when_disabled() {
        let cfg = MotionConfig {
            middle_triggers_motion: false,
            ..MotionConfig::default()
        };
        let mut s = MotionSession::new(cfg);
        assert!(!s.update_trigger(TriggerButtons {
            action: false,
            middle: true
        }));
        assert!(!s.is_active());
    }

    #[test]
    fn action_still_triggers_when_middle_disabled() {
        let cfg = MotionConfig {
            middle_triggers_motion: false,
            ..MotionConfig::default()
        };
        let mut s = MotionSession::new(cfg);
        assert!(s.update_trigger(TriggerButtons {
            action: true,
            middle: false
        }));
        assert!(s.is_active());
    }

    #[test]
    fn trigger_deactivates_on_release() {
        let mut s = MotionSession::new(MotionConfig::default());
        s.update_trigger(TriggerButtons {
            action: true,
            middle: false,
        });
        assert!(!s.update_trigger(TriggerButtons {
            action: false,
            middle: false
        }));
        assert!(!s.is_active());
    }

    #[test]
    fn should_not_process_when_idle() {
        let s = MotionSession::new(MotionConfig::default());
        assert!(!s.should_process_sample(100, true));
    }

    #[test]
    fn should_process_when_active_with_new_sample() {
        let mut s = MotionSession::new(MotionConfig::default());
        s.update_trigger(TriggerButtons {
            action: true,
            middle: false,
        });
        assert!(s.should_process_sample(100, true));
    }

    #[test]
    fn should_not_process_duplicate_ts() {
        let mut s = MotionSession::new(MotionConfig::default());
        s.update_trigger(TriggerButtons {
            action: true,
            middle: false,
        });
        let raw = crate::attitude::types::SflpGameRotationRaw { x: 0, y: 0, z: 0 };
        let attitude = AttitudeData::from(raw);
        s.update_attitude(&attitude, 100);
        assert!(!s.should_process_sample(100, true));
    }

    #[test]
    fn should_not_process_invalid_sample() {
        let mut s = MotionSession::new(MotionConfig::default());
        s.update_trigger(TriggerButtons {
            action: true,
            middle: false,
        });
        assert!(!s.should_process_sample(100, false));
    }

    #[test]
    fn first_update_calibrates_then_second_updates() {
        let mut s = MotionSession::new(MotionConfig::default());
        s.update_trigger(TriggerButtons {
            action: true,
            middle: false,
        });

        let raw = crate::attitude::types::SflpGameRotationRaw { x: 0, y: 0, z: 0 };
        let att = AttitudeData::from(raw);
        let result = s.update_attitude(&att, 100);
        assert_eq!(result.vx, 0.0);
        assert_eq!(result.vy, 0.0);

        let att2 = attitude(0.1, 0.2, 0.3);
        let result2 = s.update_attitude(&att2, 200);
        assert!(result2.valid);
    }

    #[test]
    fn invalid_attitude_returns_zero() {
        let mut s = MotionSession::new(MotionConfig::default());
        s.update_trigger(TriggerButtons {
            action: true,
            middle: false,
        });

        let bad = AttitudeData {
            roll: f32::NAN,
            ..AttitudeData::default()
        };
        let result = s.update_attitude(&bad, 100);
        assert_eq!(result.vx, 0.0);
        assert_eq!(result.vy, 0.0);
    }

    #[test]
    fn reconfigure_resets_state() {
        let mut s = MotionSession::new(MotionConfig::default());
        s.update_trigger(TriggerButtons {
            action: true,
            middle: false,
        });
        assert!(s.is_active());

        s.reconfigure(MotionConfig::default());
        assert!(!s.is_active());
    }

    #[test]
    fn reset_clears_state() {
        let mut s = MotionSession::new(MotionConfig::default());
        s.update_trigger(TriggerButtons {
            action: true,
            middle: false,
        });
        let raw = crate::attitude::types::SflpGameRotationRaw { x: 0, y: 0, z: 0 };
        let att = AttitudeData::from(raw);
        s.update_attitude(&att, 100);
        assert!(s.is_active());

        s.reset();
        assert!(!s.is_active());
        assert!(!s.should_process_sample(200, true));
        assert_eq!(s.output().vx, 0.0);
    }

    #[test]
    fn release_during_calibrating_goes_idle() {
        let mut s = MotionSession::new(MotionConfig::default());
        s.update_trigger(TriggerButtons {
            action: true,
            middle: false,
        });
        assert!(s.is_active());

        s.update_trigger(TriggerButtons {
            action: false,
            middle: false,
        });
        assert!(!s.is_active());
        assert!(!s.should_process_sample(100, true));
    }

    #[test]
    fn re_trigger_after_release_restarts() {
        let mut s = MotionSession::new(MotionConfig::default());
        s.update_trigger(TriggerButtons {
            action: true,
            middle: false,
        });
        let raw = crate::attitude::types::SflpGameRotationRaw { x: 0, y: 0, z: 0 };
        let att = AttitudeData::from(raw);
        s.update_attitude(&att, 100);

        s.update_trigger(TriggerButtons {
            action: false,
            middle: false,
        });

        assert!(s.update_trigger(TriggerButtons {
            action: true,
            middle: false
        }));

        let att_cal = attitude(0.5, 0.0, 0.0);
        let result = s.update_attitude(&att_cal, 200);
        assert_eq!(result.vx, 0.0);

        let att_move = attitude(0.5, 0.3, 0.0);
        let result2 = s.update_attitude(&att_move, 300);
        assert!(result2.vx.abs() > 0.0 || result2.vy.abs() > 0.0);
    }

    #[test]
    fn output_returns_last_result() {
        let mut s = MotionSession::new(MotionConfig::default());
        s.update_trigger(TriggerButtons {
            action: true,
            middle: false,
        });
        let raw = crate::attitude::types::SflpGameRotationRaw { x: 0, y: 0, z: 0 };
        let att = AttitudeData::from(raw);
        s.update_attitude(&att, 100);

        let out = s.output();
        assert_eq!(out.vx, 0.0);
        assert_eq!(out.vy, 0.0);
    }
}
