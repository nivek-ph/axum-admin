use std::{convert::Infallible, net::SocketAddr, ops::Deref};

use axum::{
    extract::{ConnectInfo, FromRequestParts},
    http::{HeaderMap, request::Parts},
};

/// Client IP resolved for audit/logging.
///
/// Preference order:
/// 1. first hop in `X-Forwarded-For`
/// 2. `X-Real-IP`
/// 3. TCP peer address from `ConnectInfo`
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClientIp(pub String);

impl Deref for ClientIp {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<S> FromRequestParts<S> for ClientIp
where
    S: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(Self(resolve_client_ip(&parts.headers, peer_addr(parts))))
    }
}

fn peer_addr(parts: &Parts) -> Option<SocketAddr> {
    parts
        .extensions
        .get::<ConnectInfo<SocketAddr>>()
        .map(|connect| connect.0)
}

fn resolve_client_ip(headers: &HeaderMap, peer: Option<SocketAddr>) -> String {
    if let Some(ip) = extract_from_header(headers, "x-forwarded-for") {
        return ip;
    }
    if let Some(ip) = extract_from_header(headers, "x-real-ip") {
        return ip;
    }
    peer.map(|addr| addr.ip().to_string()).unwrap_or_default()
}

fn extract_from_header(headers: &HeaderMap, key: &str) -> Option<String> {
    let value = headers.get(key)?.to_str().ok()?.split(',').next()?.trim();
    (!value.is_empty()).then(|| value.to_string())
}

#[cfg(test)]
mod tests {
    use axum::{
        extract::{ConnectInfo, FromRequestParts},
        http::{HeaderValue, Request},
    };

    use super::*;

    #[tokio::test]
    async fn prefers_first_forwarded_for_hop() {
        let mut request = Request::new(());
        request.headers_mut().insert(
            "x-forwarded-for",
            HeaderValue::from_static("203.0.113.10, 10.0.0.1"),
        );
        request
            .extensions_mut()
            .insert(ConnectInfo("127.0.0.1:3000".parse::<SocketAddr>().unwrap()));
        let (mut parts, _) = request.into_parts();

        let ClientIp(ip) = ClientIp::from_request_parts(&mut parts, &())
            .await
            .expect("client ip should resolve");
        assert_eq!(ip, "203.0.113.10");
    }

    #[tokio::test]
    async fn falls_back_to_x_real_ip() {
        let mut request = Request::new(());
        request
            .headers_mut()
            .insert("x-real-ip", HeaderValue::from_static("198.51.100.7"));
        request
            .extensions_mut()
            .insert(ConnectInfo("127.0.0.1:3000".parse::<SocketAddr>().unwrap()));
        let (mut parts, _) = request.into_parts();

        let ClientIp(ip) = ClientIp::from_request_parts(&mut parts, &())
            .await
            .expect("client ip should resolve");
        assert_eq!(ip, "198.51.100.7");
    }

    #[tokio::test]
    async fn falls_back_to_peer_when_proxy_headers_missing() {
        let mut request = Request::new(());
        request.extensions_mut().insert(ConnectInfo(
            "192.0.2.44:54321".parse::<SocketAddr>().unwrap(),
        ));
        let (mut parts, _) = request.into_parts();

        let ClientIp(ip) = ClientIp::from_request_parts(&mut parts, &())
            .await
            .expect("client ip should resolve");
        assert_eq!(ip, "192.0.2.44");
    }

    #[tokio::test]
    async fn empty_without_headers_or_peer() {
        let (mut parts, _) = Request::new(()).into_parts();

        let ClientIp(ip) = ClientIp::from_request_parts(&mut parts, &())
            .await
            .expect("client ip should resolve");
        assert_eq!(ip, "");
    }
}
