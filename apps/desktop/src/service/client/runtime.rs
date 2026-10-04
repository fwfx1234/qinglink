//! worker 线程的生命周期：启动、停机、退出时释放队列。

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use tokio::sync::broadcast;

use super::ServiceState;
use super::handle::ServiceClient;
use super::inner::{Inner, QUEUE_CAPACITY};
use super::transport::PipeTransport;
use super::worker::worker_loop;

/// Convenience so `status()` never panics on a poisoned lock.
/// Owns the worker thread.
#[derive(Debug)]
pub struct ServiceRuntime {
    client: ServiceClient,
    worker: Option<JoinHandle<()>>,
    stop: Arc<AtomicBool>,
}

impl ServiceRuntime {
    /// Start the worker thread.
    pub fn start() -> Self {
        let (jobs, receiver) = std::sync::mpsc::sync_channel(QUEUE_CAPACITY);
        let (events, _) = broadcast::channel(64);
        let stop = Arc::new(AtomicBool::new(false));
        let inner = Arc::new(Inner {
            state: Mutex::new(ServiceState::default()),
            jobs,
            stop: Arc::clone(&stop),
            wake: AtomicU64::new(0),
            events,
            next_request: AtomicU64::new(1),
        });
        let worker_inner = Arc::clone(&inner);
        let worker = thread::Builder::new()
            .name("qinglink-service-ipc".into())
            .spawn(move || worker_loop(&worker_inner, &receiver, PipeTransport))
            .ok();
        if worker.is_none() {
            tracing::error!("cannot start the service IPC thread");
        }
        Self {
            client: ServiceClient::new(inner),
            worker,
            stop,
        }
    }

    /// The client handle handed to the web layer.
    pub fn client(&self) -> ServiceClient {
        self.client.clone()
    }

    /// Stop the worker and wait for it to finish.
    pub fn shutdown(mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take()
            && worker.join().is_err()
        {
            tracing::error!("the service IPC thread panicked");
        }
    }
}

impl Drop for ServiceRuntime {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take()
            && worker.join().is_err()
        {
            tracing::error!("the service IPC thread panicked");
        }
    }
}
