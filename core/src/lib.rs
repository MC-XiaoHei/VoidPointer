#![no_std]
#![cfg_attr(coverage, feature(coverage_attribute))]

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

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    log::error!("panic;info={info}");
    loop {}
}
