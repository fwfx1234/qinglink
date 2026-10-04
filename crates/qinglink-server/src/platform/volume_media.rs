//! 系统音量与媒体播放控制。

use super::platform_impl;

use crate::protocol::responses::VolumeStatus;

// === 音量 & 媒体控制 ===

/// 获取当前音量百分比与静音状态。
///
/// # Errors
///
/// 当初始化 COM、获取默认渲染端点或读取主音量/静音状态失败时返回 `Err`；
/// 非 Windows 平台恒返回不支持错误。
pub fn get_volume() -> anyhow::Result<VolumeStatus> {
    platform_impl::get_volume().map(|value| VolumeStatus {
        percent: value.percent,
        muted: value.muted,
    })
}

/// 枚举系统中的音频输出设备。
///
/// # Errors
///
/// 当初始化 COM、创建音频设备枚举器或枚举/读取端点信息失败时返回 `Err`；
/// 非 Windows 平台恒返回不支持错误。
pub fn get_audio_outputs() -> anyhow::Result<crate::protocol::responses::AudioOutputsResponse> {
    platform_impl::get_audio_outputs().map(|value| {
        crate::protocol::responses::AudioOutputsResponse {
            devices: value
                .devices
                .into_iter()
                .map(|device| crate::protocol::responses::AudioOutputDevice {
                    id: device.id,
                    name: device.name,
                    is_default: device.is_default,
                })
                .collect(),
        }
    })
}

/// 将默认音频输出切换到指定设备。
///
/// # Errors
///
/// 当设备 ID 为空、设备不在活动输出列表中、初始化 COM 或设置默认端点失败时
/// 返回 `Err`；非 Windows 平台恒返回不支持错误。
pub fn set_audio_output(device_id: &str) -> anyhow::Result<()> {
    platform_impl::set_audio_output(device_id)
}

/// 将系统音量设置为指定百分比。
///
/// # Errors
///
/// 当初始化 COM、获取默认渲染端点或设置主音量失败时返回 `Err`；
/// 非 Windows 平台恒返回不支持错误。
pub fn set_volume(percent: u8) -> anyhow::Result<()> {
    platform_impl::set_volume(percent)
}

/// 设置或取消系统静音。
///
/// # Errors
///
/// 当初始化 COM、获取默认渲染端点或设置静音状态失败时返回 `Err`；
/// 非 Windows 平台恒返回不支持错误。
pub fn set_muted(muted: bool) -> anyhow::Result<()> {
    platform_impl::set_muted(muted)
}

/// 调高一级系统音量。
///
/// # Errors
///
/// 当初始化 COM、获取默认渲染端点或提高音量步进失败时返回 `Err`；
/// 非 Windows 平台恒返回不支持错误。
pub fn volume_up() -> anyhow::Result<()> {
    platform_impl::volume_up()
}

/// 调低一级系统音量。
///
/// # Errors
///
/// 当初始化 COM、获取默认渲染端点或降低音量步进失败时返回 `Err`；
/// 非 Windows 平台恒返回不支持错误。
pub fn volume_down() -> anyhow::Result<()> {
    platform_impl::volume_down()
}

/// 切换系统静音状态。
///
/// # Errors
///
/// 当初始化 COM、获取默认渲染端点或读取/切换静音状态失败时返回 `Err`；
/// 非 Windows 平台恒返回不支持错误。
pub fn volume_mute() -> anyhow::Result<()> {
    platform_impl::volume_mute()
}

/// 切换当前媒体会话的播放与暂停。
///
/// # Errors
///
/// 当前实现恒返回 `Ok(())`：媒体键通过 `SendInput` 模拟发送且不检查返回值；
/// 返回值保留是为了统一平台抽象层的签名。非 Windows 平台恒返回不支持错误。
pub fn media_play_pause() -> anyhow::Result<()> {
    platform_impl::media_play_pause()
}

/// 停止当前媒体会话的播放。
///
/// # Errors
///
/// 当前实现恒返回 `Ok(())`：媒体键通过 `SendInput` 模拟发送且不检查返回值；
/// 返回值保留是为了统一平台抽象层的签名。非 Windows 平台恒返回不支持错误。
pub fn media_stop() -> anyhow::Result<()> {
    platform_impl::media_stop()
}

/// 切换到下一首媒体曲目。
///
/// # Errors
///
/// 当前实现恒返回 `Ok(())`：媒体键通过 `SendInput` 模拟发送且不检查返回值；
/// 返回值保留是为了统一平台抽象层的签名。非 Windows 平台恒返回不支持错误。
pub fn media_next() -> anyhow::Result<()> {
    platform_impl::media_next()
}

/// 切换到上一首媒体曲目。
///
/// # Errors
///
/// 当前实现恒返回 `Ok(())`：媒体键通过 `SendInput` 模拟发送且不检查返回值；
/// 返回值保留是为了统一平台抽象层的签名。非 Windows 平台恒返回不支持错误。
pub fn media_prev() -> anyhow::Result<()> {
    platform_impl::media_prev()
}
