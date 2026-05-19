use core::concat;
use core::env;
use core::include;

include!(concat!(env!("OUT_DIR"), "/board_map_rust.rs"));
