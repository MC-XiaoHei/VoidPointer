use crate::get_current_attitude;
use crate::hid::sender::BleHidSender;
use crate::hid::sender::HidSender;
use crate::hid::types::MouseReport;
use crate::hid::types::{HidSendStatus, MouseButtons};
use crate::input::types::InputManager;
use crate::motion::config::MotionConfig;
use crate::motion::resolver::TiltMotionSolver;
use crate::motion::state::MotionState;
use crate::report::config::ReportConfig;
use crate::report::state::ReportState;
use crate::utils::global::MainLoopGlobal;

pub static RUNTIME: MainLoopGlobal<Runtime> = MainLoopGlobal::new();

pub struct Runtime {
    solver: TiltMotionSolver,
    report_state: ReportState,
    hid_sender: BleHidSender,
    input_manager: InputManager,
}

impl Runtime {
    pub fn new() -> Self {
        Self {
            solver: TiltMotionSolver::new(MotionConfig::default()),
            report_state: ReportState::new(ReportConfig::default()),
            hid_sender: BleHidSender::new(),
            input_manager: InputManager::new(),
        }
    }
    pub fn tick(&mut self) {
        let attitude_opt = get_current_attitude();
        let current_input = self.input_manager.get_current_input();

        let motion = match attitude_opt {
            Some(attitude) => self.solver.update(attitude),
            None => MotionState {
                vx: 0.0,
                vy: 0.0,
                valid: false,
            },
        };

        self.report_state.ingest_motion(motion);
        let delta_opt = self.report_state.peek_report();

        if let Some(delta) = delta_opt {
            let mouse_report = MouseReport {
                buttons: MouseButtons {
                    left: current_input.left,
                    right: current_input.right,
                    middle: current_input.middle,
                },
                dx: delta.dx,
                dy: delta.dy,
                wheel: current_input.wheel_delta,
            };
            let send_status = self.hid_sender.send_mouse_report(mouse_report);
            match send_status {
                HidSendStatus::Sent => {
                    self.report_state.commit_sent(delta);
                }
                HidSendStatus::RetryLater => {}
                HidSendStatus::Fatal => {
                    self.report_state.clear_pending();
                }
            }
        }
    }
}
