//! Windows 服务的日志初始化。

#[cfg(target_os = "windows")]
pub fn init_service_logging() {
    let log_dir = std::env::var_os("PROGRAMDATA")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir())
        .join("QingLink\\logs");
    if std::fs::create_dir_all(&log_dir).is_ok() {
        let file_appender = tracing_appender::rolling::daily(log_dir, "QingLinkService.log");
        let (writer, guard) = tracing_appender::non_blocking(file_appender);
        let _ = tracing_subscriber::fmt()
            .with_ansi(false)
            .with_writer(writer)
            .with_env_filter("qinglink=info")
            .try_init();
        Box::leak(Box::new(guard));
    }
}
