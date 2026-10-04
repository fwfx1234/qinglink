//! 内嵌的前端构建产物。
//!
//! `build.rs` 从 `apps/desktop/web-ui/dist` 生成下面 include 进来的模块，因此
//! 可执行文件无需旁挂目录即可提供自己的界面。过期判定与缺少 Node 时的回退方式
//! 见该脚本。

include!(concat!(env!("OUT_DIR"), "/web_assets.rs"));

/// Whether the embedded shell is the build-time placeholder rather than a real
/// frontend build.
pub fn is_placeholder() -> bool {
    IS_PLACEHOLDER
}
