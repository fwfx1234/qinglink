//! 桌面端与服务端共用的产品版本号。
//!
//! 版本号只有一个来源：`Cargo.toml` 里的 `workspace.package.version` 承载语义版本，
//! 构建号由本 crate 的 `build.rs` 在源码改动时自增。两个程序都从这里取值，界面上
//! 看到的桌面端版本与服务端版本因此可以直接比较。

/// `Cargo.toml` 中的语义版本，例如 `0.1.0`。
pub const BASE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// 完整版本号：语义版本 + 构建号，例如 `0.1.0+13`。
///
/// `+` 之后是 semver 的构建元数据，不影响版本先后比较。
pub const VERSION: &str = env!("QINGLINK_VERSION");

/// 构建号：源码每改动一次，重新构建后加一。
///
/// 解析失败（理论上不会发生）按 0 处理，让调用方能继续展示版本号。
pub fn build_number() -> u32 {
    VERSION
        .rsplit_once('+')
        .and_then(|(_, build)| build.parse().ok())
        .unwrap_or(0)
}

/// 语义版本的前三段数字，供 Windows 版本资源的四段版本号使用。
pub fn semantic_version_parts() -> (u16, u16, u16) {
    let mut parts = BASE_VERSION
        .split(['-', '+'])
        .next()
        .unwrap_or_default()
        .split('.')
        .map(|part| part.parse::<u16>().unwrap_or(0));
    (
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    )
}

/// 生成 Windows 版本资源脚本（`VERSIONINFO`），交给构建脚本写入并嵌入。
///
/// `0.1.0+13` 会生成 `FILEVERSION 0,1,0,13` 与 `FileVersion "0.1.0.13"`：文件属性、
/// 界面显示与 `/health` 应答因此永远指向同一个版本。
pub fn version_info_rc(description: &str, file_name: &str) -> String {
    let (major, minor, patch) = semantic_version_parts();
    let build = build_number().min(u32::from(u16::MAX));
    let numeric = format!("{major},{minor},{patch},{build}");
    let text = format!("{major}.{minor}.{patch}.{build}");
    format!(
        r#"#include <windows.h>

1 VERSIONINFO
FILEVERSION {numeric}
PRODUCTVERSION {numeric}
FILEFLAGSMASK 0x3fL
FILEFLAGS 0x0L
FILEOS 0x40004L
FILETYPE 0x1L
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040904e4"
    BEGIN
      VALUE "CompanyName", "QingLink\0"
      VALUE "FileDescription", "{description}\0"
      VALUE "FileVersion", "{text}\0"
      VALUE "InternalName", "{file_name}\0"
      VALUE "OriginalFilename", "{file_name}\0"
      VALUE "ProductName", "QingLink\0"
      VALUE "ProductVersion", "{text}\0"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x0409, 1252
  END
END
"#
    )
}
