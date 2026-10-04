//! 进程令牌完整性级别，以及通知区域（shell）所有者的级别。
//!
//! 同一份读取逻辑服务于两件事，所以它只在这里实现一次：
//!
//! * **托盘注册**：shell 只接受与通知区域所有者同级的进程注册图标，低级或高级
//!   （管理员）都会被拒绝，`Shell_NotifyIcon` 返回 `FALSE`、`GetLastError=5`；
//! * **打开浏览器**：低于 shell 的进程连把 URL 交给已在运行的默认浏览器都做不到
//!   —— 强制完整性策略禁止 write-up，而 `ShellExecuteW` 在这种情况下**仍然返回
//!   成功**，于是表现为"什么都没发生"，调用方无从察觉。
//!
//! 读取本身是只读的：打开目标进程、打开其令牌、读出 `TokenIntegrityLevel`。

/// 一个完整性级别，按 `SECURITY_MANDATORY_LABEL` 的最后一个子颁发机构（RID）比较。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Integrity {
    rid: u32,
}

impl Integrity {
    /// 从 `TokenIntegrityLevel` 的 RID 构造。
    pub const fn from_rid(rid: u32) -> Self {
        Self { rid }
    }

    /// 原始 RID，用于比较与日志。
    pub const fn rid(self) -> u32 {
        self.rid
    }

    /// Windows 文档里的级别名；未识别的 RID 记为 `unknown`。
    pub fn name(self) -> &'static str {
        match self.rid {
            0x0000 => "untrusted",
            0x1000 => "low",
            0x2000 => "medium",
            0x2100 => "medium-plus",
            0x3000 => "high",
            0x4000 => "system",
            0x5000 => "protected",
            _ => "unknown",
        }
    }

    /// 是否至少与 `other` 同级。
    pub const fn is_at_least(self, other: Self) -> bool {
        self.rid >= other.rid
    }
}

/// `current` 低于 `shell` 时为真：这类进程既注册不了托盘图标，也无法把 URL 交给
/// 已在运行的浏览器。
pub const fn is_below(current: Integrity, shell: Integrity) -> bool {
    current.rid < shell.rid
}

/// 本进程令牌的完整性级别；读不到时为 `None`。
#[cfg(target_os = "windows")]
pub fn current_integrity() -> Option<Integrity> {
    process_integrity(std::process::id())
}

/// 本进程令牌的完整性级别；非 Windows 上始终是 `None`。
#[cfg(not(target_os = "windows"))]
pub fn current_integrity() -> Option<Integrity> {
    None
}

/// 通知区域所有者（shell 窗口所属进程）的完整性级别；读不到时为 `None`。
#[cfg(target_os = "windows")]
pub fn shell_integrity() -> Option<Integrity> {
    use windows::Win32::UI::WindowsAndMessaging::{GetShellWindow, GetWindowThreadProcessId};

    // SAFETY: `GetShellWindow` 只读取 shell 的窗口句柄；该窗口属于别的进程，这里
    // 只取它的进程号。
    let shell = unsafe { GetShellWindow() };
    if shell.is_invalid() {
        return None;
    }
    let mut owner = 0_u32;
    // SAFETY: `shell` 是存活句柄，输出是本地变量。
    unsafe { GetWindowThreadProcessId(shell, Some(&mut owner)) };
    if owner == 0 {
        return None;
    }
    process_integrity(owner)
}

/// 通知区域所有者的完整性级别；非 Windows 上始终是 `None`。
#[cfg(not(target_os = "windows"))]
pub fn shell_integrity() -> Option<Integrity> {
    None
}

/// 本进程完整性级别的名字，读不到时是 `unknown`。
pub fn current_integrity_name() -> String {
    current_integrity().map_or_else(|| "unknown".to_string(), |level| level.name().to_string())
}

/// 通知区域所有者完整性级别的名字，读不到时是 `unknown`。
///
/// `unknown` 本身是有信息量的：读不到通常意味着本进程低于它，连它的令牌都打不开。
pub fn shell_integrity_name() -> String {
    shell_integrity().map_or_else(|| "unknown".to_string(), |level| level.name().to_string())
}

