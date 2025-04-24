mod api;
mod configs;

use std::{borrow::Cow, sync::Arc};

use anyhow::{Context, Error};
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
use tower_http::cors::CorsLayer;
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
    let level_filter = if cfg!(debug_assertions) {
        LevelFilter::DEBUG
    } else {
        LevelFilter::ERROR
    };

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
            tracing::error!("failed to build the config: {:?}", Error::msg(err.to_string()));
            err
        })
        .context("couldn't build the application config")?
        .try_deserialize::<AppConfigs>()
        .map_err(|err| {
            tracing::error!("failed to deserialize the config: {:?}", Error::msg(err.to_string()));
            err
        })
        .context("couldn't deserialize the application config")?;

    configs.validate().context("failed to validate the application config")?;

    let _sentry = sentry_init(&configs);

    let app = Router::new()
        .route("/api/v1/alive", get(alive_handler))
        .route("/api/v1/send-message", post(send_message_handler))
        .layer(
            CorsLayer::new()
                .allow_origin(
                    configs
                        .allow_cors_origins
                        .iter()
                        .map(|header| header.parse::<HeaderValue>().unwrap())
                        .collect::<Vec<_>>(),
                )
                .allow_headers([header::ACCEPT, header::CONTENT_TYPE])
                .allow_methods([Method::HEAD, Method::GET, Method::POST]),
        )
        .with_state(Arc::new(AppState { configs }));

    Ok(app.into())
}
