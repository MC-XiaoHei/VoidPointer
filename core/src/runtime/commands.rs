use crate::ffi::bindings::*;
use crate::ffi::bindings::{VP_WAKE_SOURCE_BUTTON, VP_WAKE_SOURCE_ENCODER, VP_WAKE_SOURCE_IMU};
use crate::hid::types::{CustomReport, HidSendStatus, MouseReport};
use crate::power::PowerState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeCommand {
    SendMouse {
        route: vp_hid_route_t,
        report: MouseReport,
    },
    SendVendor {
        route: vp_hid_route_t,
        report: CustomReport,
    },
    RequestPowerState {
        target: PowerState,
    },
    ReadImuFifo {
        max_samples: u16,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeCommandResult {
    MouseSent {
        route: vp_hid_route_t,
        report: MouseReport,
        status: HidSendStatus,
    },
    VendorSent {
        route: vp_hid_route_t,
        report: CustomReport,
        status: HidSendStatus,
    },
    PowerStateRequestDone {
        target: PowerState,
        accepted: bool,
    },
    ImuFifoReadRequested {
        status: vp_status_t,
    },
}

impl RuntimeCommand {
    pub fn execute(self) -> RuntimeCommandResult {
        match self {
            Self::SendMouse { route, report } => {
                let status = unsafe {
                    c_vp_hid_send_mouse(
                        route,
                        report.buttons.pack(),
                        report.dx,
                        report.dy,
                        report.wheel,
                    )
                };

                RuntimeCommandResult::MouseSent {
                    route,
                    report,
                    status: map_hid_status(status),
                }
            }
            Self::SendVendor { route, report } => {
                let status =
                    unsafe { c_vp_hid_send_vendor(route, report.data.as_ptr(), report.len) };

                RuntimeCommandResult::VendorSent {
                    route,
                    report,
                    status: map_hid_status(status),
                }
            }
            Self::RequestPowerState { target } => RuntimeCommandResult::PowerStateRequestDone {
                target,
                accepted: execute_power_state_request(target),
            },
            Self::ReadImuFifo { max_samples } => RuntimeCommandResult::ImuFifoReadRequested {
                status: unsafe { c_vp_imu_read_fifo_async(max_samples) },
            },
        }
    }
}

fn execute_power_state_request(target: PowerState) -> bool {
    // Active 是稳态，不需要额外切换动作
    match target {
        PowerState::Active => true,
        PowerState::Suspend => {
            if !enable_low_power_resume_sources(true) {
                return false;
            }

            let prepare_status = unsafe { c_vp_power_prepare_suspend() };
            let enter_status = unsafe { c_vp_power_enter_suspend() };
            let accepted =
                prepare_status == VP_STATUS_OK as u8 && enter_status == VP_STATUS_OK as u8;
            if !accepted {
                let _ = enable_low_power_resume_sources(false);
            }
            accepted
        }
        PowerState::Sleep => {
            if !enable_low_power_resume_sources(true) {
                return false;
            }

            let prepare_status = unsafe { c_vp_power_prepare_sleep() };
            let enter_status = unsafe { c_vp_power_enter_sleep() };
            let accepted =
                prepare_status == VP_STATUS_OK as u8 && enter_status == VP_STATUS_OK as u8;
            if !accepted {
                let _ = enable_low_power_resume_sources(false);
            }
            accepted
        }
    }
}

fn enable_low_power_resume_sources(enabled: bool) -> bool {
    for source in [
        VP_WAKE_SOURCE_BUTTON,
        VP_WAKE_SOURCE_ENCODER,
        VP_WAKE_SOURCE_IMU,
    ] {
        let status = unsafe { c_vp_wake_source_enable(source, if enabled { 1 } else { 0 }) };
        if status != VP_STATUS_OK as u8 {
            return false;
        }
    }

    true
}

#[allow(non_upper_case_globals)]
fn map_hid_status(status: vp_hid_send_status_t) -> HidSendStatus {
    // 未识别状态一律按 fatal 处理，避免在未知返回值上乐观重试
    match status {
        x if x == VP_HID_SEND_SENT as u8 => HidSendStatus::Sent,
        x if x == VP_HID_SEND_RETRY_LATER as u8 => HidSendStatus::RetryLater,
        x if x == VP_HID_SEND_NOT_CONNECTED as u8 => HidSendStatus::NotConnected,
        x if x == VP_HID_SEND_FATAL as u8 => HidSendStatus::Fatal,
        _ => HidSendStatus::Fatal,
    }
}
