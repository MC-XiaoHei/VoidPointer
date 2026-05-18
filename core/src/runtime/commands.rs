use crate::ffi::bindings::*;
use crate::ffi::bindings::{VP_WAKE_SOURCE_BUTTON, VP_WAKE_SOURCE_ENCODER, VP_WAKE_SOURCE_IMU};
use crate::hid::api::HidApi;
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
                let status = HidApi::send_mouse(route, report);
                RuntimeCommandResult::MouseSent {
                    route,
                    report,
                    status,
                }
            }
            Self::SendVendor { route, report } => {
                let status = HidApi::send_vendor(route, &report);
                RuntimeCommandResult::VendorSent {
                    route,
                    report,
                    status,
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
