extern crate bindgen;
extern crate cbindgen;

use std::env;
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

    println!("cargo:rerun-if-changed={}", c_api_path.display());

    let bindings = bindgen::builder()
        .header(c_api_path.to_str().expect("Path error"))
        .use_core()
        .clang_arg("--target=riscv32-unknown-none-elf")
        .clang_arg("-I../platform/HAL/include")
        .clang_arg("-I../platform/StdPeriphDriver/inc")
        .generate()
        .expect("Error while generating bindings.");

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("Cannot write bindings.rs");

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
