use crate::ffi::bindings::*;
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
    PowerTransition {
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
    PowerTransitionDone {
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
            Self::PowerTransition { target } => RuntimeCommandResult::PowerTransitionDone {
                target,
                accepted: execute_power_transition(target),
            },
            Self::ReadImuFifo { max_samples } => RuntimeCommandResult::ImuFifoReadRequested {
                status: unsafe { c_vp_imu_read_fifo_async(max_samples) },
            },
        }
    }
}

fn execute_power_transition(target: PowerState) -> bool {
    let (prepare_status, enter_status) = unsafe {
        match target {
            PowerState::Active => return true,
            PowerState::Suspend => (c_vp_power_prepare_suspend(), c_vp_power_enter_suspend()),
            PowerState::Sleep => (c_vp_power_prepare_sleep(), c_vp_power_enter_sleep()),
        }
    };

    prepare_status == VP_STATUS_OK as u8 && enter_status == VP_STATUS_OK as u8
}

#[allow(non_upper_case_globals)]
fn map_hid_status(status: vp_hid_send_status_t) -> HidSendStatus {
    match status {
        x if x == VP_HID_SEND_SENT as u8 => HidSendStatus::Sent,
        x if x == VP_HID_SEND_RETRY_LATER as u8 => HidSendStatus::RetryLater,
        x if x == VP_HID_SEND_NOT_CONNECTED as u8 => HidSendStatus::NotConnected,
        x if x == VP_HID_SEND_FATAL as u8 => HidSendStatus::Fatal,
        _ => HidSendStatus::Fatal,
    }
}
