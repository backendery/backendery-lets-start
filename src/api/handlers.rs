use std::fmt::Write;
use std::sync::Arc;

use anyhow::Result;
use axum::{extract::State, Json};
use lettre::{message::header::ContentType, AsyncTransport, Message};
use tokio_retry::{strategy::ExponentialBackoff, Retry};
use tracing::instrument;

use super::errors::{ApiErrorResponse, EmailErrors};
use super::models::LetsStartForm;
use super::{ApiJsonRequest, ApiJsonResponse};

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

    let (configs, mailer) = (state.get_configs(), state.get_mailer());

    let message = match Message::builder()
        .from(mailer.from.clone())
        .to(mailer.to.clone())
        .subject(String::from("Let's start"))
        .header(ContentType::TEXT_PLAIN)
        .body(letter_text)
    {
        Ok(msg) => msg,
        Err(cause) => {
            tracing::error!("Message builder error: {:?}", cause);
            return Err(ApiErrorResponse::EmailErrors(cause.into()));
        }
    };

    let retry_strategy =
        ExponentialBackoff::from_millis(configs.retry_timeout).take(configs.retry_count);

    if let Err(cause) = Retry::spawn(retry_strategy, || async {
        match mailer.transport.send(message.clone()).await {
            Ok(_) => Ok(()),
            Err(cause) => {
                tracing::error!("Smtp transport error: {:?}", cause);
                Err(EmailErrors::SmtpError(cause))
            }
        }
    })
    .await
    {
        tracing::error!("Smtp send retries error: {:?}", cause);
        return Err(ApiErrorResponse::EmailErrors(cause));
    }

    let response = ApiJsonResponse {
        msg: String::from("The message was successfully sent"),
        ..Default::default()
    };

    Ok(Json(response))
}
