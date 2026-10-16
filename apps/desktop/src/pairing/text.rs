//! 弹窗文案：只做格式化，方便单独阅读与修改。

use time::OffsetDateTime;

/// 第一行：谁在请求配对。
pub(super) fn device_line(device_name: &str) -> String {
    let name = device_name.trim();
    if name.is_empty() {
        "有设备请求配对本机".to_string()
    } else {
        format!("「{name}」请求配对本机")
    }
}

/// 距离过期的剩余秒数；已经过期时为 0。
pub(super) fn remaining_seconds(expires_at_unix: i64) -> i64 {
    (expires_at_unix - OffsetDateTime::now_utc().unix_timestamp()).max(0)
}

/// 倒计时行；配合剩余秒数一起使用。
pub(super) fn countdown_label(remaining: i64) -> String {
    if remaining <= 0 {
        "配对码已过期".to_string()
    } else {
        format!("{remaining} 秒后过期")
    }
}
