use std::convert::TryFrom;

use anyhow::{Context, Result};
use config::{Config, File};
use sentry::types::Dsn;
use serde::Deserialize;
use shuttle_runtime::SecretStore;
use validator::{Validate, ValidationError};

use crate::cors::validate_allow_origin_entry;

#[derive(Clone, Debug, Default, Deserialize, Validate)]
#[must_use]
pub struct AppConfigs {
    #[validate(length(min = 1, message = "must be at least one of the allowed origins"))]
    #[validate(custom(function = "validate_allow_origins_urls"))]
    pub(super) allow_cors_origins: Vec<String>,

    pub(super) from_mailbox: String,
    pub(super) to_mailbox: String,

    #[validate(range(min = 1, max = 10, message = "must be between 1 and 10 times"))]
    pub retry_count: usize,
    #[validate(range(min = 10, max = 100, message = "must be between 10 and 100 msec"))]
    pub retry_timeout: u64,

    #[validate(custom(function = "validate_sentry_dsn"))]
    pub(super) sentry_dsn: String,
    pub(super) sentry_environment: String,

    #[validate(range(
        min = 1,
        max = 1024,
        message = "must be between 1 and 1024 concurrent requests"
    ))]
    pub(super) concurrency_limit: usize,

    #[validate(custom(function = "validate_smtp_addr"))]
    pub(super) smtp_addr: String,
    #[validate(custom(function = "validate_smtp_auth_uri"))]
    pub(super) smtp_auth: String,
    #[validate(range(min = 1000, message = "must be at least 1000 msec"))]
    pub(super) smtp_connection_timeout: u64,
}

impl AppConfigs {
    pub fn new(secrets: SecretStore) -> Result<Self> {
        let secrets_source =
            Config::try_from(&secrets).context("couldn't get the secrets from the secret store")?;

        let configs: Self = Config::builder()
            .add_source(File::with_name("configs/default").required(true))
            .add_source(secrets_source)
            .build()
            .inspect_err(|_| tracing::error!("config error (sanitized)"))
            .context("couldn't build the application config")?
            .try_deserialize()
            .inspect_err(|_| tracing::error!("config deserialize error (sanitized)"))
            .context("couldn't deserialize the config")?;

        configs.validate().context("couldn't validate the config")?;

        Ok(configs)
    }
}

impl TryFrom<Config> for AppConfigs {
    type Error = anyhow::Error;

    fn try_from(cfg: Config) -> Result<Self, Self::Error> {
        let configs: Self =
            cfg.try_deserialize::<Self>().context("couldn't deserialize the config")?;
        configs.validate().context("couldn't validate the config")?;

        Ok(configs)
    }
}

fn validate_allow_origins_urls(origins: &[String]) -> Result<(), ValidationError> {
    for origin in origins {
        validate_allow_origin_entry(origin)?;
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
    let Some((user, pass)) = auth.split_once(":") else {
        let mut err = ValidationError::new("invalid_smtp_auth");
        err.message = Some("expected username:password".into());
        return Err(err);
    };
    if user.is_empty() || pass.is_empty() {
        let mut err = ValidationError::new("invalid_smtp_auth");
        err.message = Some("username/password must be non-empty".into());
        return Err(err);
    }

    Ok(())
}

fn validate_smtp_addr(addr: &str) -> Result<(), ValidationError> {
    let Some((host, port_str)) = addr.rsplit_once(":") else {
        let mut err = ValidationError::new("invalid_smtp_addr");
        err.message = Some("must be host:port".into());
        return Err(err);
    };
    if host.is_empty() || port_str.parse::<u16>().is_err() {
        let mut err = ValidationError::new("invalid_smtp_addr");
        err.message = Some("must be host:port, port 1-65535".into());
        return Err(err);
    }

    Ok(())
}
