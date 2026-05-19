use crate::util;
use std::path::PathBuf;

extern crate cbindgen;

const CAPI_EXCLUDE: &[&str] = &[
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
];

pub fn generate(bind_dir: &PathBuf, manifest_dir: &str) {
    util::rerun_if_changed("src");

    let mut builder = cbindgen::Builder::new()
        .with_crate(manifest_dir)
        .with_language(cbindgen::Language::C)
        .with_after_include("#include \"c_api.h\"");

    for name in CAPI_EXCLUDE {
        builder = builder.exclude_item(name);
    }

    builder
        .generate()
        .expect("Unable to generate C header")
        .write_to_file(&bind_dir.join("rust_api.h"));
}
