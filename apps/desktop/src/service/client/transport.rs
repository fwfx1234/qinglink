//! 阻塞式传输抽象与真实命名管道实现。

use std::sync::atomic::AtomicBool;

use qinglink_protocol::service_ipc::{DesktopRequest, ServiceResponse};

use crate::service::{PIPE_NAME, REQUEST_TIMEOUT, ServiceError};

/// Blocking transport, isolated so tests can substitute a fake.
pub(super) trait Transport: Send + 'static {
    fn request(
        &mut self,
        request: &DesktopRequest,
        stop: &AtomicBool,
    ) -> Result<ServiceResponse, ServiceError>;
}

/// The real transport: one authorized named-pipe round trip.
#[derive(Debug)]
pub(super) struct PipeTransport;

impl Transport for PipeTransport {
    fn request(
        &mut self,
        request: &DesktopRequest,
        stop: &AtomicBool,
    ) -> Result<ServiceResponse, ServiceError> {
        qinglink_windows::ipc::request(PIPE_NAME, request, REQUEST_TIMEOUT, stop).map_err(|error| {
            // Keep the whole chain: the outer context is always "cannot connect
            // to local pipe" or "cannot receive pipe response", while only the
            // innermost cause identifies whether the pipe is missing, busy, or
            // carrying an unusable payload. Collapsing it hides the answer.
            ServiceError::Unavailable(format!("{error:#}"))
        })
    }
}
