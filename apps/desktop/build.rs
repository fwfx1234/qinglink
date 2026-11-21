//! 桌面端伴随程序的构建脚本。
//!
//! 生成两个产物：
//!
//! * `embedded_input_assets.rs` — 安全输入服务的二进制文件，内嵌后安装服务不再
//!   依赖旁挂文件；
//! * `web_assets.rs` — 已构建的管理界面，内嵌后复制出的可执行文件即可提供自己的
//!   界面。
//!
//! 前端只在缺失或过期时重新构建；缺少 Node 工具链时降级为说明性占位内容，而不是让
//! 构建失败。

use std::{
    env, fs,
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};

const REQUIRED_FILES: &[&str] = &["QingLinkService.exe"];

/// Frontend project directory, relative to this crate.
const WEB_PROJECT: &str = "web-ui";

/// Vite build output inside the frontend project.
const WEB_DIST: &str = "dist";

/// Inputs whose modification should trigger a frontend rebuild.
const WEB_TRACKED_INPUTS: &[&str] = &[
    "package.json",
    "package-lock.json",
    "index.html",
    "vite.config.ts",
    "tsconfig.json",
    "tsconfig.app.json",
    "tsconfig.node.json",
];

/// Sources whose modification makes an embedded service binary outdated.
///
/// The service binary is not built by this crate, so nothing else can notice
/// that `assets/input/QingLinkService.exe` predates the management protocol the
/// desktop speaks. Shipping a stale asset produces a running service that the
/// interface reports as "未连接", which is expensive to diagnose from outside.
///
/// `crates/qinglink-windows` is part of that surface even though it is not the
/// protocol: the icon pipeline the management interface (and the phone) depends
/// on lives there, so a service binary older than it silently resolves no icons.
const SERVICE_ASSET_SOURCES: &[&str] = &[
    "../../services/qinglink-service/src",
    "../../services/qinglink-service/Cargo.toml",
    "../../crates/qinglink-protocol/src",
    "../../crates/qinglink-windows/src",
    "../../crates/qinglink-windows/Cargo.toml",
];

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
    // 优先当前 profile：固定 release 优先会让 debug 桌面端在两种产物都存在时静默内嵌
    // 上一次的 release 服务（"改了 services/ 却没生效"的常见来源）。
    let current_profile = env::var("PROFILE").unwrap_or_default();
    let profiles: [&str; 2] = if current_profile == "release" {
        ["release", "debug"]
    } else {
        ["debug", "release"]
    };
    for profile in profiles {
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
    println!("cargo:rerun-if-changed=assets/tray-icon.svg");
    println!("cargo:rerun-if-changed=qinglink.rc");
    println!("cargo:rerun-if-changed=qinglink.exe.manifest");
    // 构建号一变，嵌进可执行文件的版本资源也要跟着变（见 embed_version_resource）。
    println!("cargo:rerun-if-changed=../../crates/qinglink-version/build-number.txt");
    for entry in SERVICE_ASSET_SOURCES {
        println!("cargo:rerun-if-changed={entry}");
    }
    #[cfg(windows)]
    // .rc 里嵌入了 `RT_MANIFEST`（见 qinglink.rc / qinglink.exe.manifest）：
    // 应用清单在这里嵌入，失败会丢掉 PerMonitorV2 DPI 感知、Common-Controls
    // 依赖和执行级别声明。
    embed_resource::compile("qinglink.rc", embed_resource::NONE)
        .manifest_optional()
        .unwrap_or_else(|error| panic!("cannot embed qinglink.rc: {error}"));
    embed_version_resource();

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

    // A service asset older than the service sources is the signature of a
    // service that cannot answer the desktop's management protocol. Warn (or
    // fail the release build) instead of silently embedding it again.
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest dir"));
    if missing.is_empty()
        && let Some(asset) = find_asset(REQUIRED_FILES[0])
        && service_asset_is_stale(&manifest_dir, &asset)
    {
        let message = format!(
            "secure-input asset {} is older than the service sources; \
             rebuild it with `cargo build -p qinglink-service --bin QingLinkService` \
             and copy the executable to apps/desktop/assets/input/",
            asset.display()
        );
        if env::var_os("QINGLINK_REQUIRE_INPUT_ASSETS").is_some_and(|value| value == "1") {
            panic!("{message}");
        }
        println!("cargo:warning={message}");
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

    build_web_assets();
}

/// 生成并嵌入随构建号变化的版本资源。
///
/// `qinglink.rc` 只保留应用清单：版本信息里的构建号每次源码改动都会变，写死在
/// 仓库里必然与界面显示的 `0.1.0+13` 脱节，因此按当前版本与构建号现生成一份。
fn embed_version_resource() {
    let script = qinglink_version::version_info_rc("QingLink Desktop", "QingLink.exe");
    let path = PathBuf::from(env::var_os("OUT_DIR").expect("out dir")).join("version_info.rc");
    fs::write(&path, script).expect("write generated version resource");

    #[cfg(windows)]
    embed_resource::compile(&path, embed_resource::NONE)
        .manifest_optional()
        .unwrap_or_else(|error| panic!("cannot embed version_info.rc: {error}"));
}

/// Embed the built single-page frontend into the binary.
///
/// A double-clicked executable must serve its own interface, so the built
/// assets are compiled in rather than read from disk at runtime.
///
/// The frontend is rebuilt only when it is missing or stale. A missing Node
/// toolchain does **not** fail the build: an explanatory placeholder is
/// embedded instead, with a `cargo:warning`, so the backend remains buildable
/// and testable on machines without Node.
fn build_web_assets() {
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let project = manifest_dir.join(WEB_PROJECT);
    let dist = project.join(WEB_DIST);

    println!("cargo:rerun-if-env-changed=QINGLINK_BUILD_WEB");
    println!("cargo:rerun-if-env-changed=QINGLINK_SKIP_WEB_BUILD");
    println!("cargo:rerun-if-changed={WEB_PROJECT}/src");
    for input in WEB_TRACKED_INPUTS {
        println!("cargo:rerun-if-changed={WEB_PROJECT}/{input}");
    }

    let stale = frontend_is_stale(&project, &dist);
    if stale
        && env::var_os("QINGLINK_SKIP_WEB_BUILD").is_none_or(|value| value != "1")
        && let Err(error) = build_frontend(&project)
    {
        println!(
            "cargo:warning=cannot build the web interface ({error}); \
             embedding a placeholder page instead"
        );
    }

    // The embedded shell is only as fresh as the last build that reached `dist`.
    // A skipped build (`QINGLINK_SKIP_WEB_BUILD=1`) or a failed `npm run build`
    // leaves the previous output in place, and the executable then serves an
    // interface older than its own sources. `IS_PLACEHOLDER` only reports
    // *whether* a shell exists, so this state is otherwise invisible.
    if frontend_output_predates_sources(&project, &dist) {
        println!(
            "cargo:warning=the embedded web interface is older than the sources in \
             apps/desktop/web-ui; rebuild it with `cd apps/desktop/web-ui && npm run build`, \
             or unset QINGLINK_SKIP_WEB_BUILD"
        );
    }

    let generated = render_asset_module(&dist);
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("out dir")).join("web_assets.rs");
    fs::write(output, generated).expect("write generated web assets");
}

