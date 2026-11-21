//! 读取 PC 焦点输入框里已经存在的文本（UI Automation）。
//!
//! 手机端的软键盘输入框原来只镜像"本次从手机输入的差异"，PC 输入框里原本就有内容时
//! 手机看不到，退格与改字都会错位。这里用 UIA 取一次焦点控件文本，回填到手机输入框，
//! 让两端以同一份基线做 diff。
//!
//! 只认"`Edit` 控件 + `ValuePattern`"：密码框、只读控件、不提供 Value 模式的自绘控件
//! 一律返回 `Ok(None)`，由调用方退回"空基线"，这不是错误。
//!
//! **刻意不用 `TextPattern`**：它读的是控件可访问性文本树里的全部内容，而 Chromium 系
//! （浏览器、Electron、部分微信版本）会把输入框的占位提示（placeholder）作为文本节点
//! 放进这棵树。用文本树回填会把占位提示当成正文，用户一编辑就反向写进 PC 真实输入框。
//! 同理也不接受 `Document`（整页正文）与 `ComboBox`（选中项不是用户输入的内容）。
//!
//! 但 Chromium 系的富文本编辑器（contenteditable）连 `ValuePattern` 的 `Value` 都会
//! 混入占位提示——UIA 没有任何属性能把两者区分开。因此这里用**结构判据**兜底：
//! 有子元素的 `Edit` 视为富文本编辑器，不回填；纯 `<input>` / `<textarea>` 是叶子节点，
//! 浏览器地址栏、搜索框与普通表单照旧回填（见 [`has_child_elements`]）。

use anyhow::Result;
use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationElement, IUIAutomationValuePattern,
    TreeScope_Children, UIA_EditControlTypeId, UIA_ValuePatternId,
};

use super::ComApartment;

/// 回填文本的长度上限（按字符）。
///
/// 手机端输入框的最大长度就是 4096。多行编辑框（记事本、网页 textarea）可能有几万字，
/// 整段塞进 WS 载荷没有意义，超出部分直接截断。
const MAX_FOCUS_TEXT: usize = 4096;

/// 读取当前焦点输入框的已有文本。
///
/// 返回 `Ok(None)` 表示"没有可回填的输入框"：焦点不在 `Edit` 控件上、是 Chromium 系的
/// 富文本编辑器（`Value` 里可能混着占位提示）、是密码框或只读控件、控件不提供 Value 模式、
/// 取到的值是空白、或取到的值看起来是占位提示。只有 COM/UIA 初始化失败这类系统性故障
/// 才返回 `Err`。
///
/// # Errors
///
/// COM 公寓初始化失败，或创建 `IUIAutomation` 实例失败时返回错误。
pub fn read_focus_text() -> Result<Option<String>> {
    let _apartment = ComApartment::init()?;
    let automation: IUIAutomation =
        unsafe { CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER) }?;
    let Ok(element) = (unsafe { automation.GetFocusedElement() }) else {
        return Ok(None);
    };
    if !is_text_control(&element) {
        return Ok(None);
    }
    // Chromium 系的富文本编辑器把占位提示当成 DOM 子文本，`Value` 里会带上它，
    // 且 UIA 层无法与用户正文区分；有子元素就整段放弃回填（叶子节点照常回填）。
    if is_chromium(&element) && has_child_elements(&automation, &element) {
        return Ok(None);
    }
    let Some(pattern) = value_pattern(&element) else {
        return Ok(None);
    };
    if !is_editable(&element, &pattern) {
        return Ok(None);
    }
    let Some(text) = read_value(&pattern) else {
        return Ok(None);
    };
    if text.trim().is_empty() || is_placeholder_hint(&element, &text) {
        return Ok(None);
    }
    Ok(Some(truncate(text)))
}

/// 只接受 `Edit` 控件。
///
/// `ComboBox` 的 Value 是选中项而不是用户敲进去的内容，`Document` 的文本树是整页正文，
/// 两者回填到手机输入框都是错的——它们正是"读到的不是输入框内容"的主要来源。
fn is_text_control(element: &IUIAutomationElement) -> bool {
    let Ok(control_type) = (unsafe { element.CurrentControlType() }) else {
        return false;
    };
    control_type == UIA_EditControlTypeId
}

/// 是否 Chromium 系（Chrome / Edge / Electron / WebView2）。
///
/// 这类控件的网页内容不在 HWND 树里，只能靠 `GetFocusedElement` 拿到，而且富文本
/// 编辑器会污染 `Value`，需要额外的结构判据。
fn is_chromium(element: &IUIAutomationElement) -> bool {
    unsafe { element.CurrentFrameworkId() }
        .map(|id| id.to_string().eq_ignore_ascii_case("chrome"))
        .unwrap_or(false)
}

/// 焦点控件是否带子元素——Chromium 下这等价于"这是富文本编辑器（contenteditable）"。
///
/// 纯 `<input>` / `<textarea>` 在可访问性树里是叶子节点，占位提示不会进入 `Value`；
/// 而 contenteditable 的空文档本身就带 `<p>` / `<span>` 子节点，占位提示正是这些子节点
/// 里的文本。UIA 没有属性能把提示词和用户正文区分开，所以"有子元素就不回填"。
///
/// 读取失败按"叶子"处理：宁可多回填，也不要误伤整个浏览器。
fn has_child_elements(automation: &IUIAutomation, element: &IUIAutomationElement) -> bool {
    let Ok(condition) = (unsafe { automation.CreateTrueCondition() }) else {
        return false;
    };
    let Ok(found) = (unsafe { element.FindAll(TreeScope_Children, &condition) }) else {
        return false;
    };
    unsafe { found.Length() }
        .map(|len| len > 0)
        .unwrap_or(false)
}

