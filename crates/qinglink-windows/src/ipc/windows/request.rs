//! 一次请求-应答往返：编码、连接、发送请求帧、接收应答帧。

use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use serde::{Serialize, de::DeserializeOwned};

use super::connect::connect;
use super::pipe_path::local_pipe_path;
use super::win32::check_budget;
use crate::ipc::codec;

/// 新建一条本地管道连接，发送一个请求帧并反序列化一个应答帧。
pub(in crate::ipc) fn request<T: DeserializeOwned>(
    path: &str,
    request: &impl Serialize,
    timeout: Duration,
    cancelled: &AtomicBool,
) -> Result<T> {
    let deadline = Instant::now()
        .checked_add(timeout)
        .context("pipe request timeout is too large")?;
    check_budget(deadline, cancelled).context("cannot start pipe request")?;
    let path = local_pipe_path(path)?;
    let payload = codec::serialize(request).context("cannot encode pipe request")?;
    check_budget(deadline, cancelled).context("pipe request budget exhausted while encoding")?;
    let mut pipe = connect(&path, deadline, cancelled).context("cannot connect to local pipe")?;
    pipe.write_frame(&payload, deadline, cancelled)
        .context("cannot send pipe request")?;
    pipe.read_json(deadline, cancelled)
        .context("cannot receive pipe response")
}