/// Whether the frontend build output is missing or older than its inputs.
fn frontend_is_stale(project: &Path, dist: &Path) -> bool {
    if env::var_os("QINGLINK_BUILD_WEB").is_some_and(|value| value == "1") {
        return true;
    }
    if fs::metadata(dist.join("index.html")).is_err() {
        return true;
    }

    frontend_output_predates_sources(project, dist)
}

/// Whether `dist/index.html` is older than the frontend sources behind it.
///
/// Deliberately ignores `QINGLINK_BUILD_WEB`: it answers a different question
/// than [`frontend_is_stale`] — not "should we rebuild", but "is what we are
/// about to embed older than the sources it came from". A missing
/// `dist/index.html` does not count as predating; that is the placeholder path,
/// reported separately.
fn frontend_output_predates_sources(project: &Path, dist: &Path) -> bool {
    let Ok(index_modified) =
        fs::metadata(dist.join("index.html")).and_then(|metadata| metadata.modified())
    else {
        return false;
    };

    let mut candidates: Vec<PathBuf> = WEB_TRACKED_INPUTS
        .iter()
        .map(|name| project.join(name))
        .filter(|path| path.is_file())
        .collect();
    collect_files(&project.join("src"), &mut candidates);

    candidates
        .into_iter()
        .filter_map(|path| fs::metadata(&path).ok()?.modified().ok())
        .any(|modified| modified > index_modified)
}

/// Whether the embedded service binary predates the service sources.
///
/// A heuristic, but the failure it catches (a stale service that cannot answer
/// the management protocol) is otherwise invisible until runtime.
fn service_asset_is_stale(manifest_dir: &Path, asset: &Path) -> bool {
    let Ok(asset_modified) = fs::metadata(asset).and_then(|metadata| metadata.modified()) else {
        return false;
    };
    let mut sources: Vec<PathBuf> = Vec::new();
    for entry in SERVICE_ASSET_SOURCES {
        let path = manifest_dir.join(entry);
        if path.is_dir() {
            collect_files(&path, &mut sources);
        } else if path.is_file() {
            sources.push(path);
        }
    }
    sources
        .into_iter()
        .filter_map(|path| fs::metadata(&path).ok()?.modified().ok())
        .any(|modified| modified > asset_modified)
}

/// Collect every regular file below `directory`, ignoring build output.
fn collect_files(directory: &std::path::Path, output: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        if matches!(name.to_str(), Some("node_modules" | "dist" | ".vite")) {
            continue;
        }
        if path.is_dir() {
            collect_files(&path, output);
        } else {
            output.push(path);
        }
    }
}

