use crate::ffi::bindings::c_vp_print;
use core::ffi::c_char;
use core::fmt::Write;
use heapless::String;
use log::{LevelFilter, Log, Metadata, Record, SetLoggerError};

const CRATE_MODULE_PREFIX: &str = "void_pointer_core::";
const CRATE_MODULE_NAME: &str = "void_pointer_core";

pub struct UartLogger;

static UART_LOGGER: UartLogger = UartLogger;

fn format_module_path(module_path: Option<&str>) -> &str {
    let Some(module_path) = module_path else {
        return "?";
    };

    if module_path == CRATE_MODULE_NAME {
        return "core";
    }

    let shortened = module_path
        .strip_prefix(CRATE_MODULE_PREFIX)
        .unwrap_or(module_path);

    shortened.strip_suffix("::mod").unwrap_or(shortened)
}

pub fn print_to_uart(s: &str) {
    // SAFETY: `s` 在调用期间提供有效的 `len` 字节
    unsafe {
        c_vp_print(s.as_ptr() as *const c_char, s.len() as u16);
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
        let module_path = format_module_path(record.module_path());
        let result = write!(
            buf,
            "[{}] [{}] {}\r\n",
            record.level(),
            module_path,
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
