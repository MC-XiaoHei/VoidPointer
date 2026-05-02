//! FFI 边界
//!
//! `platform/Bind/c_api.h` 是 ABI 单一事实来源；
//! 本模块只暴露生成的 `bindgen` 结果和少量 Rust 包装

pub mod bindings;

#[inline]
pub fn ffi_bool(value: bindings::vp_bool_t) -> bool {
    value != 0
}

#[inline]
pub fn to_ffi_bool(value: bool) -> bindings::vp_bool_t {
    if value { 1 } else { 0 }
}
