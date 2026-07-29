pub mod jwt;

use std::collections::HashMap;
use std::sync::{Arc, LazyLock};

use axum::http::StatusCode;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use pingora::http::RequestHeader;
use pingora_core::connectors::http::Connector;
use pingora_core::upstreams::peer::HttpPeer;
use pingora_http::ResponseHeader;
use pingora_proxy::Session;
use subtle::ConstantTimeEq;
use urlencoding::decode;

use aralez_spec::{AuthValidator, AuthPluginEntry};
use jwt::{check_jwt, JWT_TOKEN};

pub static AUTH_CONNECTOR: LazyLock<Connector> = LazyLock::new(|| Connector::new(None));

fn create_auth_validator(method: &str, option: Option<noyalib::Value>)
-> Result<Arc<dyn AuthValidator>, Box<dyn std::error::Error>> {
    let extract_str = || {
            option
                .as_ref()
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing or invalid configuration string".to_string())
        };

        let validator: Arc<dyn AuthValidator> = match method {
            "basic" => Arc::new(BasicAuth(extract_str()?.into())),
            "apikey" => Arc::new(ApiKeyAuth(extract_str()?.into())),
            "jwt" => Arc::new(JwtAuth),
            "forward" => Arc::new(ForwardAuth(extract_str()?.into())),
            _ => return Err("unsupported auth type".into()),
        };

        Ok(validator)
}

// --- Basic Auth ---
struct BasicAuth(Arc<str>);

#[async_trait::async_trait]
impl AuthValidator for BasicAuth {
    async fn validate(&self, session: &mut Session) -> Result<(), ResponseHeader> {
        if let Some(header) = session.get_header("authorization") {
            if let Ok(h) = header.to_str() {
                if let Some((_, val)) = h.split_once(' ') {
                    if let Ok(decoded) = STANDARD.decode(val) {
                        if decoded.as_slice().ct_eq(self.0.as_bytes()).into() {
                            return Ok(());
                        }
                    }
                }
            }
        }

        let mut resp = build_error_resp(StatusCode::UNAUTHORIZED);
        resp.insert_header("WWW-Authenticate", "Basic realm=\"Access Required\"").ok();
        Err(resp)
    }
}

inventory::submit! {
    AuthPluginEntry {
        name: "basic",
        create: |cred|create_auth_validator("basic", cred),
    }
}

// --- API Key Auth ---
struct ApiKeyAuth(Arc<str>);

#[async_trait::async_trait]
impl AuthValidator for ApiKeyAuth {
    async fn validate(&self, session: &mut Session) -> Result<(), ResponseHeader> {
        if let Some(header) = session.get_header("x-api-key") {
            if let Ok(h) = header.to_str() {
                if h.as_bytes().ct_eq(self.0.as_bytes()).into() {
                    return Ok(());
                }
            }
        }
        Err(build_error_resp(StatusCode::UNAUTHORIZED))
    }
}
inventory::submit! {
    AuthPluginEntry {
        name: "apikey",
        create: |cred| create_auth_validator("apikey", cred),
    }
}

// --- JWT Auth ---
struct JwtAuth;

#[async_trait::async_trait]
impl AuthValidator for JwtAuth {
    async fn validate(&self, session: &mut Session) -> Result<(), ResponseHeader> {
        if let Some(jwtsecret) = JWT_TOKEN.clone() {
            if let Some(tok) = get_query_param(session, "araleztoken") {
                if check_jwt(tok.as_str(), jwtsecret.as_ref()) {
                    return Ok(());
                } else {
                    return Err(build_error_resp(StatusCode::UNAUTHORIZED))
                }
            }
            if let Some(auth_header) = session.get_header("authorization") {
                if let Ok(header_str) = auth_header.to_str() {
                    if let Some((scheme, token)) = header_str.split_once(' ') {
                        if scheme.eq_ignore_ascii_case("bearer") {
                            if check_jwt(token, jwtsecret.as_ref()) {
                                return Ok(());
                            } else {
                                return Err(build_error_resp(StatusCode::UNAUTHORIZED))
                            }
                        }
                    }
                }
            }
        }
        Err(build_error_resp(StatusCode::UNAUTHORIZED))
    }
}

inventory::submit! {
    AuthPluginEntry {
        name: "jwt",
        create: |cred| create_auth_validator("jwt", cred),
    }
}

// --- Forward Auth ---
struct ForwardAuth(Arc<str>);

