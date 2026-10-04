//! 可执行文件原生的启动选项：启动行为永远不依赖脚本。

use anyhow::{Context, Result, bail};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct StartupOptions {
    /// Launched by the autostart entry rather than by the user.
    pub startup: bool,
    /// Wait for this process to exit before taking over (used by restart).
    pub restart_parent: Option<u32>,
    /// Print diagnostics and exit without starting anything.
    pub self_check: bool,
    /// Probe the service pipe, print the result, and exit.
    pub check_service: bool,
}

impl StartupOptions {
    pub fn parse(args: &[String]) -> Result<Self> {
        let mut options = Self::default();
        let mut arguments = args.iter().skip(1);
        while let Some(argument) = arguments.next() {
            match argument.as_str() {
                "--startup" => options.startup = true,
                "--self-check" => options.self_check = true,
                "--check-service" => options.check_service = true,
                "--restart-parent" => {
                    options.restart_parent = Some(
                        arguments
                            .next()
                            .context("--restart-parent requires a process ID")?
                            .parse()
                            .context("invalid restart parent process ID")?,
                    );
                }
                other => bail!("未知启动参数: {other}"),
            }
        }
        Ok(options)
    }
}
