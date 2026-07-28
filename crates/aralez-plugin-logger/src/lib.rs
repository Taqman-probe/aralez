use aralez_spec::{AralezPluginLogger, LogMessage, LoggerPluginEntry};
use log::{info, LevelFilter};
use log4rs::{
    append::{
        console::ConsoleAppender,
        rolling_file::{
            policy::compound::{
                roll::fixed_window::FixedWindowRoller,
                trigger::size::SizeTrigger,
                CompoundPolicy,
            },
            RollingFileAppender,
        },
    },
    config::{Appender, Config as Log4rsConfig, Root},
    encode::pattern::PatternEncoder,
};
use std::sync::Arc;

pub fn init() {
}

#[derive(Debug, Clone)]
pub enum LogLevel {
    Access,
    Error,
    None,
}

pub struct LoggerPlugin {
    access_log_level: LogLevel,
}

#[derive(serde::Deserialize)]
pub struct RollingRule {
    compress: String,
    size_mb: u64,
    keep: u32,
}

impl LoggerPlugin {
    pub fn initialize(
        log_level: &str,
        file_location: Option<String>,
        access_level_str: &str,
        option: Option<noyalib::Value>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let level_filter = match log_level {
            "info" => LevelFilter::Info,
            "error" => LevelFilter::Error,
            "warn" => LevelFilter::Warn,
            "debug" => LevelFilter::Debug,
            "trace" => LevelFilter::Trace,
            "off" => LevelFilter::Off,
            _ => {
                println!("Error reading log level, defaulting to: INFO");
                LevelFilter::Info
            }
        };

        let access_log_level = match access_level_str {
            "all" => LogLevel::Access,
            "error" => LogLevel::Error,
            _ => LogLevel::None,
        };

        let rolling_rule: Option<RollingRule> = match option {
            Some(rule) => noyalib::from_value(&rule).ok(),
            None => None,
        };

        let pattern = "{d(%Y-%m-%d %H:%M:%S)} {l} {t} - {m}\n";

        if let Some(ref location) = file_location {
            let compress = rolling_rule
                .as_ref()
                .map(|r| r.compress.as_str())
                .unwrap_or("No");

            let size_mb: u64 = rolling_rule.as_ref().map(|r| r.size_mb).unwrap_or(100);
            let keep: u32 = rolling_rule.as_ref().map(|r| r.keep).unwrap_or(5);

            let pattern_str = match compress {
                "compress" => format!("{}.{{}}.gz", location),
                _ => format!("{}.{{}}", location),
            };

            let roller = FixedWindowRoller::builder().build(pattern_str.as_str(), keep)?;
            let trigger = SizeTrigger::new(size_mb * 1024 * 1024);
            let policy = CompoundPolicy::new(Box::new(trigger), Box::new(roller));

            let file = RollingFileAppender::builder()
                .encoder(Box::new(PatternEncoder::new(pattern)))
                .build(location, Box::new(policy)).unwrap();

            let config = Log4rsConfig::builder()
                .appender(Appender::builder().build("file", Box::new(file)))
                .build(Root::builder().appender("file").build(level_filter))?;

            log4rs::init_config(config).unwrap();

            info!(
                "Logging to: {}, Max file size: {}mb, Files to keep: {}, compression: {}",
                location, size_mb, keep, compress
            );
        } else {
            let stdout = ConsoleAppender::builder()
                .encoder(Box::new(PatternEncoder::new(pattern)))
                .build();

            let config = Log4rsConfig::builder()
                .appender(Appender::builder().build("stdout", Box::new(stdout)))
                .build(Root::builder().appender("stdout").build(level_filter))?;

            log4rs::init_config(config).unwrap();

            info!("No files are configured, logging to stdout");
        }

        Ok(Self { access_log_level })
    }
}

fn create_logger_plugin(
    log_level: &str,
    file_location: Option<String>,
    access_level: &str,
    option: Option<noyalib::Value>,
) -> Result<Arc<dyn AralezPluginLogger>, Box<dyn std::error::Error>> {
    let plugin = LoggerPlugin::initialize(
        log_level,
        file_location,
        access_level,
        option,
    )?;

    Ok(Arc::new(plugin))
}

impl AralezPluginLogger for LoggerPlugin {
    fn pre_should_log(&self, response_code: u16) -> bool {
        match self.access_log_level {
            LogLevel::Access => true,
            LogLevel::None => false,
            LogLevel::Error => !(100..=399).contains(&response_code),
        }
    }

    fn format_access_log(&self, msg: &LogMessage) -> String {
        format!(
            "{}, {}, {}, client: {}, version: {}, useragent: {}",
            msg.response_code,
            msg.cache_status,
            msg.summary,
            msg.client_ip,
            msg.version,
            msg.user_agent,
        )
    }
}

inventory::submit! {
    LoggerPluginEntry {
        create: create_logger_plugin,
    }
}
