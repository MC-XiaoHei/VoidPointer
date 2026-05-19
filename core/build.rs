#[path = "build/bindgen.rs"]
mod bindgen;
#[path = "build/board_def.rs"]
mod board_def;
#[path = "build/board_gen.rs"]
mod board_gen;
#[path = "build/board_map.rs"]
mod board_map;
#[path = "build/cbindgen.rs"]
mod cbindgen;
#[path = "build/stubs.rs"]
mod stubs;
#[path = "build/util.rs"]
mod util;

use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let project_root = PathBuf::from(&manifest_dir).parent().unwrap().to_path_buf();
    let bind_dir = project_root.join("platform").join("Bind");
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    board_map::generate(&bind_dir, &out_dir);
    bindgen::generate(&bind_dir, &out_dir);
    cbindgen::generate(&bind_dir, &manifest_dir);
}
