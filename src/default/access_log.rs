use default_interface::AccessLog;
use default_interface::LogMessage;
use log::info;

pub struct DefaultAccessLog;
impl AccessLog for DefaultAccessLog {
    fn info(msg: LogMessage) {
        info!(
            "{}, {}, client: {}, version: {:?}, useragent: {}",
            msg.response_code,
            msg.summary,
            msg.client_ip,
            msg.version,
            msg.headers.get("user-agent").and_then(|v| v.to_str().ok()).unwrap_or("-"),
        );
    }
}