/// 唯一的取值入口；拿不到 `ValuePattern` 就直接放弃，不回退到 `TextPattern`。
fn value_pattern(element: &IUIAutomationElement) -> Option<IUIAutomationValuePattern> {
    unsafe { element.GetCurrentPatternAs(UIA_ValuePatternId) }.ok()
}

/// 密码框、只读与禁用控件一律不回填。
///
/// 只读只能从 `ValuePattern` 上取：`IUIAutomationElement` 没有 `CurrentIsReadOnly`，
/// 元素级的只读属性是 `UIA_ValueIsReadOnlyPropertyId`，读它要走 `GetCurrentPropertyValue`，
/// 需要额外开启 `Win32_System_Ole` 并处理 `VARIANT`，为这一个判断不值得。
fn is_editable(element: &IUIAutomationElement, pattern: &IUIAutomationValuePattern) -> bool {
    let enabled = unsafe { element.CurrentIsEnabled() }
        .map(|flag| flag.as_bool())
        .unwrap_or(true);
    if !enabled {
        return false;
    }
    let password = unsafe { element.CurrentIsPassword() }
        .map(|flag| flag.as_bool())
        .unwrap_or(false);
    if password {
        return false;
    }
    // 老 provider 会在这里返回 E_NOTIMPL，读不到时按"可写"处理：让一个坏 provider 把整个
    // 回填能力关掉，比偶尔多看一眼只读框更糟。
    !unsafe { pattern.CurrentIsReadOnly() }
        .map(|flag| flag.as_bool())
        .unwrap_or(false)
}

fn read_value(pattern: &IUIAutomationValuePattern) -> Option<String> {
    let value = unsafe { pattern.CurrentValue() }.ok()?;
    Some(value.to_string())
}

/// 占位提示的最后一道兜底。
///
/// Chromium 在控件没有 label / aria-label 时会把 `placeholder` 当作可访问名，个别 provider
/// 还会塞进帮助文字。取到的值（空白归一后非空）与其中之一相同，就当作提示而非正文。
///
/// 比较前做空白归一：多行输入框的换行与缩进会随 DOM 结构变化，逐字符比较会漏判。
fn is_placeholder_hint(element: &IUIAutomationElement, text: &str) -> bool {
    let text = normalize(text);
    if text.is_empty() {
        return false;
    }
    let name = unsafe { element.CurrentName() }
        .map(|value| value.to_string())
        .unwrap_or_default();
    let name = normalize(&name);
    if !name.is_empty() && text == name {
        return true;
    }
    let help = unsafe { element.CurrentHelpText() }
        .map(|value| value.to_string())
        .unwrap_or_default();
    let help = normalize(&help);
    !help.is_empty() && text == help
}

/// 把连续空白压成一个空格并去掉首尾，用于比较提示词与正文。
fn normalize(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn truncate(text: String) -> String {
    if text.chars().count() <= MAX_FOCUS_TEXT {
        return text;
    }
    text.chars().take(MAX_FOCUS_TEXT).collect()
}

thread_local! {
    /// 线程内复用的 UIA 根对象。
    ///
    /// [`has_focus_input`] 由焦点轮询线程以 300ms 的节奏调用，而 `CoCreateInstance`
    /// 比读两三个属性贵一个数量级；缓存一次就够了。也正因为是 `thread_local`，
    /// 调用方必须固定在同一个线程上循环调用。
    static SHARED_AUTOMATION: Option<IUIAutomation> = {
        if ComApartment::init().is_err() {
            None
        } else {
            unsafe { CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER) }.ok()
        }
    };
}

/// 焦点是否落在可写的输入框里——只看能力，**从不读取内容**。
///
/// 判据不看控件类型，而是看焦点元素有没有**可写的 `ValuePattern`**：经典 `Edit`、
/// 记事本的 `RichEdit`、浏览器地址栏与网页 `<input>` 都有它；而 `Document`（网页正文、
/// 富文本、PDF）与下拉框没有，所以点一下网页空白不会把手机键盘弹出来。密码框同样有
/// `ValuePattern`，因此**密码框也算"在输入框里"**——这里从不取值，所以不算读密码。
///
/// 读不到一律返回 `false`：焦点不在输入控件、控件不支持 UIA（游戏、自绘界面）、
/// 或这张桌面本来就取不到焦点，都属于常态，不是错误。
pub fn has_focus_input() -> bool {
    SHARED_AUTOMATION.with(|cached| match cached {
        Some(automation) => has_focus_input_with(automation),
        None => false,
    })
}

fn has_focus_input_with(automation: &IUIAutomation) -> bool {
    let Ok(element) = (unsafe { automation.GetFocusedElement() }) else {
        return false;
    };
    let Ok(pattern) =
        (unsafe { element.GetCurrentPatternAs::<IUIAutomationValuePattern>(UIA_ValuePatternId) })
    else {
        return false;
    };
    !unsafe { pattern.CurrentIsReadOnly() }
        .map(|flag| flag.as_bool())
        .unwrap_or(false)
}
