use std::borrow::Cow;

#[derive(Debug)]
pub(super) enum AllowedOrigin {
    Exact(Cow<'static, str>),
    WildcardHttps(Cow<'static, str>),
    WildcardHttp(Cow<'static, str>),
}

impl AllowedOrigin {
    pub(super) fn matches(&self, origin: &str) -> bool {
        let origin = origin.trim_end_matches('/');

        match self {
            AllowedOrigin::Exact(exact) => origin == exact.as_ref(),
            AllowedOrigin::WildcardHttps(base) => origin
                .strip_prefix("https://")
                .is_some_and(|host| host.ends_with(base.as_ref()) && host != base.as_ref()),
            AllowedOrigin::WildcardHttp(base) => origin
                .strip_prefix("http://")
                .is_some_and(|host| host.ends_with(base.as_ref()) && host != base.as_ref()),
        }
    }
}

pub(super) fn parse_allowed_origins(origins: &[String]) -> Vec<AllowedOrigin> {
    origins
        .iter()
        .map(|origin| {
            let trimmed = origin.trim_end_matches('/').to_owned();

            match (
                trimmed.strip_prefix("https://*."),
                trimmed.strip_prefix("http://*."),
            ) {
                (Some(base), _) => AllowedOrigin::WildcardHttps(Cow::Owned(base.to_string())),
                (_, Some(base)) => AllowedOrigin::WildcardHttp(Cow::Owned(base.to_string())),
                              _ => AllowedOrigin::Exact(Cow::Owned(trimmed)),
            }
        })
        .collect()
}
