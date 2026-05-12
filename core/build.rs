extern crate bindgen;
extern crate cbindgen;

use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let bind_dir = PathBuf::from(&crate_dir)
        .parent()
        .expect("parent directory not found")
        .join("platform")
        .join("Bind");
    let c_api_path = bind_dir.join("c_api.h");
    let rust_api_path = bind_dir.join("rust_api.h");

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    println!("cargo:rerun-if-changed={}", c_api_path.display());

    let bindings = bindgen::builder()
        .header(c_api_path.to_str().expect("Path error"))
        .use_core()
        .clang_arg("--target=riscv32-unknown-none-elf")
        .clang_arg("-I../platform/HAL/include")
        .clang_arg("-I../platform/StdPeriphDriver/inc")
        .generate()
        .expect("Error while generating bindings.");

    bindings
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("Cannot write bindings.rs");

    generate_test_stubs(&out_dir);

    println!("cargo:rerun-if-changed=src");
    let mut cbindgen_builder = cbindgen::Builder::new()
        .with_crate(&crate_dir)
        .with_language(cbindgen::Language::C)
        .with_after_include("#include \"c_api.h\"");

    for item in [
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
        cbindgen_builder = cbindgen_builder.exclude_item(item);
    }

    cbindgen_builder
        .generate()
        .expect("Unable to generate C header")
        .write_to_file(&rust_api_path);
}

fn generate_test_stubs(out_dir: &PathBuf) {
    let bindings_path = out_dir.join("bindings.rs");
    let content = fs::read_to_string(&bindings_path).expect("Cannot read bindings.rs");

    let mut stubs = String::new();
    stubs.push_str("// 自动生成，勿手动编辑\n");
    stubs.push_str("use crate::ffi::bindings::*;\n");
    stubs.push_str("#[allow(unused_variables)]\n");
    stubs.push_str("#[allow(unused_unsafe)]\n\n");

    let mut pending_sig: Option<String> = None;

    for line in content.lines() {
        let trimmed = line.trim();

        if let Some(ref mut sig) = pending_sig {
            sig.push_str(trimmed);
            if !trimmed.ends_with(';') {
                continue;
            }
        } else if trimmed.starts_with("pub fn") {
            if trimmed.ends_with(';') {
                pending_sig = Some(trimmed.to_string());
            } else {
                pending_sig = Some(trimmed.to_string());
                continue;
            }
        } else {
            continue;
        }

        let full_sig = pending_sig.take().unwrap();
        let sig = full_sig.strip_suffix(';').unwrap_or(&full_sig);
        let sig = sig.strip_prefix("pub fn ").unwrap_or(&sig);

        let paren_open = sig.find('(');
        let paren_close = sig.rfind(')');
        let Some((open, close)) = paren_open.zip(paren_close) else {
            continue;
        };

        let fn_name = &sig[..open];

        if !fn_name.starts_with("c_") {
            continue;
        }

        let params = &sig[open + 1..close];
        let ret_part = sig[close + 1..].trim();

        let ret_type = if ret_part.starts_with("-> ") {
            ret_part[3..].trim().trim_end_matches(';')
        } else {
            ""
        };

        stubs.push_str("#[unsafe(no_mangle)]\n");
        stubs.push_str(&format!(
            "pub unsafe extern \"C\" fn {}({})",
            fn_name, params
        ));
        match ret_type {
            "" => stubs.push_str(" {}\n\n"),
            "u8" | "vp_status_t" | "vp_bool_t" | "vp_hid_send_status_t" => {
                stubs.push_str(" -> u8 { 0 }\n\n")
            }
            "u32" | "vp_timestamp_t" | "vp_wake_source_t" | "vp_usb_state_t" => {
                stubs.push_str(" -> u32 { 0 }\n\n")
            }
            _ => stubs.push_str(&format!(" -> {} {{ 0 }}\n\n", ret_type)),
        }
    }

    fs::write(out_dir.join("test_stubs.rs"), &stubs).expect("Cannot write test_stubs.rs");
    let count = stubs.matches("#[unsafe(no_mangle)]").count();
    println!("cargo:info=generated {} test stubs", count);
}
