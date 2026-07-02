use default_interface::{ApplicationLogHandle, LoggerModule};
use log::LevelFilter;
use log4rs::{
    append::console::ConsoleAppender,
    append::rolling_file::{
        policy::compound::{
            CompoundPolicy,
            roll::fixed_window::FixedWindowRoller,
            trigger::size::SizeTrigger,
        },
        RollingFileAppender,
    },
    config::{Appender, Config as Log4rsConfig, Root},
    encode::pattern::PatternEncoder,
};
use std::path::PathBuf;


pub struct ApplicationLogger {
    level: LevelFilter,
    path: Option<PathBuf>,
    config: Option<String>
}

impl LoggerModule for ApplicationLogger {
    fn new(level_str: &str, location: Option<String>, config: Option<String>) -> Self {
        let log_level = match level_str.to_lowercase().as_str() {
            "info"  => LevelFilter::Info,
            "error" => LevelFilter::Error,
            "warn"  => LevelFilter::Warn,
            "debug" => LevelFilter::Debug,
            "trace" => LevelFilter::Trace,
            "off"   => LevelFilter::Off,
            _ => {
                println!("Error reading log level, defaulting to: INFO");
                LevelFilter::Info
            }
        };
        Self {
            level: log_level,
            path: location.as_ref().map(PathBuf::from),
            config: config
        }
    }

    fn init (self) -> ApplicationLogHandle {
        let pattern = "{d(%Y-%m-%d %H:%M:%S%.3f)} {l} {t} - {m}\n";
        match self.path {
            Some(p) => {
                // ファイルサイズローテーション
                let size_mb: i16 = self.config.as_deref().and_then(|s| s.parse().ok()).unwrap_or(200);
                let max_size_bytes = size_mb as u64 * 1024 * 1024;
                let trigger = SizeTrigger::new(max_size_bytes);
                let roller = FixedWindowRoller::builder()
                    .base(1) // ログファイル名.1 から開始
                    .build(&format!("{}.{{}}", p.display()), 5) // 最大5番（5世代）まで保持
                    .expect("Failed to build roller");

                let policy = CompoundPolicy::new(Box::new(trigger), Box::new(roller));

                let file_appender = RollingFileAppender::builder()
                    .encoder(Box::new(PatternEncoder::new(pattern)))
                    .build(p, Box::new(policy))
                    .expect("Failed initialize of Log file.");

                let config = Log4rsConfig::builder()
                    .appender(Appender::builder().build("file", Box::new(file_appender)))
                    .build(Root::builder().appender("file").build(self.level))
                    .unwrap();
                let handle =log4rs::init_config(config).unwrap();
                ApplicationLogHandle::new(handle)
            },
            None => {
                let stdout = ConsoleAppender::builder().encoder(Box::new(PatternEncoder::new(pattern))).build();
                let config = Log4rsConfig::builder()
                    .appender(Appender::builder().build("stdout", Box::new(stdout)))
                    .build(Root::builder().appender("stdout").build(self.level))
                    .unwrap();
                let handle = log4rs::init_config(config).unwrap();
                ApplicationLogHandle::new(handle)
            }
        }
    }
}
