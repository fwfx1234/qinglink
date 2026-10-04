//! 授权使用内核提供的对端身份，以及由管理员固定的可执行映像。
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use windows::Win32::Foundation::{FILETIME, HANDLE};
use windows::Win32::Security::{
    EqualSid, GetTokenInformation, IsWellKnownSid, TOKEN_QUERY, TOKEN_USER, TokenUser,
    WinLocalSystemSid,
};
use windows::Win32::System::Pipes::{GetNamedPipeClientProcessId, GetNamedPipeClientSessionId};
use windows::Win32::System::RemoteDesktop::{ProcessIdToSessionId, WTSQueryUserToken};
use windows::Win32::System::Threading::{
    GetProcessTimes, OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION,
};

/// Cache key for a verified desktop peer: process id plus image creation time.
type TrustedKey = (u32, u32, u32);

/// Cache of already-verified desktop processes, keyed by [`TrustedKey`].
type TrustedCache = Mutex<HashMap<TrustedKey, OwnedHandle>>;

#[derive(Clone, Copy, Debug)]
pub(super) struct DesktopPeer {
    pub session: u32,
}

#[derive(Deserialize)]
struct TrustedDesktop {
    path: PathBuf,
    sha256: String,
}

pub(super) fn desktop(pipe: HANDLE) -> Result<DesktopPeer> {
    let mut pid = 0;
    let mut pipe_session = 0;
    let mut process_session = 0;
    // SAFETY: the connected endpoint is borrowed for this complete call.
    unsafe {
        GetNamedPipeClientProcessId(pipe, &mut pid)?;
        GetNamedPipeClientSessionId(pipe, &mut pipe_session)?;
        ProcessIdToSessionId(pid, &mut process_session)?;
    }
    ensure!(
        pipe_session != 0 && pipe_session == process_session,
        "desktop must run in an interactive session"
    );
    // SAFETY: the successful process handle is transferred to one owner.
    let process = unsafe {
        OwnedHandle::from_raw_handle(OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid)?.0)
    };
    let process_handle = HANDLE(process.as_raw_handle());
    let token = process_token(process_handle)?;
    let caller = token_user(HANDLE(token.as_raw_handle()))?;
    let mut session_token = HANDLE::default();
    // SAFETY: service runs as LocalSystem and obtains this session's user token.
    unsafe {
        WTSQueryUserToken(pipe_session, &mut session_token)?;
    }
    let session_token = unsafe { OwnedHandle::from_raw_handle(session_token.0) };
    let expected = token_user(HANDLE(session_token.as_raw_handle()))?;
    // SAFETY: both aligned buffers contain TOKEN_USER and retain their SID data.
    let same_user = unsafe {
        EqualSid(
            (*(caller.as_ptr().cast::<TOKEN_USER>())).User.Sid,
            (*(expected.as_ptr().cast::<TOKEN_USER>())).User.Sid,
        )
        .is_ok()
    };
    ensure!(
        same_user,
        "desktop token does not belong to the interactive session user"
    );

    let mut created = FILETIME::default();
    let mut exited = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    unsafe {
        GetProcessTimes(
            process_handle,
            &mut created,
            &mut exited,
            &mut kernel,
            &mut user,
        )?;
    }
    let key = (pid, created.dwHighDateTime, created.dwLowDateTime);
    static TRUSTED: OnceLock<TrustedCache> = OnceLock::new();
    let mut cache = TRUSTED
        .get_or_init(Mutex::default)
        .lock()
        .map_err(|_| anyhow::anyhow!("authorization cache poisoned"))?;
    if !cache.contains_key(&key) {
        let executable = std::env::current_exe()?;
        let record = executable
            .parent()
            .context("service installation directory missing")?
            .join("trusted-desktop.json");
        let trust: TrustedDesktop = serde_json::from_slice(
            &std::fs::read(record)
                .context("desktop trust record missing; repair service installation")?,
        )?;
        let image = super::process_image_path(process_handle)?;
        ensure!(
            image == trust.path.canonicalize()?,
            "desktop executable path is not registered"
        );
        let bytes = std::fs::read(&image).context("read registered desktop image")?;
        ensure!(
            format!("{:x}", Sha256::digest(bytes)) == trust.sha256,
            "desktop image changed; repair service installation"
        );
        // The live process handle plus creation time protects against PID reuse.
        if cache.len() >= 16 {
            cache.clear();
        }
        cache.insert(key, process);
    }
    Ok(DesktopPeer {
        session: pipe_session,
    })
}

fn process_token(process: HANDLE) -> Result<OwnedHandle> {
    let mut token = HANDLE::default();
    // SAFETY: valid process and writable output; transfer successful handle once.
    unsafe {
        OpenProcessToken(process, TOKEN_QUERY, &mut token)?;
        Ok(OwnedHandle::from_raw_handle(token.0))
    }
}

fn token_user(token: HANDLE) -> Result<Vec<usize>> {
    let mut needed = 0;
    // The sizing call is expected to return insufficient buffer.
    unsafe {
        let _ = GetTokenInformation(token, TokenUser, None, 0, &mut needed);
    }
    ensure!(
        needed as usize >= std::mem::size_of::<TOKEN_USER>(),
        "invalid token information length"
    );
    let mut buffer = vec![0usize; (needed as usize).div_ceil(std::mem::size_of::<usize>())];
    unsafe {
        GetTokenInformation(
            token,
            TokenUser,
            Some(buffer.as_mut_ptr().cast()),
            needed,
            &mut needed,
        )?;
    }
    Ok(buffer)
}

pub(super) fn system_process(process: HANDLE) -> Result<()> {
    let token = process_token(process)?;
    let user = token_user(HANDLE(token.as_raw_handle()))?;
    let is_system = unsafe {
        IsWellKnownSid(
            (*(user.as_ptr().cast::<TOKEN_USER>())).User.Sid,
            WinLocalSystemSid,
        )
        .as_bool()
    };
    ensure!(is_system, "agent caller is not LocalSystem");
    Ok(())
}
