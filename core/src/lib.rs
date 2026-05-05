#![no_std]

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

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    log::error!("panic;info={info}");
    loop {}
}
