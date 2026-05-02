use crate::ffi::bindings::c_vp_debug_print;
use core::ffi::c_char;
use core::fmt::Write;
use heapless::String;
use log::{LevelFilter, Log, Metadata, Record, SetLoggerError};

pub struct UartLogger;

static UART_LOGGER: UartLogger = UartLogger;

pub fn print_to_uart(s: &str) {
    // SAFETY:
    // `s` provides a valid pointer to `len` bytes for the duration of the call.
    // `c_vp_debug_print` is expected to read exactly `len` bytes.
    unsafe {
        c_vp_debug_print(s.as_ptr() as *const c_char, s.len() as u16);
    }
}

impl Log for UartLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let mut buf: String<256> = String::new();
        let result = write!(
            buf,
            "[{}] [{}] {}\r\n",
            record.level(),
            record.module_path().unwrap_or("?"),
            record.args()
        );

        if result.is_err() {
            print_to_uart("[ERROR] log message too long\r\n");
        } else {
            print_to_uart(&buf);
        }
    }

    fn flush(&self) {}
}

pub fn init_logger(level: LevelFilter) -> Result<(), SetLoggerError> {
    unsafe {
        log::set_logger_racy(&UART_LOGGER)?;
        log::set_max_level_racy(level);
    }
    Ok(())
}
