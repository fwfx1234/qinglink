//! 把日志记录采集进环形缓冲区与实时流的 `tracing` 层。

use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::sync::broadcast;
use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::{Context, Layer};

use super::buffer::LogBuffer;
use super::entry::{LogEntry, LogLevel};

/// Capacity of the live stream. Subscribers that fall behind are told how many
/// records they missed rather than being allowed to slow the emitter down.
const STREAM_CAPACITY: usize = 1024;

thread_local! {
    /// Resolved once per thread; `tracing` events are emitted far too often to
    /// re-query the thread name each time.
    static THREAD_NAME: Option<String> =
        std::thread::current().name().map(str::to_string);
}

/// Captures `tracing` events into the in-memory sinks.
///
/// Cloning is cheap and shares the id counter, so the layer installed in the
/// subscriber and the handle kept by [`super::LogSystem`] stay in lockstep.
#[derive(Clone, Debug)]
pub struct LogCollector {
    buffer: Arc<LogBuffer>,
    stream: broadcast::Sender<Arc<LogEntry>>,
    next_id: Arc<AtomicU64>,
}

impl LogCollector {
    /// Build a collector over the given buffer.
    pub fn new(buffer: Arc<LogBuffer>) -> Self {
        let (stream, _) = broadcast::channel(STREAM_CAPACITY);
        Self {
            buffer,
            stream,
            next_id: Arc::new(AtomicU64::new(1)),
        }
    }

    /// Subscribe to live records. Only records emitted after this call arrive.
    pub fn subscribe(&self) -> broadcast::Receiver<Arc<LogEntry>> {
        self.stream.subscribe()
    }
}

impl<S> Layer<S> for LogCollector
where
    S: Subscriber + for<'lookup> tracing_subscriber::registry::LookupSpan<'lookup>,
{
    fn on_event(&self, event: &Event<'_>, _context: Context<'_, S>) {
        let metadata = event.metadata();
        let mut visitor = FieldVisitor::default();
        event.record(&mut visitor);

        let thread_name = THREAD_NAME.with(|name| name.clone());
        let entry = LogEntry::new(
            self.next_id.fetch_add(1, Ordering::Relaxed),
            now_ms(),
            LogLevel::from_tracing(metadata.level()),
            metadata.target().to_string(),
            visitor.finish(),
        )
        .with_thread(thread_name)
        .with_location(metadata.file().map(str::to_string), metadata.line());

        // Memory first: the interface must see a record even if the disk is full
        // or the log file could not be opened.
        self.buffer.push(entry.clone());
        // A send error only means nobody is subscribed.
        let _ = self.stream.send(Arc::new(entry));
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as i64)
        .unwrap_or_default()
}

/// Flattens an event's fields into `message` plus trailing `key=value` pairs.
#[derive(Default)]
struct FieldVisitor {
    message: String,
    extra: Vec<String>,
}

impl FieldVisitor {
    fn finish(self) -> String {
        if self.extra.is_empty() {
            return self.message;
        }
        let extra = self.extra.join(" ");
        if self.message.is_empty() {
            extra
        } else {
            format!("{} {}", self.message, extra)
        }
    }

    fn push(&mut self, field: &Field, value: String) {
        if field.name() == "message" {
            self.message = value;
        } else {
            self.extra.push(format!("{}={}", field.name(), value));
        }
    }
}

impl Visit for FieldVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.push(field, format!("{value:?}"));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.push(field, value.to_string());
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.push(field, value.to_string());
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.push(field, value.to_string());
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.push(field, value.to_string());
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        self.push(field, value.to_string());
    }

    fn record_error(&mut self, field: &Field, value: &(dyn std::error::Error + 'static)) {
        self.push(field, value.to_string());
    }
}
