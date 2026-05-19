/// 通知 cargo 重新检查构建条件
pub fn rerun_if_changed(path: &str) {
    println!("cargo:rerun-if-changed={}", path);
}

/// 通知 cargo 注册条件编译标志
pub fn rustc_check_cfg(cfg: &str) {
    println!("cargo::rustc-check-cfg={}", cfg);
}

/// 打印构建信息
pub fn info(msg: &str) {
    println!("cargo:info={}", msg);
}
