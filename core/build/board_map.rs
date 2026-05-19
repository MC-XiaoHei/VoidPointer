use crate::board_def;
use crate::board_gen;
use crate::util;
use std::fs;
use std::path::PathBuf;

pub fn generate(bind_dir: &PathBuf, out_dir: &PathBuf) {
    let board = board_def::voidpointer_board();

    fs::write(
        bind_dir.join("board_map.h"),
        board_gen::generate_c_header(&board),
    )
    .expect("Cannot write board_map.h");

    fs::write(
        bind_dir.join("board_map_gen.c"),
        board_gen::generate_c_source(&board),
    )
    .expect("Cannot write board_map_gen.c");

    fs::write(
        out_dir.join("board_map_rust.rs"),
        board_gen::generate_rust_bindings(&board),
    )
    .expect("Cannot write board_map_rust.rs");

    util::info(&format!("board_map: generated {} signals", board.len()));
}
