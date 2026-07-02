use http::HeaderValue;
use http::header::HeaderMap;
use pingora_http::Version;
use serde_json::Value;
use std::any::Any;
use std::net::IpAddr;
use std::sync::Arc;
use tokio::time::Instant;

pub struct ApplicationLogHandle {
    _inner: Option<Box<dyn Any + Send + Sync>>
}

impl ApplicationLogHandle {
    pub fn new<T: Send + Sync + 'static>(handle: T) -> Self {
        Self {
            _inner: Some(Box::new(handle)),
        }
    }

    pub fn empty() -> Self {
        Self { _inner: None }
    }
}

pub trait LoggerModule {
    fn new(level_str: &str, path: Option<String>, config: Option<String>) -> Self;
    fn init(self) -> ApplicationLogHandle;
}

pub trait AccessLog {
    fn info(msg: LogMessage);
}

#[derive(Debug)]
pub struct LogMessage {
    pub response_code: u16,
    pub summary: String,
    pub client_ip: IpAddr,
    pub version: Version,
    pub headers: HeaderMap<HeaderValue>,
    pub matched_path: Option<Arc<String>>,
    pub backend_id: Option<String>,
    pub start_time: Instant,
    pub upstream_peer: Option<Value>,
    pub client_headers: Option<Vec<(String, Arc<str>)>>,
    pub x4xx_limit: Option<u32>,
}
