use default_interface::{ApplicationLogHandle, LoggerModule};
use log::LevelFilter;
use log4rs::{
    append::{console::ConsoleAppender, file::FileAppender},
    config::{Appender, Config as Log4rsConfig, Root},
    encode::pattern::PatternEncoder,
};

pub struct ApplicationLogger {
    pub level: LevelFilter,
    pub location: Option<String>,
    pub _config: Option<String>
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
            location: location,
            _config: config,
        }
    }

    fn init (self) -> ApplicationLogHandle{
        let pattern = "{d(%Y-%m-%d %H:%M:%S)} {l} {t} - {m}\n";
        match self.location {
            Some(p) => {
                let file = FileAppender::builder().encoder(Box::new(PatternEncoder::new(pattern))).build(&p).unwrap();
                let config = Log4rsConfig::builder()
                    .appender(Appender::builder().build("file", Box::new(file)))
                    .build(Root::builder().appender("file").build(self.level))
                    .unwrap();
                log4rs::init_config(config).unwrap();
                ApplicationLogHandle::empty()
            },
            None => {
                let stdout = ConsoleAppender::builder().encoder(Box::new(PatternEncoder::new(pattern))).build();
                let config = Log4rsConfig::builder()
                    .appender(Appender::builder().build("stdout", Box::new(stdout)))
                    .build(Root::builder().appender("stdout").build(self.level))
                    .unwrap();
                log4rs::init_config(config).unwrap();
                ApplicationLogHandle::empty()
            }
        }
    }
}
