#![no_std]

use crate::attitude::get_current_attitude;
use crate::logger::init_logger;
use crate::runtime::{RUNTIME, Runtime};
use log::LevelFilter::Info;
use log::info;

pub mod attitude;
pub mod bindings;
pub mod hid;
pub mod logger;
pub mod motion;
pub mod report;
pub mod runtime;

#[unsafe(no_mangle)]
pub extern "C" fn init_core() {
    init_logger(Info).expect("Failed to initialize logger");
    unsafe {
        RUNTIME = Some(Runtime::new());
    }
    info!("Core initialized.");
}

#[unsafe(no_mangle)]
pub extern "C" fn tick() {
    unsafe {
        let rt_ptr = core::ptr::addr_of_mut!(RUNTIME);

        if let Some(runtime) = (*rt_ptr).as_mut() {
            runtime.tick();
        } else {
            log::error!("Call tick() before runtime initialize!");
        }
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    log::error!("{info}");
    loop {}
}
