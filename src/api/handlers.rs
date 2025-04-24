use std::str::FromStr;
use std::sync::Arc;
use std::{fmt::Write, time::Duration};

use anyhow::Result;
use axum::{extract::State, Json};
use lettre::{
    message::{header::ContentType, Mailbox},
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use tokio_retry::{strategy::FixedInterval, Retry};
use tracing::instrument;

use super::errors::{ApiErrorResponse, EmailErrors};
use super::{ApiJsonRequest, ApiJsonResponse};
use super::models::LetsStartForm;

use crate::AppState;

#[instrument]
pub async fn alive_handler() -> Result<Json<ApiJsonResponse>, ApiErrorResponse> {
    let response = ApiJsonResponse {
        msg: String::from("The server is alive and well :)"),
        ..Default::default()
    };

    Ok(Json(response))
}

#[instrument]
pub async fn send_message_handler(
    State(state): State<Arc<AppState>>,
    ApiJsonRequest(request): ApiJsonRequest<LetsStartForm>,
) -> Result<Json<ApiJsonResponse>, ApiErrorResponse> {
    /* Estimate the size of the summary str */
    let mut letter_text = String::with_capacity(1_024);

    write!(
        &mut letter_text,
        r#"
Hey,

I hope this message finds you well. My name is {}.
I would like to discuss a potential collaboration with you on an upcoming project.

A brief overview of the project:
• {}
• Our budget ranges from {} to {} U.S. dollars

If you are interested in discussing this opportunity further, please, reach out to me
at {} email address.

Looking forward to your response.

Regards.
        "#,
        request.name,
        request.project_description,
        request.min_budget,
        request.max_budget,
        request.email
    )
    .unwrap();
    letter_text = letter_text.trim().to_string();

    let configs = state.configs();

    let retry_strategy =
        FixedInterval::from_millis(configs.retry_timeout).take(configs.retry_count);

    let message = match Message::builder()
        .from(Mailbox::from_str(configs.message_from_email.as_str()).unwrap())
        .to(Mailbox::from_str(configs.message_to_email.as_str()).unwrap())
        .subject(String::from("Let's start"))
        .header(ContentType::TEXT_PLAIN)
        .body(letter_text)
    {
        Ok(msg) => msg,
        Err(cause) => return Err(ApiErrorResponse::EmailErrors(cause.into())),
    };

    let timeout = Some(Duration::from_millis(configs.smtp_connection_timeout));
    let url = format!(
        "smtps://{}@{}",
        configs.smtp_auth.as_str(),
        configs.smtp_addr.as_str()
    );

    let mailer = match AsyncSmtpTransport::<Tokio1Executor>::from_url(url.as_str()) {
        Ok(transport) => transport.timeout(timeout).build(),
        Err(cause) => return Err(ApiErrorResponse::EmailErrors(cause.into())),
    };

    if let Err(cause) = Retry::spawn(retry_strategy, || async {
        match mailer.send(message.clone()).await {
            Ok(_) => Ok(()),
            Err(cause) => Err(EmailErrors::SmtpError(cause)),
        }
    })
    .await
    {
        return Err(ApiErrorResponse::EmailErrors(cause));
    }

    let response =
        ApiJsonResponse { msg: String::from("Message successfully sent"), ..Default::default() };

    Ok(Json(response))
}
