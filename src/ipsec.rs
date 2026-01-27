use axum::http::Request;
use std::net::IpAddr;
use tower_governor::{GovernorError, key_extractor::KeyExtractor};

/// Secure IP extractor for rate limiting behind trusted proxies (Shuttle, AWS, Cloudflare)
///
/// Priority:
/// 1. X-Real-IP (set by last trusted proxy, cannot be spoofed by client)
/// 2. ConnectInfo peer address (fallback for local dev without proxy)
///
/// SECURITY: Never uses X-Forwarded-For without trusted proxy validation
/// to prevent IP spoofing attacks.
#[derive(Clone, Copy)]
pub struct SecureIpKeyExtractor;

impl KeyExtractor for SecureIpKeyExtractor {
    type Key = IpAddr;

    fn extract<B>(&self, rq: &Request<B>) -> Result<Self::Key, GovernorError> {
        // PRIORITY 1: X-Real-IP (trusted proxy header)
        // This header is set by the last proxy (Shuttle/AWS/Cloudflare)
        // and overwrites any client-provided value, making it secure
        let ip = rq
            .headers()
            .get("x-real-ip")
            .and_then(|value| value.to_str().ok())
            .and_then(|x| x.trim().parse::<IpAddr>().ok());

        // PRIORITY 2: ConnectInfo (actual TCP peer)
        // Fallback for local development or when no proxy is present
        // Returns proxy IP in production, but client IP in dev
        ip.or_else(|| {
            rq.extensions()
                .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
                .map(|ci| ci.0.ip())
        })
        .ok_or(GovernorError::UnableToExtractKey)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::extract::ConnectInfo;
    use axum::http::Request;
    use std::net::SocketAddr;

    #[test]
    fn test_x_real_ip_takes_priority() {
        let extractor = SecureIpKeyExtractor;
        let mut rq = Request::builder()
            .header("x-real-ip", "203.0.113.5")
            .header("x-forwarded-for", "1.1.1.1") // Should be ignored
            .body(Body::empty())
            .unwrap();

        let connect_info = ConnectInfo("127.0.0.1:8080".parse::<SocketAddr>().unwrap());
        rq.extensions_mut().insert(connect_info);

        let result = extractor.extract(&rq);
        assert_eq!(result.unwrap(), "203.0.113.5".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn test_fallback_to_connect_info() {
        let extractor = SecureIpKeyExtractor;
        let mut rq = Request::builder().body(Body::empty()).unwrap();

        let connect_info = ConnectInfo("192.168.1.1:8080".parse::<SocketAddr>().unwrap());
        rq.extensions_mut().insert(connect_info);

        let result = extractor.extract(&rq);
        assert_eq!(result.unwrap(), "192.168.1.1".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn test_ignores_spoofed_x_forwarded_for() {
        let extractor = SecureIpKeyExtractor;
        let mut rq = Request::builder()
            .header("x-forwarded-for", "1.1.1.1, 8.8.8.8")
            .body(Body::empty())
            .unwrap();

        let connect_info = ConnectInfo("203.0.113.5:8080".parse::<SocketAddr>().unwrap());
        rq.extensions_mut().insert(connect_info);

        let result = extractor.extract(&rq);
        // Should use ConnectInfo, NOT spoofed X-Forwarded-For
        assert_eq!(result.unwrap(), "203.0.113.5".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn test_handles_ipv6() {
        let extractor = SecureIpKeyExtractor;
        let rq = Request::builder()
            .header("x-real-ip", "2001:db8::1")
            .body(Body::empty())
            .unwrap();

        let result = extractor.extract(&rq);
        assert_eq!(result.unwrap(), "2001:db8::1".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn test_handles_invalid_ip() {
        let extractor = SecureIpKeyExtractor;
        let mut rq = Request::builder()
            .header("x-real-ip", "not-an-ip")
            .body(Body::empty())
            .unwrap();

        let connect_info = ConnectInfo("127.0.0.1:8080".parse::<SocketAddr>().unwrap());
        rq.extensions_mut().insert(connect_info);

        let result = extractor.extract(&rq);
        // Should fallback to ConnectInfo when X-Real-IP is invalid
        assert_eq!(result.unwrap(), "127.0.0.1".parse::<IpAddr>().unwrap());
    }
}
