//! 物理鼠标按键状态读取。
//!
//! 用 `GetAsyncKeyState` 的最高位判断"此刻是否按下"。最低位表示"自本线程上次查询以来
//! 是否被按过"，语义依赖同线程历史，这里刻意不用。
//!
//! 注意：`SendInput` 注入的按下同样会反映在异步键状态里，所以远端拖拽时 PC 上物理
//! 鼠标的状态与注入状态是同一份事实，手机端不需要自己再合并。

use qinglink_protocol::MouseButtonsState;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VK_LBUTTON, VK_MBUTTON, VK_RBUTTON,
};

/// 读取左右中三键的当前按下状态。
pub fn pressed_buttons() -> MouseButtonsState {
    MouseButtonsState {
        left: is_down(VK_LBUTTON.0),
        right: is_down(VK_RBUTTON.0),
        middle: is_down(VK_MBUTTON.0),
    }
}

fn is_down(key: u16) -> bool {
    // 高字节非 0 表示当前处于按下状态。
    unsafe { GetAsyncKeyState(i32::from(key)) < 0 }
}
