#[cfg(not(feature = "custom-access-log"))]
use crate::default::access_log::DefaultAccessLog;
use crate::utils::metrics::LOGGING_ERRORS;
use crate::web::proxyhttp::Context;
#[cfg(feature = "custom-access-log")]
use custom_access_log::CustomAccessLog;
use default_interface::AccessLog;
use default_interface::LogMessage;
use log::info;
use pingora_proxy::Session;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::OnceLock;
use tokio::sync::mpsc;



static LOG_SENDER: OnceLock<mpsc::Sender<LogMessage>> = OnceLock::new();
static ACCESS_LOG: OnceLock<LogLevel> = OnceLock::new();
const LOG_BUFFER: usize = 16384;

pub fn init_access_log(level_str: &str) {
    let level = LogLevel::from_str(level_str);
    let _ = ACCESS_LOG.set(level);
}

#[derive(Debug)]
pub enum LogLevel {
    Access,
    Error,
    None,
}

impl LogLevel {
    pub fn from_str(s: &str) -> Self {
        match s {
            "all" => LogLevel::Access,
            "error" => LogLevel::Error,
            _ => LogLevel::None,
        }
    }
}

pub fn access_log(response_code: u16, summary: &str, session: &Session, ctx: &Context) {
    let level = ACCESS_LOG.get().unwrap_or(&LogLevel::None);

    let should_log = match level {
        LogLevel::Access => true,
        LogLevel::None => false,
        LogLevel::Error => !(100..=399).contains(&response_code),
    };

    if !should_log {
        return;
    }

    let ip = session
        .client_addr()
        .and_then(|addr| addr.as_inet())
        .map(|addr| addr.ip())
        .unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST));

    //let user_agent = session.req_header().headers.get("user-agent").and_then(|v| v.to_str().ok()).unwrap_or("-");

    let log = LogMessage {
        response_code,
        summary: summary.to_owned(),
        client_ip: ip,
        version: session.req_header().version,
        headers: session.req_header().headers.clone(),
        matched_path: ctx.matched_path.clone(),
        backend_id: ctx.backend_id.clone(),
        start_time: ctx.start_time,
        upstream_peer: serde_json::to_value(ctx.upstream_peer.clone()).ok(),
        client_headers: ctx.client_headers.clone(),
        x4xx_limit: ctx.x4xx_limit,
    };

    if let Some(sender) = LOG_SENDER.get() {
        let sender = sender;
        if let Err(_) = sender.try_send(log) {
            LOGGING_ERRORS.inc();
        }
    }
}

pub fn init_logging(enabled: Option<String>) {
    if let Some(_) = enabled {
        LOGGING_ERRORS.set(0);
        info!("Enabling {:?} log, with buffer of {} messages", ACCESS_LOG.get().unwrap_or(&LogLevel::None), LOG_BUFFER);
        let (ltx, lrx) = mpsc::channel(LOG_BUFFER);
        LOG_SENDER.set(ltx).unwrap();
        std::thread::spawn(move || log_receiver(lrx));
    }
}

pub fn log_receiver(mut receiver: mpsc::Receiver<LogMessage>) {
    while let Some(msg) = receiver.blocking_recv() {
        #[cfg(not(feature = "custom-access-log"))]
        DefaultAccessLog::info(msg);
        #[cfg(feature = "custom-access-log")]
        CustomAccessLog::info(msg);
    }
}
