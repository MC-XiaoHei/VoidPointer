//! FFI integration boundary.
//!
//! `platform/Bind/c_api.h` is the single source of truth for Rust→C ABI types,
//! constants, and imported functions. This module exposes the generated
//! `bindgen` output and may contain small idiomatic Rust helpers/wrappers, but
//! it must not duplicate C-style ABI type definitions by hand.

pub mod bindings;

#[inline]
pub fn ffi_bool(value: bindings::vp_bool_t) -> bool {
    value != 0
}

#[inline]
pub fn to_ffi_bool(value: bool) -> bindings::vp_bool_t {
    if value { 1 } else { 0 }
}
