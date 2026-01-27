use axum::http::Request;
use std::net::IpAddr;
use tower_governor::{GovernorError, key_extractor::KeyExtractor};

/// Advanced IP extractor with proper X-Forwarded-For parsing
///
/// Use this if you:
/// - Know your proxy architecture (how many proxies in chain)
/// - Need to support X-Forwarded-For correctly
/// - Want more control over IP extraction logic
///
/// Priority:
/// 1. X-Real-IP (if present and valid)
/// 2. Last IP from X-Forwarded-For (rightmost = closest to server)
/// 3. ConnectInfo peer address
#[derive(Clone, Copy)]
pub struct AdvancedIpKeyExtractor;

impl KeyExtractor for AdvancedIpKeyExtractor {
    type Key = IpAddr;

    fn extract<B>(&self, rq: &Request<B>) -> Result<Self::Key, GovernorError> {
        // PRIORITY 1: X-Real-IP (most reliable)
        let ip = rq
            .headers()
            .get("x-real-ip")
            .and_then(|value| value.to_str().ok())
            .and_then(|x| x.trim().parse::<IpAddr>().ok());

        // PRIORITY 2: Last IP from X-Forwarded-For
        // X-Forwarded-For format: client, proxy1, proxy2, ...
        // Take the LAST IP (rightmost) which is added by the trusted proxy
        let ip = ip.or_else(|| {
            rq.headers()
                .get("x-forwarded-for")
                .and_then(|value| value.to_str().ok())
                .and_then(|xff| {
                    xff.split(',')
                        .last() // Take LAST IP, not first!
                        .and_then(|ip_str| ip_str.trim().parse::<IpAddr>().ok())
                })
        });

        // PRIORITY 3: ConnectInfo
        ip.or_else(|| {
            rq.extensions()
                .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
                .map(|ci| ci.0.ip())
        })
        .ok_or(GovernorError::UnableToExtractKey)
    }
}

/// Configurable IP extractor with trusted proxy support
///
/// This is the most secure approach if you know your proxy IPs.
/// Configure trusted proxies and it will:
/// - Validate X-Forwarded-For chain
/// - Extract rightmost non-trusted IP
/// - Prevent spoofing completely
#[derive(Clone)]
pub struct TrustedProxyIpExtractor {
    /// List of trusted proxy IPs (e.g., your load balancer IPs)
    trusted_proxies: Vec<IpAddr>,
}

impl TrustedProxyIpExtractor {
    pub fn new(trusted_proxies: Vec<IpAddr>) -> Self {
        Self { trusted_proxies }
    }

    /// Create with common cloud provider proxy ranges
    /// WARNING: This is just an example. You should configure your actual proxy IPs!
    pub fn with_common_cloud_proxies() -> Self {
        Self {
            // Add your actual proxy IPs here
            // For example, if using AWS ALB or Cloudflare
            trusted_proxies: vec![
                // Example: "10.0.0.0/8".parse().unwrap(), // Private network
                // Add your actual load balancer IPs
            ],
        }
    }
}

impl KeyExtractor for TrustedProxyIpExtractor {
    type Key = IpAddr;

    fn extract<B>(&self, rq: &Request<B>) -> Result<Self::Key, GovernorError> {
        // Try X-Real-IP first
        let ip = rq
            .headers()
            .get("x-real-ip")
            .and_then(|value| value.to_str().ok())
            .and_then(|x| x.trim().parse::<IpAddr>().ok())
            .filter(|ip| !self.trusted_proxies.contains(ip));

        // Parse X-Forwarded-For and find rightmost non-trusted IP
        let ip = ip.or_else(|| {
            rq.headers()
                .get("x-forwarded-for")
                .and_then(|value| value.to_str().ok())
                .and_then(|xff| {
                    // Parse all IPs in the chain
                    let ips: Vec<IpAddr> = xff
                        .split(',')
                        .filter_map(|s| s.trim().parse::<IpAddr>().ok())
                        .collect();

                    // Find the rightmost IP that is NOT in trusted proxies
                    // This is the real client IP
                    ips.into_iter()
                        .rev() // Start from rightmost
                        .find(|ip| !self.trusted_proxies.contains(ip))
                })
        });

        // Fallback to ConnectInfo
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
    fn test_advanced_takes_last_xff() {
        let extractor = AdvancedIpKeyExtractor;
        let mut rq = Request::builder()
            .header("x-forwarded-for", "1.1.1.1, 8.8.8.8, 203.0.113.5")
            .body(Body::empty())
            .unwrap();

        let connect_info = ConnectInfo("127.0.0.1:8080".parse::<SocketAddr>().unwrap());
        rq.extensions_mut().insert(connect_info);

        let result = extractor.extract(&rq);
        // Should take LAST IP from X-Forwarded-For
        assert_eq!(result.unwrap(), "203.0.113.5".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn test_trusted_proxy_filters_proxy_ips() {
        let proxy_ip: IpAddr = "10.0.0.1".parse().unwrap();
        let extractor = TrustedProxyIpExtractor::new(vec![proxy_ip]);

        let mut rq = Request::builder()
            // Client IP, then proxy IP
            .header("x-forwarded-for", "203.0.113.5, 10.0.0.1")
            .body(Body::empty())
            .unwrap();

        let connect_info = ConnectInfo("10.0.0.1:8080".parse::<SocketAddr>().unwrap());
        rq.extensions_mut().insert(connect_info);

        let result = extractor.extract(&rq);
        // Should return client IP, not proxy IP
        assert_eq!(result.unwrap(), "203.0.113.5".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn test_trusted_proxy_prevents_spoofing() {
        let proxy_ip: IpAddr = "10.0.0.1".parse().unwrap();
        let extractor = TrustedProxyIpExtractor::new(vec![proxy_ip]);

        let mut rq = Request::builder()
            // Attacker tries to spoof by adding fake IPs
            .header("x-forwarded-for", "1.1.1.1, 2.2.2.2, 203.0.113.5, 10.0.0.1")
            .body(Body::empty())
            .unwrap();

        let connect_info = ConnectInfo("10.0.0.1:8080".parse::<SocketAddr>().unwrap());
        rq.extensions_mut().insert(connect_info);

        let result = extractor.extract(&rq);
        // Should return the rightmost non-trusted IP
        assert_eq!(result.unwrap(), "203.0.113.5".parse::<IpAddr>().unwrap());
    }
}
