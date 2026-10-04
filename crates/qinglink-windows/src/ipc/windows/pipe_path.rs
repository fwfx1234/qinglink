//! 本地命名管道路径的校验与宽字符转换。

use anyhow::{Result, ensure};

/// 校验 `\\.\pipe\NAME` 形式的本地管道名，并转换为 NUL 结尾的宽字符串。
///
/// 拒绝远程 UNC 路径、设备命名空间别名、空名、多段名与非法字符。
pub(super) fn local_pipe_path(path: &str) -> Result<Vec<u16>> {
    const PREFIX: &str = r"\\.\pipe\";
    let prefix = path.get(..PREFIX.len()).unwrap_or_default();
    ensure!(
        prefix.eq_ignore_ascii_case(PREFIX),
        "pipe path must use the local \\\\.\\pipe\\ namespace"
    );
    let name = &path[PREFIX.len()..];
    ensure!(
        !name.is_empty()
            && name != "."
            && name != ".."
            && !name.chars().any(|c| matches!(c, '\0' | '/' | '\\' | ':')),
        "pipe path must contain a nonempty single-component local name"
    );
    let mut wide: Vec<u16> = path.encode_utf16().collect();
    ensure!(wide.len() <= 256, "local pipe path is too long");
    wide.push(0);
    Ok(wide)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_accepts_local_single_component_pipe_paths() {
        assert!(local_pipe_path(r"\\.\pipe\qinglink-service").is_ok());
        assert!(local_pipe_path(r"\\.\PIPE\qinglink-service").is_ok());
        for path in [
            r"\\server\pipe\qinglink-service",
            r"\\localhost\pipe\qinglink-service",
            r"\\?\pipe\qinglink-service",
            r"\\.\pipe\",
            r"\\.\pipe\..",
            r"\\.\pipe\name\child",
            r"\\.\pipe\name/child",
            r"\\.\pipe\name:stream",
            "\\\\.\\pipe\\name\0suffix",
            "世界",
        ] {
            assert!(local_pipe_path(path).is_err());
        }
    }
}
