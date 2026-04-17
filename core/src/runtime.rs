use crate::get_current_attitude;
use crate::hid::sender::BleHidSender;
use crate::hid::sender::HidSender;
use crate::hid::types::HidSendStatus;
use crate::hid::types::MouseReport;
use crate::motion::config::MotionConfig;
use crate::motion::resolver::TiltMotionSolver;
use crate::motion::state::MotionState;
use crate::report::config::ReportConfig;
use crate::report::state::ReportState;

pub static mut RUNTIME: Option<Runtime> = None;

pub struct Runtime {
    solver: TiltMotionSolver,
    report_state: ReportState,
    hid_sender: BleHidSender,
}

impl Runtime {
    pub fn new() -> Self {
        Self {
            solver: TiltMotionSolver::new(MotionConfig::default()),
            report_state: ReportState::new(ReportConfig::default()),
            hid_sender: BleHidSender::new(),
        }
    }

    pub fn tick(&mut self) {
        let attitude_opt = get_current_attitude();

        let motion = match attitude_opt {
            Some(attitude) => self.solver.update(attitude),
            None => MotionState {
                vx: 0.0,
                vy: 0.0,
                valid: false,
            },
        };

        self.report_state.ingest_motion(motion);

        if let Some(delta) = self.report_state.peek_report() {
            let mouse_report = MouseReport {
                buttons: 0,
                dx: delta.dx,
                dy: delta.dy,
                wheel: 0,
            };

            // info!("{mouse_report:#?}");

            match self.hid_sender.send_mouse_report(mouse_report) {
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
