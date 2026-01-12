use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use url::Url;
use validator::ValidationError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Scheme {
    Http,
    Https,
}

impl Scheme {
    fn from_str(proto: &str) -> Option<Self> {
        match proto {
            x if x.eq_ignore_ascii_case("http") => Some(Scheme::Http),
            x if x.eq_ignore_ascii_case("https") => Some(Scheme::Https),
            _ => None,
        }
    }

    fn default_port(self) -> u16 {
        match self {
            Scheme::Http => 80,
            Scheme::Https => 443,
        }
    }
}

pub struct CorsMatcher {
    // (Scheme, Port) -> Set of lowercase hosts
    allow_list: HashMap<(Scheme, u16), HashSet<String>>,
}

impl CorsMatcher {
    pub fn new(origins: &[String]) -> Self {
        let mut allow_list: HashMap<(Scheme, u16), HashSet<String>> = HashMap::new();

        for origin in origins {
            if let Ok((scheme, host, port)) = parse_exact_origin(origin) {
                allow_list.entry((scheme, port)).or_default().insert(host);
            }
        }
        Self { allow_list }
    }

    pub fn matches(&self, origin: &str) -> bool {
        let (scheme_part, rest) = match origin.split_once("://") {
            Some(x) => x,
            None => return false,
        };

        let scheme = match Scheme::from_str(scheme_part) {
            Some(x) => x,
            None => return false,
        };

        // Host and port parsing (support for IPv6 and standard ports)
        let (host, port) = if rest.starts_with('[') {
            if let Some(bracket_end) = rest.find(']') {
                let ht = &rest[..=bracket_end];

                let port_part = &rest[bracket_end + 1..];
                let pt = if let Some(pt_str) = port_part.strip_prefix(':') {
                    pt_str.parse::<u16>().unwrap_or(0)
                } else {
                    scheme.default_port()
                };

                (ht, pt)
            } else {
                (rest, scheme.default_port())
            }
        } else if let Some((ht, pt_str)) = rest.rsplit_once(':') {
            (ht, pt_str.parse::<u16>().unwrap_or(0))
        } else {
            (rest, scheme.default_port())
        };

        // Zero-allocation check if host is already in lowercase
        let request_host: Cow<'_, str> = if host.bytes().any(|x| x.is_ascii_uppercase()) {
            Cow::Owned(host.to_ascii_lowercase())
        } else {
            Cow::Borrowed(host)
        };

        self.allow_list
            .get(&(scheme, port))
            .is_some_and(|hosts| hosts.contains(&*request_host))
    }
}

pub fn validate_allow_origin_entry(origin: &str) -> Result<(), ValidationError> {
    if origin == "*" {
        return Ok(());
    }
    parse_exact_origin(origin).map(|_| ())
}

fn parse_exact_origin(origin: &str) -> Result<(Scheme, String, u16), ValidationError> {
    let normalized = origin.trim_end_matches('/');

    if normalized == "*" {
        return Err(invalid_origin_error(
            "use AllowOrigin::any() for global wildcard",
        ));
    }
    if normalized.contains('*') {
        return Err(invalid_origin_error(
            "wildcards are not supported in this mode",
        ));
    }

    // Special processing of localhost so as not to parse the URL unnecessarily if you don't want to,
    // but using Url::parse is more reliable. Let's stick with Url::parse for consistency
    let url = Url::parse(normalized).map_err(|_| invalid_origin_error("must be a valid URL"))?;
    if url.path() != "/" || url.query().is_some() || url.fragment().is_some() {
        return Err(invalid_origin_error("path/query/fragment not allowed"));
    }

    let scheme = Scheme::from_str(url.scheme()).ok_or_else(|| invalid_origin_error("unsupported scheme"))?;
    let host = url
        .host_str()
        .map(|hst| hst.to_ascii_lowercase())
        .ok_or_else(|| invalid_origin_error("no host"))?;
    let port = url.port_or_known_default().unwrap_or(scheme.default_port());

    Ok((scheme, host, port))
}

fn invalid_origin_error(message: &'static str) -> ValidationError {
    let mut err = ValidationError::new("invalid_allow_origin");
    err.message = Some(Cow::Borrowed(message));
    err
}
