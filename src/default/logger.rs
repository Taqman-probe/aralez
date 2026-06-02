use default_interface::{ApplicationLogHandle, LoggerModule};
use std::path::{Path, PathBuf};
use tracing::level_filters::LevelFilter;

pub struct ApplicationLogger {
    pub level: LevelFilter,
    pub path: Option<PathBuf>,
    pub _config: Option<String>
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
            _config: config,
        }
    }

    fn init (self) -> ApplicationLogHandle{
        match self.path {
            Some(p) => {
                let appender = tracing_appender::rolling::never(
                    p.parent().unwrap_or_else(|| Path::new(".")),
                    p.file_name().unwrap(),
                );

                tracing_subscriber::fmt()
                    .with_writer(appender)
                    .with_ansi(false)
                    .with_timer(tracing_subscriber::fmt::time::ChronoLocal::new("%Y-%m-%d %H:%M:%S".to_string()))
                    .with_target(true)
                    .with_max_level(self.level)
                    .init();
                ApplicationLogHandle::empty()
            },
            None => {
                tracing_subscriber::fmt()
                    .with_writer(std::io::stdout)
                    .with_timer(tracing_subscriber::fmt::time::ChronoLocal::new("%Y-%m-%d %H:%M:%S".to_string()))
                    .with_target(true)
                    .with_max_level(self.level)
                    .init();
                ApplicationLogHandle::empty()
            }
        }
    }
}
