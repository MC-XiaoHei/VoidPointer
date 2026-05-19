use crate::util::info;
use std::fs;
use std::path::PathBuf;

/// 从 bindgen 生成的 bindings 生成 test stubs
pub fn from_bindings(out_dir: &PathBuf) {
    let source = fs::read_to_string(out_dir.join("bindings.rs")).expect("Cannot read bindings.rs");
    let stubs = build_stubs_from(&source);
    fs::write(out_dir.join("test_stubs.rs"), &stubs).expect("Cannot write test_stubs.rs");
    info(&format!(
        "generated {} test stubs",
        stubs.matches("#[unsafe(no_mangle)]").count()
    ));
}

fn build_stubs_from(source: &str) -> String {
    let mut stubs = String::new();
    stubs.push_str("// 自动生成，勿手动编辑\n");
    stubs.push_str("use crate::ffi::bindings::*;\n\n");

    let lines: Vec<&str> = source.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let trimmed = lines[i].trim();
        i += 1;

        if !trimmed.starts_with("pub fn") {
            continue;
        }

        let mut sig = trimmed.to_string();
        if trimmed.ends_with(';') {
            stubs.push_str(&stub_body_from(&sig));
            continue;
        }

        while i < lines.len() {
            let t = lines[i].trim();
            i += 1;
            sig.push(' ');
            sig.push_str(t);
            if t.ends_with(';') {
                stubs.push_str(&stub_body_from(&sig));
                break;
            }
        }
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
