#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]

use core::concat;
use core::env;
use core::include;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
