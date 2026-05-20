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

#[cfg_attr(coverage, coverage(off))]
pub fn print_to_uart(s: &str) {
    // SAFETY: `s` 在调用期间提供有效的 `len` 字节
    unsafe {
        c_vp_print(s.as_ptr() as *const c_char, s.len() as u16);
    }
}

#[cfg_attr(coverage, coverage(off))]
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

#[cfg_attr(coverage, coverage(off))]
pub fn init_logger(level: LevelFilter) -> Result<(), SetLoggerError> {
    unsafe {
        log::set_logger_racy(&UART_LOGGER)?;
        log::set_max_level_racy(level);
    }
    Ok(())
}

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use super::format_module_path;

    #[test]
    fn none_module_returns_question_mark() {
        assert_eq!(format_module_path(None), "?");
    }

    #[test]
    fn root_crate_module_returns_core() {
        assert_eq!(format_module_path(Some("void_pointer_core")), "core");
    }

    #[test]
    fn nested_module_strips_prefix() {
        assert_eq!(
            format_module_path(Some("void_pointer_core::config::load")),
            "config::load"
        );
    }

    #[test]
    fn module_ends_with_mod() {
        assert_eq!(
            format_module_path(Some("void_pointer_core::config::load::mod")),
            "config::load"
        );
    }
}
