//! Generated board map types  — clean Rust enum (not bindgen's u32 alias)
//! Source: core/build/board_def.rs + board_gen.rs → OUT_DIR/board_map_rust.rs

use core::concat;
use core::env;
use core::include;

include!(concat!(env!("OUT_DIR"), "/board_map_rust.rs"));
