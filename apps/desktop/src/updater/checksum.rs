//! SHA256SUMS 校验清单的解析与查询。
//!
//! 清单沿用 GNU `sha256sum` 的行格式：`<64位十六进制>␠␠<文件名>`，每行一条。
//! 解析时容忍 CRLF、单/双空格与行首 `\`（coreutils 对含特殊字符文件名的转义标记），
//! 但对哈希本身严格校验：非 64 位十六进制、重复文件名都视为格式错误。
//! 文件名统一小写后作为键，便于按 release asset 名查询。

use std::collections::HashMap;

use anyhow::{Result, bail, ensure};

/// 解析清单文本，返回 `小写文件名 → 小写哈希` 的映射。
pub fn parse_manifest(text: &str) -> Result<HashMap<String, String>> {
    let mut map = HashMap::new();
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        // coreutils 在文件名含 `\` 或换行时会在行首加 `\` 作转义标记；本项目的资产名
        // 不含这些字符，这里只剥掉标记，不解析其转义语义（遇到即视为不支持的清单）。
        if line.starts_with('\\') {
            bail!("校验清单含转义文件名，暂不支持：{line}");
        }
        let mut parts = line.split_whitespace();
        let hash = parts.next().unwrap_or_default();
        let name = parts.collect::<Vec<_>>().join(" ");
        ensure!(
            hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit()),
            "校验清单存在无效哈希：{line}"
        );
        // GNU sha256sum 在二进制模式输出 `hash *file`（星号分隔），文本模式输出
        // `hash  file`（双空格）。split_whitespace 后星号会粘在文件名前，剥掉它。
        let name = name.strip_prefix('*').unwrap_or(&name);
        ensure!(!name.is_empty(), "校验清单存在缺少文件名的行：{line}");
        let key = name.to_ascii_lowercase();
        ensure!(
            map.insert(key.clone(), hash.to_ascii_lowercase()).is_none(),
            "校验清单存在重复文件名：{name}"
        );
    }
    ensure!(!map.is_empty(), "校验清单为空");
    Ok(map)
}

/// 按文件名查询期望哈希；文件名大小写不敏感。
pub fn lookup<'a>(map: &'a HashMap<String, String>, name: &str) -> Result<&'a str> {
    map.get(&name.to_ascii_lowercase())
        .map(String::as_str)
        .ok_or_else(|| anyhow::anyhow!("校验清单缺少条目：{name}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_standard_manifest() {
        let text =
            "d7a8fbb307d7809469ca9abcb0082e4f8d5651e46d3cdb762d02d0bf37c9e592  QingLink.exe\n";
        let map = parse_manifest(text).unwrap();
        assert_eq!(
            lookup(&map, "qinglink.exe").unwrap(),
            "d7a8fbb307d7809469ca9abcb0082e4f8d5651e46d3cdb762d02d0bf37c9e592"
        );
    }

    #[test]
    fn tolerates_crlf_and_single_space() {
        let text =
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa QingLink.exe\r\n";
        let map = parse_manifest(text).unwrap();
        assert_eq!(lookup(&map, "QingLink.exe").unwrap(), "a".repeat(64));
    }

    #[test]
    fn tolerates_binary_mode_star_separator() {
        let text = format!("{} *QingLink.exe", "f".repeat(64));
        let map = parse_manifest(&text).unwrap();
        assert_eq!(lookup(&map, "QingLink.exe").unwrap(), "f".repeat(64));
    }

    #[test]
    fn handles_plus_in_filename() {
        let text = format!("{}  QingLink-mobile-1.0.0+3-android.apk", "b".repeat(64));
        let map = parse_manifest(&text).unwrap();
        assert_eq!(
            lookup(&map, "QingLink-mobile-1.0.0+3-android.apk").unwrap(),
            "b".repeat(64)
        );
    }

    #[test]
    fn rejects_invalid_hash() {
        let text = "short  QingLink.exe";
        assert!(parse_manifest(text).is_err());
    }

    #[test]
    fn rejects_duplicate_names() {
        let text = format!(
            "{}  QingLink.exe\n{}  qinglink.exe",
            "c".repeat(64),
            "d".repeat(64)
        );
        assert!(parse_manifest(&text).is_err());
    }

    #[test]
    fn missing_entry_is_error() {
        let map = parse_manifest(&format!("{}  QingLink.exe", "e".repeat(64))).unwrap();
        assert!(lookup(&map, "Missing.exe").is_err());
    }
}
