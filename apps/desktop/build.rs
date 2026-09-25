use std::{env, fs, path::PathBuf};

use sha2::{Digest, Sha256};

const REQUIRED_FILES: &[&str] = &["QingLinkService.exe"];

fn find_asset(name: &str) -> Option<PathBuf> {
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let asset_dir = manifest_dir.join("assets/input");
    let path = asset_dir.join(name);
    if path.is_file() {
        return Some(path);
    }
    if let Ok(target_dir) = env::var("QINGLINK_INPUT_SERVICE_PATH") {
        let path = PathBuf::from(&target_dir).join(name);
        if path.is_file() {
            return Some(path);
        }
    }
    let target_dir = env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest_dir.join("../../../target"));
    for profile in &["release", "debug"] {
        let path = target_dir.join(profile).join(name);
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

fn main() {
    println!("cargo:rerun-if-env-changed=QINGLINK_REQUIRE_INPUT_ASSETS");
    println!("cargo:rerun-if-env-changed=QINGLINK_INPUT_SERVICE_PATH");
    println!("cargo:rerun-if-changed=assets/input");
    println!("cargo:rerun-if-changed=qinglink.rc");
    #[cfg(windows)]
    embed_resource::compile("qinglink.rc", embed_resource::NONE)
        .manifest_optional()
        .unwrap();

    let missing: Vec<_> = REQUIRED_FILES
        .iter()
        .filter(|name| find_asset(name).is_none())
        .copied()
        .collect();
    if !missing.is_empty()
        && env::var_os("QINGLINK_REQUIRE_INPUT_ASSETS").is_some_and(|value| value == "1")
    {
        panic!(
            "secure-input release assets are missing: {}",
            missing.join(", ")
        );
    }

    let output =
        PathBuf::from(env::var_os("OUT_DIR").expect("out dir")).join("embedded_input_assets.rs");
    let mut generated = String::from("const INPUT_ASSETS: &[EmbeddedAsset] = &[\n");
    if missing.is_empty() {
        for name in REQUIRED_FILES {
            let path = find_asset(name).expect("asset file exists");
            let bytes = fs::read(&path).expect("read secure-input asset");
            let hash = format!("{:x}", Sha256::digest(&bytes));
            generated.push_str(&format!(
                "    EmbeddedAsset {{ relative_path: {name:?}, bytes: include_bytes!({path:?}), sha256: {hash:?} }},\n",
                path = path.to_string_lossy()
            ));
        }
    }
    generated.push_str("];\n");
    fs::write(output, generated).expect("write generated secure-input assets");
}
