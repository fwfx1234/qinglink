// The shell's "pick a file on this PC" dialog.
//
// The management interface runs in a browser, which cannot see the host's file
// system: an `<input type="file">` would hand back an opaque blob rather than
// the absolute path the catalog has to store. The desktop process lives in the
// user's own session, so it opens the real shell picker on request and answers
// with the path the service can then read.

use std::path::Path;

/// What the picker is being opened for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileDialogKind {
    /// Icon artwork: a bitmap file, or a program whose icon can be extracted.
    Image,
    /// Something launchable to register in the catalog.
    Executable,
}

impl FileDialogKind {
    /// Parse the wire value used by the management API.
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "image" => Some(Self::Image),
            "executable" => Some(Self::Executable),
            _ => None,
        }
    }

    /// Caption shown in the dialog's title bar.
    fn title(self) -> &'static str {
        match self {
            Self::Image => "选择图标图片或程序",
            Self::Executable => "选择要添加的程序",
        }
    }

    /// `(label, pattern)` pairs for the file-type dropdown, best match first.
    fn filters(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Self::Image => &[
                ("图片文件", "*.png;*.jpg;*.jpeg;*.bmp;*.ico"),
                ("程序", "*.exe"),
                ("全部文件", "*.*"),
            ],
            Self::Executable => &[("程序与快捷方式", "*.exe;*.lnk"), ("全部文件", "*.*")],
        }
    }
}

/// Open the shell's file picker and return the chosen absolute path.
///
/// `Ok(None)` means the user cancelled. The call blocks until the dialog closes,
/// so it belongs on a thread of its own: the picker is a single-threaded
/// apartment component and will not open on an MTA thread.
///
/// # Errors
/// Fails when COM cannot be initialized on this thread, or when the dialog
/// cannot be created or shown.
pub fn pick_file(kind: FileDialogKind, start_in: Option<&Path>) -> anyhow::Result<Option<String>> {
    use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance};
    use windows::Win32::UI::Shell::{FileOpenDialog, IFileDialog, IFileOpenDialog};
    use windows::core::Interface;

    let _apartment = StaApartment::init()?;
    let dialog: IFileOpenDialog =
        unsafe { CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER) }
            .map_err(|error| anyhow::anyhow!("无法创建文件选择窗口: {error}"))?;
    // The derived interface exposes only its own methods, so the shared
    // `IFileDialog` surface is reached through the base interface.
    let base: IFileDialog = dialog
        .cast()
        .map_err(|error| anyhow::anyhow!("文件选择窗口接口不完整: {error}"))?;

    unsafe {
        let options = base.GetOptions().unwrap_or_default();
        base.SetOptions(
            options
                | windows::Win32::UI::Shell::FOS_FORCEFILESYSTEM
                | windows::Win32::UI::Shell::FOS_FILEMUSTEXIST
                | windows::Win32::UI::Shell::FOS_PATHMUSTEXIST
                | windows::Win32::UI::Shell::FOS_NOCHANGEDIR,
        )
        .map_err(|error| anyhow::anyhow!("无法配置文件选择窗口: {error}"))?;

        let title = wide(kind.title());
        base.SetTitle(windows::core::PCWSTR(title.as_ptr()))
            .map_err(|error| anyhow::anyhow!("无法设置窗口标题: {error}"))?;
        set_filters(&base, kind)?;
        set_start_folder(&base, start_in);

        // Owning the dialog to whatever the user just clicked makes it appear on
        // top of the browser instead of behind it.
        let owner = windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow();
        match base.Show(Some(owner)) {
            Ok(()) => {}
            Err(error) if error.code().0 == CANCELLED => return Ok(None),
            Err(error) => return Err(anyhow::anyhow!("文件选择窗口无法打开: {error}")),
        }

        let item = base
            .GetResult()
            .map_err(|error| anyhow::anyhow!("读取选择结果失败: {error}"))?;
        let raw = item
            .GetDisplayName(windows::Win32::UI::Shell::SIGDN_FILESYSPATH)
            .map_err(|error| anyhow::anyhow!("该位置没有文件系统路径: {error}"))?;
        let path = raw
            .to_string()
            .map_err(|_| anyhow::anyhow!("选择的路径包含无效字符"));
        windows::Win32::System::Com::CoTaskMemFree(Some(raw.0 as *const core::ffi::c_void));
        path.map(Some)
    }
}

/// `HRESULT_FROM_WIN32(ERROR_CANCELLED)`: the user closed the dialog.
const CANCELLED: i32 = 0x8007_04C7u32 as i32;

/// Install the file-type dropdown.
#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn set_filters(
    base: &windows::Win32::UI::Shell::IFileDialog,
    kind: FileDialogKind,
) -> anyhow::Result<()> {
    use windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;

    // `SetFileTypes` keeps the pointers, so the UTF-16 buffers must outlive the
    // call: build every string first, then the descriptor array that points at
    // them.
    let labels: Vec<Vec<u16>> = kind
        .filters()
        .iter()
        .map(|(label, _)| wide(label))
        .collect();
    let patterns: Vec<Vec<u16>> = kind
        .filters()
        .iter()
        .map(|(_, pattern)| wide(pattern))
        .collect();
    let specs: Vec<COMDLG_FILTERSPEC> = labels
        .iter()
        .zip(patterns.iter())
        .map(|(label, pattern)| COMDLG_FILTERSPEC {
            pszName: windows::core::PCWSTR(label.as_ptr()),
            pszSpec: windows::core::PCWSTR(pattern.as_ptr()),
        })
        .collect();

    base.SetFileTypes(&specs)
        .map_err(|error| anyhow::anyhow!("无法设置文件类型筛选: {error}"))
}

/// Open the picker in a directory the caller already knows about.
///
/// Advisory only: an unknown or unreachable folder leaves the shell's own
/// "last visited place" behaviour in charge.
#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn set_start_folder(base: &windows::Win32::UI::Shell::IFileDialog, start_in: Option<&Path>) {
    use windows::Win32::UI::Shell::{IShellItem, SHCreateItemFromParsingName};

    let Some(directory) = start_in.filter(|path| path.is_dir()) else {
        return;
    };
    let directory = wide(&directory.to_string_lossy());
    let Ok(item) = SHCreateItemFromParsingName::<_, _, IShellItem>(
        windows::core::PCWSTR(directory.as_ptr()),
        None,
    ) else {
        return;
    };
    let _ = base.SetDefaultFolder(&item);
}

/// NUL-terminated UTF-16 for a Win32 string parameter.
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

/// A single-threaded apartment, held for the lifetime of one dialog.
///
/// `IFileOpenDialog` is an apartment-threaded component: created from an MTA
/// thread it fails outright, which is why the caller gives the picker its own
/// thread rather than a pooled one.
struct StaApartment {
    /// Whether *this* call initialized COM, and therefore has to undo it.
    owns_initialization: bool,
}

impl StaApartment {
    fn init() -> anyhow::Result<Self> {
        use windows::Win32::Foundation::RPC_E_CHANGED_MODE;
        use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};

        let result = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
        if result.is_ok() {
            return Ok(Self {
                owns_initialization: true,
            });
        }
        if result == RPC_E_CHANGED_MODE {
            anyhow::bail!("文件选择窗口需要单线程 COM 单元，当前线程已属于其他单元");
        }
        Err(windows::core::Error::from(result).into())
    }
}

impl Drop for StaApartment {
    fn drop(&mut self) {
        if self.owns_initialization {
            unsafe { windows::Win32::System::Com::CoUninitialize() };
        }
    }
}
