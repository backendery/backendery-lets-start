use anyhow::Result;
use sentry::types::Dsn;
use serde::Deserialize;
use url::Url;
use validator::{Validate, ValidationError};

#[derive(Clone, Debug, Default, Deserialize, Validate)]
#[must_use]
pub struct AppConfigs {
    #[validate(
        length(
            min = 1,
            message = "must be at least one of the allowed origins"
        )
    )]
    #[validate(custom(function = "validate_allow_origins_urls"))]
    pub(super) allow_cors_origins: Vec<String>,

    pub message_from_email: String,
    pub message_to_email: String,

    #[validate(
        range(
            min = 1,
            max = 10,
            message = "must be between 1 and 10 times"
        )
    )]
    pub retry_count: usize,
    #[validate(
        range(
            min = 10,
            max = 100,
            message = "must be between 10 and 100 msec"
        )
    )]
    pub retry_timeout: u64,

    #[validate(custom(function = "validate_sentry_dsn"))]
    pub(super) sentry_dsn: String,
    pub(super) sentry_environment: String,

    #[validate(url(message = "must be a valid SMTP addr (e.g., smtp.gmail.com:587)"))]
    pub smtp_addr: String,
    #[validate(custom(function = "validate_smtp_auth_uri"))]
    pub smtp_auth: String,
    #[validate(
        range(
            min = 1000,
            message = "must be at least 1000 msec"
        )
    )]
    pub smtp_connection_timeout: u64,
}

fn validate_allow_origins_urls(origins: &[String]) -> Result<(), ValidationError> {
    for origin in origins {
        Url::parse(origin).map_err(|_| {
            let mut err = ValidationError::new("invalid_allow_origins");
            err.message = Some("must be a valid URLs".into());
            err
        })?;
    }

    Ok(())
}

fn validate_sentry_dsn(dsn: &str) -> Result<(), ValidationError> {
    dsn.parse::<Dsn>().map_err(|_| {
        let mut err = ValidationError::new("invalid_sentry_dsn");
        err.message = Some("must be a valid Sentry DSN".into());
        err
    })?;

    Ok(())
}

fn validate_smtp_auth_uri(auth: &str) -> Result<(), ValidationError> {
    let re = regex::Regex::new(r"^[^@]+@[^@]+\.[^@]+:.+$").unwrap();
    if !re.is_match(auth) {
        let mut err = ValidationError::new("invalid_smtp_auth");
        err.message = Some("must be a valid auth string (e.g., email:password)".into());

        return Err(err);
    }

    Ok(())
}
