//! FFI 边界只负责两件事：承认 `c_api.h` 是唯一 ABI 事实来源，并在 Rust 侧提供最薄的一层包装

pub mod api;
pub mod bindings;

// 自动生成的 C 函数测试 stub，由 build.rs 从 c_api.h 生成
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
