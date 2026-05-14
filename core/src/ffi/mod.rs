//! FFI 边界只负责两件事：承认 `c_api.h` 是唯一 ABI 事实来源，并在 Rust 侧提供最薄的一层包装

#![cfg_attr(test, allow(unused_variables))]
#![cfg_attr(test, allow(unused_unsafe))]

pub mod api;
pub mod bindings;
pub mod board_map;

#[cfg(test)]
include!(concat!(env!("OUT_DIR"), "/test_stubs.rs"));

#[inline]
pub fn ffi_bool(value: bindings::vp_bool_t) -> bool {
    value != 0
}

#[inline]
pub fn to_ffi_bool(value: bool) -> bindings::vp_bool_t {
    if value { 1 } else { 0 }
}

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn ffi_bool_true() {
        assert!(ffi_bool(1));
        assert!(ffi_bool(2));
    }

    #[test]
    fn ffi_bool_false() {
        assert!(!ffi_bool(0));
    }

    #[test]
    fn to_ffi_bool_works() {
        assert_eq!(to_ffi_bool(true), 1);
        assert_eq!(to_ffi_bool(false), 0);
    }
}
