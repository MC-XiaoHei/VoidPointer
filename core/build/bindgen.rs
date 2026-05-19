use crate::stubs;
use crate::util;
use bindgen;
use std::path::PathBuf;

pub fn generate(bind_dir: &PathBuf, out_dir: &PathBuf) {
    let header = bind_dir.join("c_api.h");

    util::rustc_check_cfg("cfg(coverage)");
    util::rerun_if_changed(&header.display().to_string());
    util::rerun_if_changed("build.rs");

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

    stubs::from_bindings(out_dir);
}
