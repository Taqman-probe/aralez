use default_interface::{ApplicationLogHandle, LoggerModule};
use rolling_file::{BasicRollingFileAppender, RollingConditionBasic};
use std::path::PathBuf;
use tracing_appender::non_blocking;
use tracing::level_filters::LevelFilter;

pub struct ApplicationLogger {
    level: LevelFilter,
    path: Option<PathBuf>,
    config: Option<String>
}

impl LoggerModule for ApplicationLogger {
    fn new(level_str: &str, location: &Option<String>, config: Option<String>) -> Self {
        let log_level = match level_str.to_lowercase().as_str() {
            "info"  => LevelFilter::INFO,
            "error" => LevelFilter::ERROR,
            "warn"  => LevelFilter::WARN,
            "debug" => LevelFilter::DEBUG,
            "trace" => LevelFilter::TRACE,
            "off"   => LevelFilter::OFF,
            _ => {
                println!("Error reading log level, defaulting to: INFO");
                LevelFilter::INFO
            }
        };
        Self {
            level: log_level,
            path: location.as_ref().map(PathBuf::from),
            config: config
        }
    }

    fn init (self) -> ApplicationLogHandle {
        match self.path {
            Some(p) => {
                let size_mb: i16 = self.config.as_deref().and_then(|s| s.parse().ok()).unwrap_or(200);
                let appender = BasicRollingFileAppender::new(
                    p,
                    RollingConditionBasic::new().max_size(size_mb as u64 * 1024 * 1024),
                    5,
                )
                .expect("Failed initialize of Log file.");
                let (non_blocking_file, guard) = non_blocking(appender);

                tracing_subscriber::fmt()
                    .with_writer(non_blocking_file)
                    .with_ansi(false) 
                    .with_timer(tracing_subscriber::fmt::time::ChronoLocal::new("%Y-%m-%d %H:%M:%S%.3f".to_string()))
                    .with_target(true)
                    .with_max_level(self.level)
                    .init();
                ApplicationLogHandle::new(guard)
            },
            None => {
                tracing_subscriber::fmt()
                    .with_writer(std::io::stdout)
                    .with_timer(tracing_subscriber::fmt::time::ChronoLocal::new("%Y-%m-%d %H:%M:%S%.3f".to_string()))
                    .with_target(true)
                    .with_max_level(self.level)
                    .init();
                ApplicationLogHandle::empty()
            }
        }
    }
}