#[async_trait::async_trait]
impl AuthValidator for ForwardAuth {
    async fn validate(&self, session: &mut Session) -> Result<(), ResponseHeader> {
            let method = match session.req_header().method.as_str() {
                "HEAD" => "HEAD",
                _ => "GET",
            };

            let auth_url = &self.0;

        let (plain, tls) = if let Some(p) = auth_url.strip_prefix("http://") {
            (p, false)
        } else if let Some(p) = auth_url.strip_prefix("https://") {
            (p, true)
        } else {
            return Err(build_error_resp(StatusCode::INTERNAL_SERVER_ERROR));
        };

        let (addr, uri) = if let Some(pos) = plain.find('/') {
            (&plain[..pos], &plain[pos..])
        } else {
            (plain, "/")
        };

        let hp = match split_host_port(addr, tls) {
            Some(hp) => hp,
            None => {
                return Err(build_error_resp(StatusCode::INTERNAL_SERVER_ERROR));
            }
        };

        let peer = HttpPeer::new((hp.0, hp.1), tls, hp.0.to_string());

        let (mut http_session, _) = match AUTH_CONNECTOR.get_http_session(&peer).await {
            Ok(s) => s,
            Err(e) => {
                log::warn!("ForwardAuth: connect failed: {}", e);
                return Err(build_error_resp(StatusCode::BAD_GATEWAY));
            }
        };

        let mut auth_req = match RequestHeader::build(method, uri.as_bytes(), None) {
            Ok(r) => r,
            Err(e) => {
                log::warn!("ForwardAuth: failed to build request: {}", e);
                return Err(build_error_resp(StatusCode::INTERNAL_SERVER_ERROR));
            }
        };

        auth_req.insert_header("Host", addr).ok();
        auth_req.insert_header("X-Forwarded-Uri", uri).ok();
        auth_req.insert_header("X-Forwarded-Method", session.req_header().method.as_str()).ok();
        if let Some(auth) = session.req_header().headers.get("authorization") {
            auth_req.insert_header("Authorization", auth.clone()).ok();
        }

        if let Some(cookie) = session.req_header().headers.get("cookie") {
            auth_req.insert_header("Cookie", cookie.clone()).ok();
        }

        if tls {
            auth_req.insert_header("X-Forwarded-Proto", "https").ok();
        } else {
            auth_req.insert_header("X-Forwarded-Proto", "http").ok();
        }

        if let Err(e) = http_session.write_request_header(Box::new(auth_req)).await {
            log::warn!("ForwardAuth: write failed: {}", e);
            return Err(build_error_resp(StatusCode::BAD_GATEWAY));
        }

        let status = match http_session.read_response_header().await {
            Ok(_) => http_session.response_header().map(|r| r.status.as_u16()).unwrap_or(500),
            Err(e) => {
                log::warn!("ForwardAuth: read failed: {}", e);
                return Err(build_error_resp(StatusCode::BAD_GATEWAY));
            }
        };

        let auth_headers_to_forward: Vec<(String, String)> = if let Some(resp_header) = http_session.response_header() {
            resp_header
                .headers
                .iter()
                .filter_map(|(name, value)| {
                    let name_str = name.as_str();
                    if name_str.starts_with("x-") || name_str.starts_with("remote-") || name_str.starts_with("locat") {
                        value.to_str().ok().map(|v| (name_str.to_string(), v.to_string()))
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            Vec::new()
        };

        AUTH_CONNECTOR.release_http_session(http_session, &peer, None).await;

        if (200..300).contains(&status) {
            for (name, value) in auth_headers_to_forward {
                session.req_header_mut().insert_header(name, value).ok();
            }
            Ok(())
        } else {
            let status_code = StatusCode::from_u16(status).unwrap_or(StatusCode::UNAUTHORIZED);
            let mut resp = ResponseHeader::build(status_code, None).unwrap_or_else(|_| {
                ResponseHeader::build(StatusCode::UNAUTHORIZED, None).unwrap()
            });

            for (name, value) in auth_headers_to_forward {
                resp.insert_header(name, value).ok();
            }
            resp.insert_header("Content-Length", "0").ok();
            Err(resp)
        }
    }
}

inventory::submit! {
    AuthPluginEntry {
        name: "forward",
        create: |cred| create_auth_validator("forward", cred),
    }
}

// Helper
fn build_error_resp(status: StatusCode) -> ResponseHeader {
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

#[allow(clippy::needless_return)]
fn split_host_port(addr: &str, tls: bool) -> Option<(&str, u16, bool, &str)> {
    match addr.split_once(':') {
        Some((h, p)) => match p.parse::<u16>() {
            Ok(port) => return Some((h, port, tls, h)),
            Err(_) => {
                log::warn!("ForwardAuth: invalid port in {}", addr);
                return None;
            }
        },
        None => {
            if tls {
                return Some((addr, 443u16, tls, addr));
            } else {
                return Some((addr, 80u16, tls, addr));
            }
        }
    };
}