/// 本进程是否低于通知区域所有者；任一侧读不到时为 `None`。
pub fn is_below_shell() -> Option<bool> {
    Some(is_below(current_integrity()?, shell_integrity()?))
}

/// 任意进程令牌的完整性级别。
///
/// 打不开进程或令牌时返回 `None`，这本身就是结论：低完整性进程通常打不开更高
/// 完整性进程的令牌。
#[cfg(target_os = "windows")]
pub fn process_integrity(pid: u32) -> Option<Integrity> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::Security::{
        GetSidSubAuthority, GetSidSubAuthorityCount, GetTokenInformation, TOKEN_MANDATORY_LABEL,
        TOKEN_QUERY, TokenIntegrityLevel,
    };
    use windows::Win32::System::Threading::{OpenProcess, OpenProcessToken, PROCESS_ACCESS_RIGHTS};

    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;

    // SAFETY: 两个句柄都在返回前关闭，缓冲区由 `GetTokenInformation` 定尺寸，且在其中
    // 保留着 `TOKEN_MANDATORY_LABEL` 的 SID 数据。
    unsafe {
        let process = OpenProcess(
            PROCESS_ACCESS_RIGHTS(PROCESS_QUERY_LIMITED_INFORMATION),
            false,
            pid,
        )
        .ok()?;
        let mut token = Default::default();
        if OpenProcessToken(process, TOKEN_QUERY, &mut token).is_err() {
            let _ = CloseHandle(process);
            return None;
        }
        let mut size = 0_u32;
        let _ = GetTokenInformation(token, TokenIntegrityLevel, None, 0, &mut size);
        let result = if size == 0 {
            None
        } else {
            let mut buffer = vec![0_u8; size as usize];
            let queried = GetTokenInformation(
                token,
                TokenIntegrityLevel,
                Some(buffer.as_mut_ptr() as *mut core::ffi::c_void),
                size,
                &mut size,
            );
            if queried.is_err() {
                None
            } else {
                let label = &*(buffer.as_ptr() as *const TOKEN_MANDATORY_LABEL);
                let count = GetSidSubAuthorityCount(label.Label.Sid);
                if count.is_null() || *count == 0 {
                    None
                } else {
                    let rid = GetSidSubAuthority(label.Label.Sid, (*count - 1) as u32);
                    if rid.is_null() {
                        None
                    } else {
                        Some(Integrity::from_rid(*rid))
                    }
                }
            }
        };
        let _ = CloseHandle(token);
        let _ = CloseHandle(process);
        result
    }
}

/// 任意进程令牌的完整性级别；非 Windows 上始终是 `None`。
#[cfg(not(target_os = "windows"))]
pub fn process_integrity(_pid: u32) -> Option<Integrity> {
    None
}

#[cfg(test)]
mod tests {
    use super::{Integrity, is_below};

    #[test]
    fn ranks_follow_the_documented_rids() {
        let low = Integrity::from_rid(0x1000);
        let medium = Integrity::from_rid(0x2000);
        let high = Integrity::from_rid(0x3000);

        assert!(is_below(low, medium), "low 低于 medium");
        assert!(!is_below(medium, medium), "同级不算低于");
        assert!(!is_below(high, medium), "管理员高于 shell，不是低于");
        assert!(high.is_at_least(medium));
        assert!(!low.is_at_least(medium));
    }

    #[test]
    fn names_match_the_reported_json() {
        assert_eq!(Integrity::from_rid(0x0000).name(), "untrusted");
        assert_eq!(Integrity::from_rid(0x1000).name(), "low");
        assert_eq!(Integrity::from_rid(0x2000).name(), "medium");
        assert_eq!(Integrity::from_rid(0x3000).name(), "high");
        assert_eq!(Integrity::from_rid(0x1234).name(), "unknown");
    }
}
