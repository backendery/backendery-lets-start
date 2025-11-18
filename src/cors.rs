use std::borrow::Cow;

use globset::{GlobBuilder, GlobMatcher};
use url::Url;
use validator::ValidationError;

#[derive(Debug)]
pub(super) enum AllowedOrigin {
    Exact { scheme: Scheme, host: String, port: u16 },
    Localhost { scheme: Scheme },
    Wildcard { scheme: Scheme, matcher: GlobMatcher, base_host: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Scheme {
    Http,
    Https,
}

impl Scheme {
    fn try_from_str(value: &str) -> Option<Self> {
        match value {
            "http" => Some(Scheme::Http),
            "https" => Some(Scheme::Https),
            _ => None,
        }
    }

    fn port(self) -> u16 {
        match self {
            Scheme::Http => 80,
            Scheme::Https => 443,
        }
    }
}

impl AllowedOrigin {
    pub(super) fn matches(&self, origin: &str) -> bool {
        let trimmed = origin.trim_end_matches('/');
        let Ok(url) = Url::parse(trimmed) else {
            return false;
        };

        let Some(host) = url.host_str() else {
            return false;
        };

        let request_scheme = match Scheme::try_from_str(url.scheme()) {
            Some(scheme) => scheme,
            None => return false,
        };

        let request_host = host.to_ascii_lowercase();
        let request_port = url.port_or_known_default().unwrap_or_else(|| request_scheme.port());

        match self {
            AllowedOrigin::Exact { scheme, host, port } => {
                request_scheme == *scheme && request_host == *host && request_port == *port
            }
            AllowedOrigin::Localhost { scheme } => {
                request_scheme == *scheme && request_host == "localhost"
            }
            AllowedOrigin::Wildcard { scheme, matcher, base_host } => {
                request_scheme == *scheme
                    && matcher.is_match(&request_host)
                    && request_host != *base_host
            }
        }
    }
}

pub(super) fn parse_allowed_origins(origins: &[String]) -> Vec<AllowedOrigin> {
    origins.iter().filter_map(|origin| compile_allowed_origin(origin).ok()).collect()
}

pub fn validate_allow_origin_entry(origin: &str) -> Result<(), ValidationError> {
    compile_allowed_origin(origin).map(|_| ())
}

fn compile_allowed_origin(origin: &str) -> Result<AllowedOrigin, ValidationError> {
    let normalized = origin.trim_end_matches('/');

    if normalized.eq_ignore_ascii_case("http://localhost") {
        return Ok(AllowedOrigin::Localhost { scheme: Scheme::Http });
    }

    if normalized.eq_ignore_ascii_case("https://localhost") {
        return Ok(AllowedOrigin::Localhost { scheme: Scheme::Https });
    }

    if normalized.contains('*') {
        return compile_wildcard_origin(normalized);
    }

    compile_exact_origin(normalized)
}

fn compile_exact_origin(origin: &str) -> Result<AllowedOrigin, ValidationError> {
    let url = Url::parse(origin).map_err(|_| invalid_origin_error("must be a valid URL"))?;

    let scheme = Scheme::try_from_str(url.scheme())
        .ok_or_else(|| invalid_origin_error("only http/https schemes are supported"))?;
    let host = url
        .host_str()
        .map(|host| host.to_ascii_lowercase())
        .ok_or_else(|| invalid_origin_error("must contain a host"))?;
    let port = url.port_or_known_default().unwrap_or_else(|| scheme.port());

    if url.path() != "/" || url.query().is_some() || url.fragment().is_some() {
        return Err(invalid_origin_error(
            "must not contain path, query, or fragment",
        ));
    }

    Ok(AllowedOrigin::Exact { scheme, host, port })
}

fn compile_wildcard_origin(origin: &str) -> Result<AllowedOrigin, ValidationError> {
    let (scheme_part, host_part) = origin
        .split_once("://")
        .ok_or_else(|| invalid_origin_error("wildcard origins must include scheme"))?;

    let scheme = Scheme::try_from_str(scheme_part)
        .ok_or_else(|| invalid_origin_error("only http/https schemes are supported"))?;

    if !host_part.starts_with("*.") {
        return Err(invalid_origin_error(
            "wildcard origins must use the *.example.com format",
        ));
    }

    if host_part.contains('/') {
        return Err(invalid_origin_error(
            "wildcard origins cannot include paths",
        ));
    }

    if host_part.contains(':') {
        return Err(invalid_origin_error(
            "wildcard origins cannot include ports",
        ));
    }

    let base_host = host_part[2..].to_ascii_lowercase();

    if base_host.is_empty() || !base_host.contains('.') {
        return Err(invalid_origin_error(
            "wildcard base must be a valid domain like example.com",
        ));
    }

    if base_host.contains('*') {
        return Err(invalid_origin_error(
            "only a single leading wildcard is supported",
        ));
    }

    let pattern = format!("*.{base_host}");
    let matcher = GlobBuilder::new(&pattern)
        .case_insensitive(true)
        .literal_separator(true)
        .build()
        .map_err(|_| invalid_origin_error("invalid wildcard glob pattern"))?
        .compile_matcher();

    Ok(AllowedOrigin::Wildcard { scheme, matcher, base_host })
}

fn invalid_origin_error(message: &str) -> ValidationError {
    let mut err = ValidationError::new("invalid_allow_origin");
    err.message = Some(Cow::Owned(message.to_string()));
    err
}
