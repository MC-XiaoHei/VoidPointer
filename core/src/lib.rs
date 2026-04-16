#![cfg_attr(not(test), no_std)]

#[allow(dead_code)]
#[allow(non_snake_case)]
#[allow(non_camel_case_types)]
#[allow(non_upper_case_globals)]
pub mod c_bindings {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

mod imu;

#[no_mangle]
pub extern "C" fn my_rust_function(x: i32) -> i32 {
    x * 2
}

#[no_mangle]
pub extern "C" fn my_rust_function2(x: i32) -> bool {
    x > 0
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
