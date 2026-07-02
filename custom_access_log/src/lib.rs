use default_interface::AccessLog;
use default_interface::LogMessage;
use log::info;

pub struct CustomAccessLog;

impl AccessLog for CustomAccessLog {
    fn info(msg: LogMessage) {
        info!(
            "{}, {}, client: {}, version: {:?}, useragent: {}, matched_path: {}",
            msg.response_code,
            msg.summary,
            msg.client_ip,
            msg.version,
            msg.headers.get("user-agent").and_then(|v| v.to_str().ok()).unwrap_or("-"),
            msg.matched_path.as_deref().map(|s| s.as_str()).unwrap_or("-"),
        );
    }
}
