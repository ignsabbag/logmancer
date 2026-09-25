use std::io;
use std::path::Path;
use std::sync::OnceLock;
use tracing_subscriber::Layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub const MAX_LOG_FILES: usize = 7;

static LOG_GUARD: OnceLock<tracing_appender::non_blocking::WorkerGuard> = OnceLock::new();
static INITIALIZED: OnceLock<()> = OnceLock::new();

pub fn init_file_logging(
    directory: &Path,
    variant: &str,
    stderr_is_terminal: bool,
) -> io::Result<()> {
    init_file_logging_with_name(
        directory,
        &format!("logmancer-{variant}.log"),
        stderr_is_terminal,
    )
}

pub fn init_file_logging_with_name(
    directory: &Path,
    file_name: &str,
    stderr_is_terminal: bool,
) -> io::Result<()> {
    if INITIALIZED.get().is_some() {
        return Ok(());
    }

    std::fs::create_dir_all(directory)?;

    let (prefix, suffix) = match file_name.rsplit_once('.') {
        Some((stem, extension)) if !stem.is_empty() && !extension.is_empty() => {
            (stem, Some(extension))
        }
        _ => (file_name, None),
    };
    let mut builder = tracing_appender::rolling::RollingFileAppender::builder()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix(prefix);
    if let Some(suffix) = suffix {
        builder = builder.filename_suffix(suffix);
    }
    let file_appender = builder
        .max_log_files(MAX_LOG_FILES)
        .build(directory)
        .map_err(io::Error::other)?;

    if tracing::dispatcher::has_been_set() {
        return Ok(());
    }

    let (writer, guard) = tracing_appender::non_blocking(file_appender);
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    let file_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_target(false)
        .with_writer(writer)
        .with_filter(env_filter.clone());

    let subscriber = tracing_subscriber::registry().with(file_layer);
    if stderr_is_terminal {
        subscriber
            .with(
                tracing_subscriber::fmt::layer()
                    .with_target(false)
                    .with_writer(std::io::stderr)
                    .with_filter(env_filter)
                    .with_filter(tracing_subscriber::filter::filter_fn(|metadata| {
                        *metadata.level() == tracing::Level::ERROR
                    })),
            )
            .try_init()
            .map_err(io::Error::other)?;
    } else {
        subscriber.try_init().map_err(io::Error::other)?;
    }

    let _ = tracing_log::LogTracer::init();
    let _ = LOG_GUARD.set(guard);
    let _ = INITIALIZED.set(());
    Ok(())
}
