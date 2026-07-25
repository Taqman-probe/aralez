use std::net::IpAddr;
use std::sync::Arc;

#[derive(Debug, Clone, serde::Serialize)]
pub struct LogMessage {
    pub response_code: u16,
    pub summary: String,
    pub client_ip: IpAddr,
    pub version: String,
    pub user_agent: String,
    pub cache_status: String,
}

pub trait AralezPluginLogger: Send + Sync {
    fn pre_should_log(&self, _response_code: u16) -> bool {
        true
    }

    fn should_log(&self, _msg: &LogMessage) -> bool {
        true
    }

    fn format_access_log(&self, msg: &LogMessage) -> String;
}

pub type LoggerFactory = fn(
    log_level: &str,
    file_location: Option<String>,
    access_level: &str,
    option: Option<noyalib::Value>,
) -> Result<Arc<dyn AralezPluginLogger>, Box<dyn std::error::Error>>;

pub struct LoggerPluginEntry {
    pub create: LoggerFactory,
}

inventory::collect!(LoggerPluginEntry);
