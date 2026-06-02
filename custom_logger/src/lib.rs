use default_interface::{ApplicationLogHandle, LoggerModule};
use std::path::PathBuf;
use tracing::level_filters::LevelFilter;

pub struct ApplicationLogger {
    pub level: LevelFilter,
    pub path: Option<PathBuf>,
    pub config: Option<String>
}

impl LoggerModule for ApplicationLogger {
    fn new(_level_str: &str, _location: &Option<String>, _config: Option<String>) -> Self {
        todo!("Please implement your own logic here, referencing default/logger.rs.")
    }

    fn init (self) -> ApplicationLogHandle{
        todo!("Please implement your own logic here, referencing default/logger.rs.")
    }
}