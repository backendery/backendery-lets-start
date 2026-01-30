mod api;
mod configs;
mod cors;
mod ipsec;
mod services;

use std::{
    borrow::Cow,
    path::PathBuf,
    sync::{Arc, OnceLock},
    time::Duration,
};

use anyhow::{Context, Result};
use axum::{
    Router,
    http::{HeaderValue, Method, header, request::Parts},
    middleware,
    routing::{get, post},
};
use clap::Parser;
use sentry::ClientInitGuard;
use tokio::signal;
use tower::limit::ConcurrencyLimitLayer;
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};
use tower_http::{cors::AllowOrigin, cors::CorsLayer, limit::RequestBodyLimitLayer};
use tracing_subscriber::{
    filter::{EnvFilter, LevelFilter},
    prelude::*,
};

use crate::{
    api::{
        errors_transformer::transform_errors_middleware,
        handlers::{alive_handler, send_message_handler},
    },
    configs::AppConfigs,
    cors::CorsMatcher,
    ipsec::SecureIpKeyExtractor,
    services::mailer::Mailer,
};

static SENTRY_GUARD: OnceLock<ClientInitGuard> = OnceLock::new();

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to config file
    #[arg(short, long, default_value = "./configs/default.toml")]
    config_path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct AppState {
    pub app_configs: AppConfigs,
    pub mailer: Mailer,
}

fn build_cors_layer(app_configs: &AppConfigs) -> CorsLayer {
    let layer = CorsLayer::new()
        .allow_headers([header::ACCEPT, header::CONTENT_TYPE, header::AUTHORIZATION])
        .allow_methods([Method::GET, Method::HEAD, Method::OPTIONS, Method::POST]);

    // If there is an "*", we return "any" — this is the fastest way.
    if app_configs
        .allow_cors_origins
        .iter()
        .any(|origin| origin == "*")
    {
        return layer.allow_origin(AllowOrigin::any());
    }

    // Initialize our fast O(1) mapper
    let matcher = CorsMatcher::new(&app_configs.allow_cors_origins);

    // Predicate for Axum/Tower
    let predicate = move |origin: &HeaderValue, _parts: &Parts| {
        origin
            .to_str()
            .map(|origin_str| matcher.matches(origin_str))
            .unwrap_or(false)
    };

    layer.allow_origin(AllowOrigin::predicate(predicate))
}

fn sentry_init(app_configs: &AppConfigs) {
    let dsn = app_configs.sentry_dsn.as_str();
    let environment = Some(Cow::Owned(app_configs.sentry_environment.clone()));

    SENTRY_GUARD.get_or_init(|| {
        sentry::init((
            dsn,
            sentry::ClientOptions {
                environment,
                release: sentry::release_name!(),
                send_default_pii: false,
                traces_sample_rate: 0.1,
                ..Default::default()
            },
        ))
    });
}

fn tracing_init() {
    let level_filter = if cfg!(debug_assertions) { LevelFilter::DEBUG } else { LevelFilter::INFO };

    let filter_layer = EnvFilter::builder()
        .with_default_directive(level_filter.into())
        .from_env_lossy();

    let fmt_layer = tracing_subscriber::fmt::layer()
        .compact()
        .with_ansi(true)
        .with_target(false)
        .without_time();

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .init();
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    tracing_init();

    // Configs
    let app_configs = AppConfigs::new(&args.config_path).context("couldn't load app configs")?;
    let governor_configs = GovernorConfigBuilder::default()
        .period(Duration::from_secs(app_configs.period_seconds_limit))
        .burst_size(app_configs.burst_limit)
        .use_headers()
        .key_extractor(SecureIpKeyExtractor)
        .finish()
        .context("couldn't build governor configs")?;

    let mailer = Mailer::new(&app_configs).context("couldn't create mailer")?;

    sentry_init(&app_configs);

    // Layers
    let cors_layer = build_cors_layer(&app_configs);
    let governor_layer = GovernorLayer::new(Arc::new(governor_configs));

    // Create the listener
    let listener = tokio::net::TcpListener::bind(format!(
        "{host}:{port}",
        host = app_configs.serve_host,
        port = app_configs.serve_port
    ))
    .await
    .context("couldn't bind to address")?;

    // Build the Axum app
    let app = Router::new()
        .route("/api/v1/alive", get(alive_handler))
        .route("/api/v1/send-message", post(send_message_handler))
        /* LAYER ORDER (Outermost -> Innermost):
           1. Concurrency (server resource protection)
           2. Governor (protection against spam/DDoS by IP)
           3. Body Limit (cutting off heavy packets)
           4. CORS (domain verification)
        */
        .layer(cors_layer)
        .layer(RequestBodyLimitLayer::new(app_configs.body_limit * 1024))
        .layer(governor_layer)
        .layer(ConcurrencyLimitLayer::new(app_configs.concurrency_limit))
        .layer(middleware::from_fn(transform_errors_middleware))
        // ... and add global State
        .with_state(Arc::new(AppState { app_configs, mailer }));

    // Serve
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .context("couldn't startup the `axum` server")?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install `Ctrl+C` handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
