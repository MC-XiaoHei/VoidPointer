#![no_std]

use core::sync::atomic::Ordering;
use log::LevelFilter::Info;
use log::info;
use runtime::{POLL_PENDING, POLL_RUNNING, RUNTIME, Runtime};
use utils::logger::init_logger;

pub mod attitude;
pub mod config;
pub mod ffi;
pub mod hid;
pub mod input;
pub mod motion;
pub mod power;
pub mod report;
pub mod route;
pub mod runtime;
pub mod utils;
pub mod vendor;

#[unsafe(no_mangle)]
pub extern "C" fn vp_core_init() {
    let _ = init_logger(Info);
    RUNTIME.init(Runtime::new());
    POLL_PENDING.store(true, Ordering::Release);
    info!("VoidPointer core initialized.");
}

#[unsafe(no_mangle)]
pub extern "C" fn vp_core_poll() {
    if POLL_RUNNING.load(Ordering::Acquire) {
        POLL_PENDING.store(true, Ordering::Release);
        return;
    }
    POLL_RUNNING.store(true, Ordering::Release);

    let ret = RUNTIME.execute(Runtime::poll);

    if ret.is_none() {
        log::error!("Call vp_core_poll() before vp_core_init()!");
    }

    POLL_RUNNING.store(false, Ordering::Release);

    if POLL_PENDING.load(Ordering::Acquire) {
        Runtime::request_poll();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vp_on_ble_connected(timestamp: u32) {
    if let Some(()) = RUNTIME.execute(|runtime| {
        runtime.router.set_ble_connected(true);
        runtime.mark_activity(timestamp);
    }) {
        Runtime::request_poll();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vp_on_ble_disconnected(_reason: u8, timestamp: u32) {
    if let Some(()) = RUNTIME.execute(|runtime| {
        runtime.router.set_ble_connected(false);
        runtime.mark_activity(timestamp);
    }) {
        Runtime::request_poll();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vp_on_dongle_connected(timestamp: u32) {
    if let Some(()) = RUNTIME.execute(|runtime| {
        runtime.router.set_dongle_connected(true);
        runtime.mark_activity(timestamp);
    }) {
        Runtime::request_poll();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vp_on_dongle_disconnected(_reason: u8, timestamp: u32) {
    if let Some(()) = RUNTIME.execute(|runtime| {
        runtime.router.set_dongle_connected(false);
        runtime.mark_activity(timestamp);
    }) {
        Runtime::request_poll();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vp_on_usb_state_changed(state: ffi::bindings::vp_usb_state_t, timestamp: u32) {
    if let Some(()) = RUNTIME.execute(|runtime| {
        runtime.router.set_usb_state(route::UsbState::from(state));
        runtime.mark_activity(timestamp);
    }) {
        Runtime::request_poll();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vp_on_button_exti(
    _button_id: ffi::bindings::vp_button_id_t,
    _level: ffi::bindings::vp_bool_t,
    timestamp: u32,
) {
    if let Some(()) = RUNTIME.execute(|runtime| {
        runtime.mark_activity(timestamp);
        runtime.pending.events = true;
        runtime.dirty.input = true;
    }) {
        Runtime::request_poll();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vp_on_mode_switch_exti(_level: ffi::bindings::vp_bool_t, timestamp: u32) {
    if let Some(()) = RUNTIME.execute(|runtime| {
        runtime.mark_activity(timestamp);
        runtime.pending.events = true;
        runtime.dirty.input = true;
    }) {
        Runtime::request_poll();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vp_on_debounce_tick(timestamp: u32) {
    if let Some(()) = RUNTIME.execute(|runtime| {
        runtime.mark_activity(timestamp);
        runtime.pending.events = true;
        runtime.dirty.input = true;
    }) {
        Runtime::request_poll();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vp_on_encoder_exti(
    _a_level: ffi::bindings::vp_bool_t,
    _b_level: ffi::bindings::vp_bool_t,
    timestamp: u32,
) {
    if let Some(()) = RUNTIME.execute(|runtime| {
        runtime.mark_activity(timestamp);
        runtime.pending.events = true;
        runtime.dirty.input = true;
    }) {
        Runtime::request_poll();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vp_on_imu_int(timestamp: u32) {
    if let Some(()) = RUNTIME.execute(|runtime| {
        runtime.mark_activity(timestamp);
        runtime.pending.imu_fifo_read = true;
        runtime.dirty.motion = true;
    }) {
        Runtime::request_poll();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vp_on_imu_sample(_raw_x: u16, _raw_y: u16, _raw_z: u16, timestamp: u32) {
    if let Some(()) = RUNTIME.execute(|runtime| {
        runtime.mark_activity(timestamp);
        runtime.dirty.motion = true;
        runtime.dirty.report = true;
    }) {
        Runtime::request_poll();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vp_on_imu_fifo_done(
    _status: ffi::bindings::vp_status_t,
    _dropped_count: u16,
    timestamp: u32,
) {
    if let Some(()) = RUNTIME.execute(|runtime| {
        runtime.mark_activity(timestamp);
        runtime.pending.imu_fifo_read = false;
    }) {
        Runtime::request_poll();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vp_on_hid_send_done(
    _route: ffi::bindings::vp_hid_route_t,
    _status: ffi::bindings::vp_hid_send_status_t,
    timestamp: u32,
) {
    if let Some(()) = RUNTIME.execute(|runtime| {
        runtime.mark_activity(timestamp);
        runtime.pending.hid_retry = true;
    }) {
        Runtime::request_poll();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vp_on_vendor_report_rx(
    _route: ffi::bindings::vp_hid_route_t,
    _ptr: *const u8,
    _len: u16,
    timestamp: u32,
) {
    if let Some(()) = RUNTIME.execute(|runtime| {
        runtime.mark_activity(timestamp);
        runtime.vendor.mark_rx_pending();
        runtime.pending.vendor_rx = true;
    }) {
        Runtime::request_poll();
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    log::error!("{info}");
    loop {}
}
