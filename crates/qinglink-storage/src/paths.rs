use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

/// Subdirectory of the system data directory holding human-readable logs.
const LOG_DIRECTORY: &str = "logs";

/// `NT AUTHORITY\SYSTEM`.
const SYSTEM_SID: &str = "*S-1-5-18:(OI)(CI)(F)";

/// `BUILTIN\Administrators`.
const ADMINISTRATORS_SID: &str = "*S-1-5-32-544:(OI)(CI)(F)";

/// `BUILTIN\Users`, used only to grant read access to the log subtree.
const BUILTIN_USERS_SID: &str = "*S-1-5-32-545";

/// 应用数据目录解析结果，提供数据库、配置与日志文件的路径。
#[derive(Clone, Debug)]
pub struct AppPaths {
    data_dir: PathBuf,
}

impl AppPaths {
    /// 解析当前用户的数据目录（`QINGLINK_DATA_DIR` 优先）并确保它存在。
    ///
    /// # Errors
    ///
    /// 无法确定用户数据目录，或创建该目录失败时返回错误。
    pub fn resolve() -> Result<Self> {
        let data_dir = match env::var_os("QINGLINK_DATA_DIR") {
            Some(value) => PathBuf::from(value),
            None => dirs::data_local_dir()
                .or_else(dirs::data_dir)
                .context("cannot resolve user data directory")?
                .join("QingLink"),
        };
        fs::create_dir_all(&data_dir)
            .with_context(|| format!("cannot create data directory {}", data_dir.display()))?;
        Ok(Self { data_dir })
    }

    /// 解析服务使用的系统数据目录，并对其收紧 ACL（Windows）。
    ///
    /// # Errors
    ///
    /// 无法确定系统数据目录、创建该目录失败，或收紧 ACL 失败时返回错误。
    pub fn resolve_system() -> Result<Self> {
        let (data_dir, enforce_acl) = match env::var_os("QINGLINK_DATA_DIR") {
            Some(value) => (PathBuf::from(value), false),
            None => (system_data_dir()?, true),
        };
        fs::create_dir_all(&data_dir).with_context(|| {
            format!("cannot create system data directory {}", data_dir.display())
        })?;
        if enforce_acl {
            enforce_private_acl(&data_dir).with_context(|| {
                format!("cannot secure system data directory {}", data_dir.display())
            })?;
        }
        Ok(Self { data_dir })
    }

    /// 用给定目录直接构造实例，跳过目录解析与 ACL 处理。
    pub fn for_test(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
        }
    }

    /// 返回应用数据目录。
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }
    /// 返回数据库文件路径；当前所有数据库共用同一个文件。
    pub fn database(&self, _name: &str) -> PathBuf {
        self.data_dir.join("qinglink.db")
    }

    /// 返回配置目录下的文件路径，必要时创建配置目录。
    pub fn config(&self, name: &str) -> PathBuf {
        self.child("config", name)
    }

    /// 返回日志目录下的文件路径，必要时创建日志目录。
    pub fn log_file(&self, name: &str) -> PathBuf {
        self.child("logs", name)
    }

    fn child(&self, directory: &str, name: &str) -> PathBuf {
        let dir = self.data_dir.join(directory);
        if let Err(error) = fs::create_dir_all(&dir) {
            tracing::warn!(%error, path = %dir.display(), "cannot create application directory");
        }
        dir.join(name)
    }
}

fn system_data_dir() -> Result<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let root = env::var_os("ProgramData")
            .map(PathBuf::from)
            .context("ProgramData is not available")?;
        Ok(root.join("QingLink"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        dirs::data_local_dir()
            .or_else(dirs::data_dir)
            .context("cannot resolve system data directory")
            .map(|path| path.join("QingLink"))
    }
}

fn enforce_private_acl(path: &Path) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        let target = path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("system data directory contains invalid characters"))?;
        run_icacls(&[target, "/reset", "/T", "/C"])?;
        run_icacls(&[target, "/inheritance:r", "/T", "/C"])?;
        run_icacls(&[
            target,
            "/grant:r",
            SYSTEM_SID,
            ADMINISTRATORS_SID,
            "/T",
            "/C",
        ])?;

        // Logs are diagnostics rather than business data, and the desktop's
        // management interface is the only way to read them: without read access
        // for the interactive user it can only report that the service failed,
        // never why. The relaxation is scoped to the log subtree and grants
        // read+execute only, so no client can rewrite the service's own record.
        let logs = path.join(LOG_DIRECTORY);
        if fs::create_dir_all(&logs).is_ok()
            && let Some(logs) = logs.to_str()
        {
            let users_grant = format!("{BUILTIN_USERS_SID}:(OI)(CI)(RX)");
            run_icacls(&[logs, "/grant:r", &users_grant, "/T", "/C"])?;
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = path;
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn run_icacls(arguments: &[&str]) -> Result<()> {
    let output = std::process::Command::new("icacls.exe")
        .args(arguments)
        .output()
        .context("cannot run icacls.exe")?;
    anyhow::ensure!(
        output.status.success(),
        "icacls.exe failed: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn databases_share_the_single_application_file() {
        let paths = AppPaths::for_test("test-data");
        assert_eq!(paths.database("auth.db"), paths.database("catalog.db"));
        assert!(paths.database("ignored").ends_with("qinglink.db"));
    }
}
