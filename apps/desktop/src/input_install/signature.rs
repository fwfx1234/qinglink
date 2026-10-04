use std::{fs, path::Path, process::Command};

use anyhow::{Context, Result, ensure};

use super::assets;

pub(crate) fn verify_embedded_assets() -> Result<()> {
    assets::verify_embedded_assets()
}

pub(crate) fn verify_embedded_signatures() -> Result<()> {
    let staging =
        std::env::temp_dir().join(format!("qinglink-signature-check-{}", std::process::id()));
    let result = (|| {
        assets::extract(&staging)?;
        verify_signature(&staging.join("QingLinkService.exe"))
    })();
    let _ = fs::remove_dir_all(staging);
    result
}

pub(crate) fn verify_signature(path: &Path) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        let literal = powershell_literal(&path.to_string_lossy());
        let script = format!(
            "if ((Get-AuthenticodeSignature -LiteralPath '{literal}').Status -ne 'Valid') {{ exit 1 }}"
        );
        let status = Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .status()
            .context("无法验证 Windows Authenticode 签名")?;
        ensure!(status.success(), "签名无效：{}", path.display());
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = path;
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn powershell_literal(value: &str) -> String {
    value.replace('\'', "''")
}

#[cfg(test)]
mod tests {
    #[test]
    fn powershell_paths_are_escaped() {
        assert_eq!(
            super::powershell_literal("C:\\O'Brien\\app.exe"),
            "C:\\O''Brien\\app.exe"
        );
    }
}
