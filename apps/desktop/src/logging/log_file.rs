//! 磁盘上日志文件的名称校验与有界读取。
//!
//! 返回给调用方的每条路径都要过两道检查：请求的文件名必须符合严格字符集，规范化
//! 之后解析出的路径必须仍位于所选日志目录内。读取始终有界，因此下载或诊断包不会
//! 被一个增长数月的日志文件变成无界读取。

use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// Validate a file name and resolve it inside `directory`.
pub(crate) fn resolve_log_path(directory: &Path, requested: &str) -> Result<PathBuf> {
    anyhow::ensure!(
        is_safe_log_file_name(requested),
        "日志文件名不合法：{requested}"
    );
    let candidate = directory.join(requested);
    let canonical_directory = directory
        .canonicalize()
        .context("cannot resolve the log directory")?;
    let canonical_file = candidate
        .canonicalize()
        .with_context(|| format!("日志文件不存在：{requested}"))?;
    anyhow::ensure!(
        canonical_file.starts_with(&canonical_directory) && canonical_file.is_file(),
        "日志文件不在日志目录中：{requested}"
    );
    Ok(canonical_file)
}

/// Read at most `max_bytes` from the end of a file, dropping a partial first line.
pub(crate) fn read_tail(path: &Path, max_bytes: u64) -> Result<Vec<u8>> {
    let mut file =
        fs::File::open(path).with_context(|| format!("无法打开日志文件：{}", path.display()))?;
    let length = file
        .metadata()
        .with_context(|| format!("无法读取日志文件属性：{}", path.display()))?
        .len();
    let start = length.saturating_sub(max_bytes);
    file.seek(SeekFrom::Start(start))
        .context("cannot seek the log file")?;
    let mut buffer = Vec::new();
    file.take(max_bytes)
        .read_to_end(&mut buffer)
        .context("cannot read the log file")?;
    if start > 0
        && let Some(index) = buffer.iter().position(|byte| *byte == b'\n')
    {
        buffer.drain(..=index);
    }
    Ok(buffer)
}

/// Reject anything that could address a file outside the log directory.
fn is_safe_log_file_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && name != "."
        && name != ".."
        && !name.contains("..")
        && name.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
        })
}
