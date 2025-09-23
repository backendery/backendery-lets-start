mod api;
mod configs;
mod cors;
mod services;

use std::{borrow::Cow, sync::Arc};

use anyhow::Context;
use axum::{
    http::{header, request::Parts, HeaderValue, Method},
    routing::{get, post}
};
use sentry::ClientInitGuard;
use shuttle_axum::{axum::Router as ShuttleRouter, ShuttleAxum};
use shuttle_runtime::{
    main as shuttle_main, SecretStore as ShuttleSecretStore, Secrets as ShuttleSecrets,
};
use tower_http::cors::{AllowOrigin, CorsLayer};
use tracing_subscriber::{
    filter::{EnvFilter, LevelFilter},
    prelude::*,
};

use crate::{
    api::handlers::{alive_handler, send_message_handler},
    configs::AppConfigs,
    cors::parse_allowed_origins,
    services::mailer::Mailer,
};

#[derive(Clone, Debug)]
pub struct AppState {
    pub configs: AppConfigs,
    pub mailer: Mailer,
}

fn build_cors_layer(allowed_origins: &[String]) -> CorsLayer {
    let parsed = parse_allowed_origins(allowed_origins);

    let predicate = move |origin: &HeaderValue, _parts: &Parts| match origin.to_str() {
        Ok(origin_str) => parsed.iter().any(|allowed| allowed.matches(origin_str)),
        Err(_) => false,
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
            traces_sample_rate: 0.1,
            ..Default::default()
        },
    ))
}

fn tracing_init() {
    let level_filter = if cfg!(debug_assertions) { LevelFilter::DEBUG } else { LevelFilter::INFO };

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

    let configs = AppConfigs::new(secrets).context("couldn't load app configs")?;
    let mailer = Mailer::new(&configs).context("couldn't create mailer")?;

    let _sentry_guard = sentry_init(&configs);

    let app = ShuttleRouter::new()
        .route("/api/v1/alive", get(alive_handler))
        .route("/api/v1/send-message", post(send_message_handler))
        .layer(build_cors_layer(&configs.allow_cors_origins))
        .with_state(Arc::new(AppState { configs, mailer }));

    Ok(app.into())
}