/// Run the frontend build through `cmd`, so `npm.cmd` resolves on Windows.
///
/// `npm run build` (Vite) is the standard path. On failure it retries with
/// `npm run build:fallback`, a subprocess-free Rollup build, because restricted
/// environments can block the child process esbuild needs. Both produce the same
/// `dist/` shape, so nothing downstream cares which one ran.
fn build_frontend(project: &PathBuf) -> Result<(), String> {
    if !project.join("package.json").is_file() {
        return Err(format!("{} has no package.json", project.display()));
    }
    if !project.join("node_modules").is_dir() {
        run_npm(project, &["install", "--no-audit", "--no-fund"])?;
    }
    match run_npm(project, &["run", "build"]) {
        Ok(()) => Ok(()),
        Err(primary) => {
            eprintln!(
                "cargo:warning=the Vite build failed, retrying with the subprocess-free build"
            );
            run_npm(project, &["run", "build:fallback"]).map_err(|fallback| {
                format!("both frontend builds failed.\nVite: {primary}\nfallback: {fallback}")
            })
        }
    }
}

fn run_npm(project: &PathBuf, arguments: &[&str]) -> Result<(), String> {
    let mut command = std::process::Command::new("cmd");
    command
        .arg("/c")
        .arg("npm")
        .args(arguments)
        .current_dir(project);
    let output = command
        .output()
        .map_err(|error| format!("cannot run `npm {}`: {error}", arguments.join(" ")))?;
    if output.status.success() {
        return Ok(());
    }
    Err(format!(
        "`npm {}` failed: {}{}",
        arguments.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ))
}

/// Generate the Rust module that embeds every file under `dist`.
fn render_asset_module(dist: &std::path::Path) -> String {
    let mut files = Vec::new();
    collect_files(dist, &mut files);
    files.sort();

    let mut module = String::from(
        "// @generated by build.rs from apps/desktop/web-ui/dist. Do not edit.\n\
         //\n\
         // Every frontend file is embedded so a copied executable serves its own\n\
         // interface without shipping a sidecar directory.\n\n",
    );

    // The SPA entry point, used for deep links as well.
    let has_index = files.iter().any(|path| path.ends_with("index.html"));
    match has_index {
        true => {
            let index = files
                .iter()
                .find(|path| path.ends_with("index.html"))
                .expect("index.html is present");
            module.push_str(&format!(
                "/// The single-page application shell.\npub const INDEX_HTML: &[u8] = include_bytes!({:?});\n\n",
                index.to_string_lossy()
            ));
        }
        false => module.push_str(
            "/// Placeholder served when the frontend was never built.\npub const INDEX_HTML: &[u8] = PLACEHOLDER.as_bytes();\n\n",
        ),
    }

    module.push_str("/// Number of embedded asset files.\n");
    module.push_str(&format!(
        "pub const ASSET_COUNT: usize = {};\n\n",
        files.len()
    ));
    module.push_str("/// Whether the embedded shell is the build-time placeholder.\n");
    module.push_str(&format!(
        "pub const IS_PLACEHOLDER: bool = {};\n\n",
        !has_index
    ));

    module.push_str(
        "/// Look up an embedded asset by its URL path.\npub fn lookup(path: &str) -> Option<(&'static str, &'static [u8])> {\n    match path {\n",
    );
    for file in &files {
        let Ok(relative) = file.strip_prefix(dist) else {
            continue;
        };
        let url = format!("/{}", relative.to_string_lossy().replace('\\', "/"));
        let content_type = content_type_for(&url);
        module.push_str(&format!(
            "        {url:?} => Some(({content_type:?}, include_bytes!({:?}))),\n",
            file.to_string_lossy()
        ));
    }
    module.push_str("        _ => None,\n    }\n}\n");

    if !has_index {
        module.push_str("\nconst PLACEHOLDER: &str = r#\"");
        module.push_str(PLACEHOLDER_HTML);
        module.push_str("\"#;\n");
    }

    module
}

/// Instruct the browser how to decode an embedded file.
fn content_type_for(url: &str) -> &'static str {
    match url.rsplit('.').next().unwrap_or_default() {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" | "map" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "ttf" => "font/ttf",
        "txt" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

/// Shown when the frontend could not be built. Kept deliberately plain and
/// dependency-free, because it is served precisely when nothing else works.
const PLACEHOLDER_HTML: &str = r#"<!DOCTYPE html>
<html lang="zh-CN"><head><meta charset="utf-8"><title>QingLink</title></head>
<body style="font-family:system-ui,sans-serif;background:#EEF1F6;color:#101725;padding:3rem">
<h1>界面资源未构建</h1>
<p>桌面端正在运行，Web 管理界面尚未构建。</p>
<pre style="background:#F4F7FB;border:1px solid #DDE4EE;padding:1rem;border-radius:8px">cd apps/desktop/web-ui
npm install
npm run build
cargo build -p qinglink-desktop</pre>
<p>后台接口可用：<a style="color:#0B6DBE" href="/api/admin/v1/health">/api/admin/v1/health</a></p>
</body></html>
"#;
