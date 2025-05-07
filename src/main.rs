mod api;
mod configs;

use std::{borrow::Cow, sync::Arc};

use anyhow::{Context, Error};
use axum::http::request::Parts;
use axum::{
    http::{header, HeaderValue, Method},
    routing::{get, post},
    Router,
};
use config::{Config, File};
use sentry::ClientInitGuard;

use shuttle_axum::ShuttleAxum;
use shuttle_runtime::{
    main as shuttle_main, SecretStore as ShuttleSecretStore, Secrets as ShuttleSecrets,
};
use tower_http::cors::{AllowOrigin, CorsLayer};
use tracing_subscriber::filter::{EnvFilter, LevelFilter};
use tracing_subscriber::prelude::*;
use validator::Validate;

use crate::api::handlers::{alive_handler, send_message_handler};
use crate::configs::AppConfigs;

#[derive(Clone, Debug)]
pub struct AppState {
    configs: AppConfigs,
}

impl AppState {
    pub fn configs(&self) -> &AppConfigs {
        &self.configs
    }
}

fn build_cors_layer(allowed_origins: &[String]) -> CorsLayer {
    let parsed: Vec<_> = allowed_origins
        .into_iter()
        .map(|origin| origin.trim_end_matches('/').to_string())
        .collect();

    let predicate = move |origin: &HeaderValue, _parts: &Parts| {
        if let Ok(origin_str) = origin.to_str() {
            let origin_str = origin_str.trim_end_matches('/');

            for allowed in &parsed {
                if allowed.contains('*') {
                    if let Some(base) = allowed.strip_prefix("https://*.") {
                        if origin_str.starts_with("https://")
                            && origin_str.ends_with(base)
                            && origin_str != format!("https://{}", base)
                        {
                            return true;
                        }
                    } else if let Some(base) = allowed.strip_prefix("http://*.") {
                        if origin_str.starts_with("http://")
                            && origin_str.ends_with(base)
                            && origin_str != format!("http://{}", base)
                        {
                            return true;
                        }
                    }
                } else if origin_str == allowed {
                    return true;
                }
            }
            false
        } else {
            false
        }
    };

    CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(predicate))
        .allow_headers([header::ACCEPT, header::CONTENT_TYPE])
        .allow_methods([Method::GET, Method::HEAD, Method::OPTIONS, Method::POST])
}

fn sentry_init(configs: &AppConfigs) -> ClientInitGuard {
    let dsn = configs.sentry_dsn.as_str();
    let environment = Some(Cow::Owned(configs.sentry_environment.clone()));

    sentry::init((
        dsn,
        sentry::ClientOptions {
            environment,
            release: sentry::release_name!(),
            send_default_pii: true,
            traces_sample_rate: 1.0,
            ..Default::default()
        },
    ))
}

fn tracing_init() {
    let level_filter = if cfg!(debug_assertions) { LevelFilter::DEBUG } else { LevelFilter::ERROR };

    let filter_layer =
        EnvFilter::builder().with_default_directive(level_filter.into()).from_env_lossy();

    let fmt_layer = tracing_subscriber::fmt::layer()
        .compact()
        .with_ansi(true)
        .with_target(false)
        .without_time();

    tracing_subscriber::registry().with(filter_layer).with(fmt_layer).init();
}

#[shuttle_main]
async fn axum(#[ShuttleSecrets] secrets: ShuttleSecretStore) -> ShuttleAxum {
    tracing_init();

    let secrets_source = Config::try_from(&secrets).context("couldn't get the secrets")?;
    let configs = Config::builder()
        .add_source(File::with_name("configs/default").required(true))
        .add_source(secrets_source)
        .build()
        .map_err(|err| {
            tracing::error!(
                "failed to build the config: {:?}",
                Error::msg(err.to_string())
            );
            err
        })
        .context("couldn't build the application config")?
        .try_deserialize::<AppConfigs>()
        .map_err(|err| {
            tracing::error!(
                "failed to deserialize the config: {:?}",
                Error::msg(err.to_string())
            );
            err
        })
        .context("couldn't deserialize the application config")?;

    configs.validate().context("failed to validate the application config")?;

    let _sentry = sentry_init(&configs);

    let app = Router::new()
        .route("/api/v1/alive", get(alive_handler))
        .route("/api/v1/send-message", post(send_message_handler))
        .layer(build_cors_layer(&configs.allow_cors_origins))
        .with_state(Arc::new(AppState { configs }));

    Ok(app.into())
}
