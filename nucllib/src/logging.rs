use std::fs;
use std::path::Path;
use tracing::info;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{filter::EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// Initializes the tracing subscriber.
/// MUST bind the returned `WorkerGuard` to a variable in `main()` to ensure logs flush on exit.
pub fn init_logger(app_name: &str) -> Option<WorkerGuard> {
    let debug_mode = std::env::var("NUCLINIT_DEBUG")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);

    // Default to verbose trace in debug mode, info in standard mode.
    let filter = EnvFilter::from_default_env().add_directive(if debug_mode {
        "nucl=trace".parse().unwrap()
    } else {
        "nucl=info".parse().unwrap()
    });

    if debug_mode {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().with_writer(std::io::stdout))
            .init();

        info!("Started {} in DEBUG mode (stdout)", app_name);
        None
    } else {
        let log_dir = Path::new("/run/log/nuclinit");
        if let Err(e) = fs::create_dir_all(log_dir) {
            eprintln!(
                "CRITICAL: Failed to create log directory at {:?}: {}",
                log_dir, e
            );
        }

        let file_appender = tracing_appender::rolling::never(log_dir, format!("{}.log", app_name));
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().with_writer(non_blocking))
            .init();

        info!("Started {} in PRODUCTION mode (logging to disk)", app_name);
        Some(guard)
    }
}
