pub mod jwt;

use std::collections::HashMap;
use std::sync::LazyLock;

use axum::http::StatusCode;
use pingora_core::connectors::http::Connector;
use pingora_http::ResponseHeader;
use pingora_proxy::Session;
use urlencoding::decode;

pub static AUTH_CONNECTOR: LazyLock<Connector> = LazyLock::new(|| Connector::new(None));

pub fn build_error_resp(status: StatusCode) -> ResponseHeader {
    let mut resp = ResponseHeader::build(status, None).unwrap();
    resp.insert_header("Content-Length", "0").ok();
    resp
}

pub fn get_query_param(session: &mut Session, key: &str) -> Option<String> {
    let query = session.req_header().uri.query()?;

    let params: HashMap<_, _> = query
        .split('&')
        .filter_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            let k = parts.next()?;
            let v = parts.next().unwrap_or("");
            Some((k, v))
        })
        .collect();
    params.get(key).and_then(|v| decode(v).ok()).map(|s| s.to_string())
}
