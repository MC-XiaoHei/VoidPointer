#![no_std]

use crate::attitude::get_current_attitude;
use log::LevelFilter::Info;
use log::info;
use utils::logger::init_logger;
use utils::runtime::{RUNTIME, Runtime};

pub mod attitude;
pub mod bindings;
pub mod hid;
pub mod input;
pub mod motion;
pub mod report;
pub mod utils;

#[unsafe(no_mangle)]
pub extern "C" fn init_core() {
    init_logger(Info).expect("Failed to initialize logger");
    RUNTIME.init(Runtime::new());
    info!("Core initialized.");
}

#[unsafe(no_mangle)]
pub extern "C" fn tick() {
    let ret = RUNTIME.execute(Runtime::tick);

    if ret.is_none() {
        log::error!("Call tick() before runtime initialize!");
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    log::error!("{info}");
    loop {}
}
