use crate::utils::metrics::LOGGING_ERRORS;
use crate::utils::structs::AppConfig;
use aralez_spec::{
    AccessLogger, LoggerPluginEntry, LogMessage as PluginLogMessage,
};
use log::info;
use pingora_cache::CachePhase;
use pingora_http::Version;
use pingora_proxy::Session;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, OnceLock};
use tokio::sync::mpsc;

static LOGGER: OnceLock<Arc<dyn AccessLogger>> = OnceLock::new();
static LOG_SENDER: OnceLock<mpsc::Sender<PluginLogMessage>> = OnceLock::new();

const LOG_BUFFER: usize = 16384;

pub fn log_builder(
    conf: &AppConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let plugin = build_logger_plugin(conf)?;

    LOGGING_ERRORS.set(0);

    info!(
        "Enabling {:?} log, with buffer of {} messages",
        conf.access_log, LOG_BUFFER
    );

    let (ltx, lrx) = mpsc::channel(LOG_BUFFER);

    LOG_SENDER
        .set(ltx)
        .map_err(|_| "access log sender is already initialized")?;

    let worker_plugin = plugin.clone();
    std::thread::spawn(move || access_log_worker(worker_plugin, lrx));

    LOGGER
        .set(plugin)
        .map_err(|_| "logger plugin is already initialized")?;

    Ok(())
}

fn build_logger_plugin(
    conf: &AppConfig,
) -> Result<Arc<dyn AccessLogger>, Box<dyn std::error::Error>> {
    let mut iter = inventory::iter::<LoggerPluginEntry>.into_iter();

    let Some(registration) = iter.next() else {
        return Err("no logger plugin is registered".into());
    };

    if iter.next().is_some() {
        return Err("multiple logger plugins are registered".into());
    }

    let logger_option = conf
        .options
        .as_ref()
        .and_then(|o| o.logger.clone());

    let access_level = conf
        .access_log
        .clone()
        .unwrap_or_else(|| "none".to_string());

    (registration.create)(
        conf.log_level.as_str(),
        conf.log_file.clone(),
        access_level.as_str(),
        logger_option,
    )
}

fn access_log_worker(
    plugin: Arc<dyn AccessLogger>,
    mut receiver: mpsc::Receiver<PluginLogMessage>,
) {
    while let Some(msg) = receiver.blocking_recv() {
        let line = plugin.format_access_log(&msg);
        info!("{}", line);
    }
}

pub fn access_log(response_code: u16, summary: &str, session: &Session) {
    let Some(plugin) = LOGGER.get() else {
        return;
    };

    if !plugin.pre_should_log(response_code) {
        return;
    }

    let client_ip = session
        .client_addr()
        .and_then(|addr| addr.as_inet())
        .map(|addr| addr.ip())
        .unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST));

    let user_agent = session
        .req_header()
        .headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-")
        .to_owned();

    let msg = PluginLogMessage {
        response_code,
        summary: summary.to_owned(),
        client_ip,
        version: http_version_to_string(session.req_header().version),
        user_agent,
        cache_status: cache_phase_to_string(session.cache.phase()),
    };

    if !plugin.should_log(&msg) {
        return;
    }

    if let Some(sender) = LOG_SENDER.get() {
        if sender.try_send(msg).is_err() {
            LOGGING_ERRORS.inc();
        }
    }
}

fn http_version_to_string(version: Version) -> String {
    format!("{:?}", version)
}

fn cache_phase_to_string(phase: CachePhase) -> String {
    phase.as_str().to_owned()
}
