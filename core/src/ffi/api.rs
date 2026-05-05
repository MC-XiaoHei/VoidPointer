use crate::runtime::events::RuntimeEvent;
use crate::runtime::{EVENT_QUEUE, EVENTS_PENDING, POLL_PENDING, POLL_RUNNING, RUNTIME, Runtime};
use crate::utils::logger::init_logger;
use crate::vendor::VENDOR_RX_QUEUE;
use core::sync::atomic::Ordering;
use log::LevelFilter::Debug;

fn enqueue_runtime_event(event: RuntimeEvent) {
    let _ = EVENT_QUEUE.push(event);
    EVENTS_PENDING.store(true, Ordering::Release);
    POLL_PENDING.store(true, Ordering::Release);
    Runtime::request_poll();
}

#[unsafe(no_mangle)]
pub extern "C" fn vp_core_init() {
    let _ = init_logger(Debug);
    RUNTIME.init(Runtime::new());
    POLL_PENDING.store(true, Ordering::Release);
    log::info!("core initialized");
}

#[unsafe(no_mangle)]
pub extern "C" fn vp_input_enable() {
    let ret = RUNTIME.execute(Runtime::enable_input_interrupts);
    if ret.is_none() {
        log::error!("api misuse;func=vp_input_enable,reason=before_init");
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vp_core_poll() {
    if POLL_RUNNING.load(Ordering::Acquire) {
        POLL_PENDING.store(true, Ordering::Release);
        return;
    }
    POLL_RUNNING.store(true, Ordering::Release);

    let ret = RUNTIME.execute(Runtime::poll);

    match ret {
        Some(Some(command)) => {
            POLL_RUNNING.store(false, Ordering::Release);
            let result = command.execute();
            POLL_RUNNING.store(true, Ordering::Release);
            let _ = RUNTIME.execute(|runtime| runtime.apply_command_result(result));
            POLL_RUNNING.store(false, Ordering::Release);
        }
        Some(None) => {
            POLL_RUNNING.store(false, Ordering::Release);
        }
        None => {
            log::error!("api misuse;func=vp_core_poll,reason=before_init");
            POLL_RUNNING.store(false, Ordering::Release);
        }
    }

    if POLL_PENDING.load(Ordering::Acquire) {
        Runtime::request_poll();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vp_on_ble_connected(timestamp: u32) {
    enqueue_runtime_event(RuntimeEvent::BleConnected { timestamp });
}

#[unsafe(no_mangle)]
pub extern "C" fn vp_on_ble_input_ready(timestamp: u32) {
    enqueue_runtime_event(RuntimeEvent::BleInputReady { timestamp });
}

#[unsafe(no_mangle)]
pub extern "C" fn vp_on_ble_disconnected(reason: u8, timestamp: u32) {
    enqueue_runtime_event(RuntimeEvent::BleDisconnected { reason, timestamp });
}

#[unsafe(no_mangle)]
pub extern "C" fn vp_on_dongle_connected(timestamp: u32) {
    enqueue_runtime_event(RuntimeEvent::DongleConnected { timestamp });
}

#[unsafe(no_mangle)]
pub extern "C" fn vp_on_dongle_disconnected(reason: u8, timestamp: u32) {
    enqueue_runtime_event(RuntimeEvent::DongleDisconnected { reason, timestamp });
}

#[unsafe(no_mangle)]
pub extern "C" fn vp_on_usb_state_changed(
    state: crate::ffi::bindings::vp_usb_state_t,
    timestamp: u32,
) {
    enqueue_runtime_event(RuntimeEvent::UsbStateChanged { state, timestamp });
}

#[unsafe(no_mangle)]
pub extern "C" fn vp_on_button_exti(
    button_id: crate::ffi::bindings::vp_button_id_t,
    level: crate::ffi::bindings::vp_bool_t,
    timestamp: u32,
) {
    enqueue_runtime_event(RuntimeEvent::ButtonExti {
        button_id,
        level,
        timestamp,
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn vp_on_mode_switch_exti(level: crate::ffi::bindings::vp_bool_t, timestamp: u32) {
    enqueue_runtime_event(RuntimeEvent::ModeSwitchExti { level, timestamp });
}

#[unsafe(no_mangle)]
pub extern "C" fn vp_on_debounce_tick(timestamp: u32) {
    enqueue_runtime_event(RuntimeEvent::DebounceTick { timestamp });
}

#[unsafe(no_mangle)]
pub extern "C" fn vp_on_encoder_exti(
    a_level: crate::ffi::bindings::vp_bool_t,
    b_level: crate::ffi::bindings::vp_bool_t,
    timestamp: u32,
) {
    enqueue_runtime_event(RuntimeEvent::EncoderExti {
        a_level,
        b_level,
        timestamp,
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn vp_on_imu_int(timestamp: u32) {
    enqueue_runtime_event(RuntimeEvent::ImuInt { timestamp });
}

#[unsafe(no_mangle)]
pub extern "C" fn vp_on_imu_sample(raw_x: u16, raw_y: u16, raw_z: u16, timestamp: u32) {
    enqueue_runtime_event(RuntimeEvent::ImuSample {
        raw_x,
        raw_y,
        raw_z,
        timestamp,
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn vp_on_imu_fifo_done(
    status: crate::ffi::bindings::vp_status_t,
    dropped_count: u16,
    timestamp: u32,
) {
    enqueue_runtime_event(RuntimeEvent::ImuFifoDone {
        status,
        dropped_count,
        timestamp,
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn vp_on_hid_send_done(
    route: crate::ffi::bindings::vp_hid_route_t,
    status: crate::ffi::bindings::vp_hid_send_status_t,
    timestamp: u32,
) {
    enqueue_runtime_event(RuntimeEvent::HidSendDone {
        route,
        status,
        timestamp,
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn vp_on_vendor_report_rx(
    route: crate::ffi::bindings::vp_hid_route_t,
    ptr: *const u8,
    len: u16,
    timestamp: u32,
) {
    let copied = unsafe { VENDOR_RX_QUEUE.copy_from_ptr(route, ptr, len, timestamp) };
    if copied {
        enqueue_runtime_event(RuntimeEvent::VendorReportRx {
            route,
            len,
            timestamp,
        });
    }
}
