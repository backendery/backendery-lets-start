use axum::http::Request;
use std::net::IpAddr;
use tower_governor::{GovernorError, key_extractor::KeyExtractor};

#[derive(Clone, Copy)]
pub struct SmartIpKeyExtractor;

impl KeyExtractor for SmartIpKeyExtractor {
    type Key = IpAddr;

    fn extract<B>(&self, rq: &Request<B>) -> Result<Self::Key, GovernorError> {
        // Try to take from X-Forwarded-For (standard for proxies)
        let ip = rq.headers()
            .get("x-forwarded-for")
            .and_then(|value| value.to_str().ok())
            .and_then(|x| x.split(',').next()) // Get the first IP in the list
            .and_then(|x| x.trim().parse::<IpAddr>().ok());

        // If empty, try X-Real-IP
        let ip = ip.or_else(|| {
            rq.headers()
                .get("x-real-ip")
                .and_then(|value| value.to_str().ok())
                .and_then(|x| x.parse::<IpAddr>().ok())
        });

        // Fallback to peer_addr (if there is no proxy or we are in dev)
        ip.or_else(|| {
            rq.extensions()
                .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
                .map(|ci| ci.0.ip())
        })
        .ok_or(GovernorError::UnableToExtractKey)
    }
}
