mod api;
mod configs;
mod cors;

use std::{borrow::Cow, str::FromStr, sync::Arc, time::Duration};

use anyhow::Context;
use axum::{
    http::{header, request::Parts, HeaderValue, Method},
    routing::{get, post},
};
use config::{Config, File};
use lettre::{message::Mailbox as MailBox, AsyncSmtpTransport, Tokio1Executor};
use sentry::ClientInitGuard;

use shuttle_axum::{axum::Router as ShuttleRouter, ShuttleAxum};
use shuttle_runtime::{
    main as shuttle_main, SecretStore as ShuttleSecretStore, Secrets as ShuttleSecrets,
};
use tower_http::cors::{AllowOrigin, CorsLayer};
use tracing_subscriber::filter::{EnvFilter, LevelFilter};
use tracing_subscriber::prelude::*;

use crate::api::handlers::{alive_handler, send_message_handler};
use crate::configs::AppConfigs;
use crate::cors::parse_allowed_origins;

#[derive(Clone, Debug)]
pub struct Mailer {
    from: MailBox,
    to: MailBox,
    transport: AsyncSmtpTransport<Tokio1Executor>,
}

#[derive(Clone, Debug)]
pub struct AppState {
    configs: AppConfigs,
    mailer: Mailer,
}

impl AppState {
    pub fn get_configs(&self) -> &AppConfigs {
        &self.configs
    }

    pub fn get_mailer(&self) -> &Mailer {
        &self.mailer
    }
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

    let secrets_source = Config::try_from(&secrets).context("couldn't get the secrets")?;
    let configs: AppConfigs = Config::builder()
        .add_source(File::with_name("configs/default").required(true))
        .add_source(secrets_source)
        .build()
        .inspect_err(|err| tracing::error!("config error: {:?}", err))
        .context("couldn't build the application config")?
        .try_into()
        .inspect_err(|err| tracing::error!("config error: {:?}", err))
        .context("invalid or incomplete config")?;

    // Pre-parse mailboxes and construct a reusable SMTP transport once
    let timeout = Some(Duration::from_millis(configs.smtp_connection_timeout));
    let url = format!(
        "smtps://{}@{}",
        configs.smtp_auth.as_str(),
        configs.smtp_addr.as_str()
    );
    let transport = AsyncSmtpTransport::<Tokio1Executor>::from_url(url.as_str())
        .inspect_err(|err| tracing::error!("smtp error: {:?}", err))
        .context("couldn't create SMTP transport from URL")?
        .timeout(timeout)
        .build();
    let from = MailBox::from_str(configs.from_mailbox.as_str())
        .inspect_err(|err| tracing::error!("mailbox error: {:?}", err))
        .context("invalid or incompatible <from>")?;
    let to = MailBox::from_str(configs.to_mailbox.as_str())
        .inspect_err(|err| tracing::error!("mailbox error: {:?}", err))
        .context("invalid or incompatible <to>")?;

    let _sentry_guard = sentry_init(&configs);

    let app = ShuttleRouter::new()
        .route("/api/v1/alive", get(alive_handler))
        .route("/api/v1/send-message", post(send_message_handler))
        .layer(build_cors_layer(&configs.allow_cors_origins))
        .with_state(Arc::new(AppState {
            configs,
            mailer: Mailer { from, to, transport },
        }));

    Ok(app.into())
}
