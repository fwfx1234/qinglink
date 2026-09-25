use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

#[derive(Clone, Debug)]
pub struct AppPaths {
    data_dir: PathBuf,
}

impl AppPaths {
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

    pub fn resolve_system() -> Result<Self> {
        let data_dir = match env::var_os("QINGLINK_DATA_DIR") {
            Some(value) => PathBuf::from(value),
            None => system_data_dir()?,
        };
        fs::create_dir_all(&data_dir).with_context(|| {
            format!("cannot create system data directory {}", data_dir.display())
        })?;
        enforce_private_acl(&data_dir);
        Ok(Self { data_dir })
    }

    pub fn for_test(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
        }
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }
    pub fn database(&self, _name: &str) -> PathBuf {
        self.data_dir.join("qinglink.db")
    }

    pub fn config(&self, name: &str) -> PathBuf {
        self.child("config", name)
    }

    pub fn log_file(&self, name: &str) -> PathBuf {
        self.child("logs", name)
    }

    pub fn feature_state(&self, feature: &str, name: &str) -> PathBuf {
        self.child(&format!("state/{feature}"), name)
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
        return Ok(root.join("QingLink"));
    }
    #[cfg(not(target_os = "windows"))]
    {
        dirs::data_local_dir()
            .or_else(dirs::data_dir)
            .context("cannot resolve system data directory")
            .map(|path| path.join("QingLink"))
    }
}

fn enforce_private_acl(path: &Path) {
    #[cfg(target_os = "windows")]
    {
        // 服务拥有凭据和配对令牌。即使旧版本安装创建了该目录，
        // 也要保持 ACL 权限尽可能严格。
        let target = path.to_string_lossy().into_owned();
        let _ = std::process::Command::new("icacls")
            .args([
                &target,
                "/inheritance:e",
                "/grant",
                "SYSTEM:(OI)(CI)(F)",
                "*S-1-5-32-544:(OI)(CI)(F)",
            ])
            .status();
    }
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
