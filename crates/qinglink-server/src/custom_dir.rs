//! 自定义目录管理。
//!
//! 允许用户指定额外的目录，供应用扫描使用。
//!
//! 按职责拆分：配置与请求模型在 `model`，写入校验与持久化在 `persistence`，
//! 目录增删改查 API 在 `api`，目录遍历扫描在 `scan`，与 `AppScanner` 的条目
//! 映射在 `entry`，既有测试在 `tests`。

mod api;
mod entry;
mod model;
mod persistence;
mod scan;
#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests;

pub use model::{
    AddCustomDirRequest, CustomDir, CustomDirConfig, DirValidationResult, UpdateCustomDirRequest,
};

/// 递归扫描的深度上限，写入配置时用于收敛用户给出的 `max_depth`。
const MAX_SCAN_DEPTH: u32 = 8;

/// 自定义目录管理器
#[derive(Debug)]
pub struct CustomDirManager {
    config: CustomDirConfig,
    config_path: std::path::PathBuf,
}
