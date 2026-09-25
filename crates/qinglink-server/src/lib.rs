use std::sync::{Mutex, MutexGuard};

pub mod custom_dir;
pub mod platform;
pub mod protocol;
pub mod server;
pub mod service;
pub mod steam;

pub use server::{AppState, RemoteServer};
pub use service::RemoteControlService;

/// 锁定互斥锁，并在中毒状态下通过获取内部值来恢复。
pub(crate) fn lock_unpoisoned<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}
