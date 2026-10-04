//! QingLink 的本地持久化层：应用数据路径解析、SQLite 连接池与 schema 迁移。

/// SQLite 连接池与应用数据库服务。
pub mod database;
/// 版本化的 schema 迁移。
pub mod migrations;
mod paths;

pub use paths::AppPaths;
