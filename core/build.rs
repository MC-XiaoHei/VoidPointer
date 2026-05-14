#[path = "build/board_def.rs"]
mod board_def;
#[path = "build/board_gen.rs"]
mod board_gen;

extern crate bindgen;
extern crate cbindgen;

use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let project_root = PathBuf::from(&manifest_dir).parent().unwrap().to_path_buf();
    let bind_dir = project_root.join("platform").join("Bind");
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    generate_board_map(&bind_dir, &out_dir);
    generate_rust_from_c(&bind_dir, &out_dir);
    generate_c_from_rust(&bind_dir, &manifest_dir);
}

fn generate_board_map(bind_dir: &PathBuf, out_dir: &PathBuf) {
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

    println!("cargo:info=board_map: generated {} signals", board.len());
}

fn generate_rust_from_c(bind_dir: &PathBuf, out_dir: &PathBuf) {
    let header = bind_dir.join("c_api.h");

    println!("cargo::rustc-check-cfg=cfg(coverage)");
    println!("cargo:rerun-if-changed={}", header.display());

    let bindings = bindgen::builder()
        .header(header.to_str().expect("Path error"))
        .use_core()
        .clang_arg("--target=riscv32-unknown-none-elf")
        .clang_arg("-I../platform/HAL/include")
        .clang_arg("-I../platform/StdPeriphDriver/inc")
        .generate()
        .expect("Error while generating bindings.");

    bindings
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("Cannot write bindings.rs");

    generate_test_stubs(out_dir);
}

fn generate_test_stubs(out_dir: &PathBuf) {
    let source = fs::read_to_string(out_dir.join("bindings.rs")).expect("Cannot read bindings.rs");
    let stubs = build_stubs_from(&source);
    fs::write(out_dir.join("test_stubs.rs"), &stubs).expect("Cannot write test_stubs.rs");
    println!(
        "cargo:info=generated {} test stubs",
        stubs.matches("#[unsafe(no_mangle)]").count()
    );
}

fn build_stubs_from(source: &str) -> String {
    let mut stubs = String::new();
    stubs.push_str("// 自动生成，勿手动编辑\n");
    stubs.push_str("use crate::ffi::bindings::*;\n");
    stubs.push_str("#[allow(unused_variables)]\n");
    stubs.push_str("#[allow(unused_unsafe)]\n\n");

    let mut pending: Option<String> = None;

    for line in source.lines() {
        let trimmed = line.trim();
        pending = match pending {
            None if trimmed.starts_with("pub fn") => Some(trimmed.to_string()),
            _ => pending,
        };

        let Some(ref mut sig) = pending else { continue };
        if !trimmed.ends_with(';') {
            sig.push_str(trimmed);
            continue;
        }

        let sig = pending.take().unwrap();
        let body = stub_body_from(&sig);
        stubs.push_str(&body);
    }

    stubs
}

fn stub_body_from(sig: &str) -> String {
    let sig = sig.strip_suffix(';').unwrap_or(sig);
    let sig = sig.strip_prefix("pub fn ").unwrap_or(sig);

    let paren = match (sig.find('('), sig.rfind(')')) {
        (Some(o), Some(c)) => (o, c),
        _ => return String::new(),
    };

    let fn_name = &sig[..paren.0];
    if !fn_name.starts_with("c_") {
        return String::new();
    }

    let params = &sig[paren.0 + 1..paren.1];

    let ret_part = sig[paren.1 + 1..].trim();
    let ret_type = ret_part
        .strip_prefix("-> ")
        .map(|s| s.trim().trim_end_matches(';'))
        .unwrap_or("");

    let mut out = format!(
        "#[unsafe(no_mangle)]\npub unsafe extern \"C\" fn {}({})",
        fn_name, params
    );

    match ret_type {
        "" => out.push_str(" {}\n\n"),
        "u8" | "vp_status_t" | "vp_bool_t" | "vp_hid_send_status_t" => {
            out.push_str(" -> u8 { 0 }\n\n")
        }
        "u32" | "vp_timestamp_t" | "vp_wake_source_t" | "vp_usb_state_t" => {
            out.push_str(" -> u32 { 0 }\n\n")
        }
        _ => out.push_str(&format!(" -> {} {{ 0 }}\n\n", ret_type)),
    }

    out
}

fn generate_c_from_rust(bind_dir: &PathBuf, manifest_dir: &str) {
    println!("cargo:rerun-if-changed=src");
    let mut builder = cbindgen::Builder::new()
        .with_crate(manifest_dir)
        .with_language(cbindgen::Language::C)
        .with_after_include("#include \"c_api.h\"");

    for name in [
        "vp_timestamp_t",
        "vp_bool_t",
        "vp_status_t",
        "vp_button_id_t",
        "vp_input_id_t",
        "vp_output_id_t",
        "vp_exti_edge_t",
        "vp_hid_route_t",
        "vp_hid_send_status_t",
        "vp_usb_state_t",
        "vp_wake_source_t",
        "VP_WAKE_SOURCE_BUTTON",
        "VP_WAKE_SOURCE_ENCODER",
        "VP_WAKE_SOURCE_IMU",
        "VP_WAKE_SOURCE_USB",
        "AxisDir",
        "AxisMap",
        "SourceAxis",
    ] {
        builder = builder.exclude_item(name);
    }

    builder
        .generate()
        .expect("Unable to generate C header")
        .write_to_file(&bind_dir.join("rust_api.h"));
}
