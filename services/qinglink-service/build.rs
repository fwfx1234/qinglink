//! `qinglink-service` 的构建脚本：内嵌随构建号变化的版本资源。
//!
//! 版本信息不再写死在仓库里的静态 `.rc` 中：构建号每次源码改动都会变，写死的
//! `FILEVERSION` 会让文件属性永远停在旧版本。这里按当前版本与构建号现生成一份
//! `VERSIONINFO` 后嵌入；服务没有应用清单，因此不需要额外的 `.rc` 文件。

use std::{env, fs, path::PathBuf};

fn main() {
    // 构建号一变，嵌进可执行文件的版本资源也要跟着变。
    println!("cargo:rerun-if-changed=../../crates/qinglink-version/build-number.txt");

    let script =
        qinglink_version::version_info_rc("QingLink System Service", "QingLinkService.exe");
    let path = PathBuf::from(env::var_os("OUT_DIR").expect("out dir")).join("version_info.rc");
    fs::write(&path, script).expect("write generated version resource");

    #[cfg(windows)]
    embed_resource::compile(&path, embed_resource::NONE)
        .manifest_optional()
        .unwrap_or_else(|error| panic!("cannot embed version_info.rc: {error}"));
}
