mod errors;
pub mod handlers;
mod models;

use axum::{
    extract::{rejection::JsonRejection, FromRequest, Request},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

use serde::{de::DeserializeOwned, Serialize};
use validator::{Validate, ValidationErrorsKind};

use super::api::errors::{ApiErrorResponse, FieldError};

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Default, Copy, Clone)]
#[must_use]
pub(crate) struct ApiJsonRequest<T>(pub T);

impl<S, T> FromRequest<S> for ApiJsonRequest<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Validate,
    Json<T>: FromRequest<S, Rejection = JsonRejection>,
{
    type Rejection = ApiErrorResponse;

    async fn from_request(rq: Request, state: &S) -> Result<Self, Self::Rejection> {
        // First, parse the JSON
        let Json(payload) = Json::<T>::from_request(rq, state).await?;
        // ... then validate
        payload.validate()?;

        Ok(ApiJsonRequest(payload))
    }
}

#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
#[must_use]
pub(crate) struct ApiJsonResponse {
    msg: String,
    details: Option<Vec<FieldError>>,
}

impl IntoResponse for ApiErrorResponse {
    fn into_response(self) -> Response {
        // Constants for error messages
        const JSON_ERROR_MSG: &str = "Invalid JSON format";
        const VALIDATION_ERROR_MSG: &str = "Invalid JSON validation";
        const EMAIL_ERROR_MSG: &str = "Unable to send email";

        let (status_code, msg, details) = match self {
            /* Json handling */
            ApiErrorResponse::JsonErrors(err) => {
                let errors = vec![FieldError::new(
                    "$body",
                    vec![capitalize(err.to_string().split(" at line").next().unwrap_or_default())],
                )];

                (
                    StatusCode::BAD_REQUEST,
                    JSON_ERROR_MSG.to_string(),
                    Some(errors),
                )
            }

            /* Validator handling */
            ApiErrorResponse::ValidationErrors(err) => {
                let errors = err
                    .errors()
                    .iter()
                    .map(|err_kind| {
                        let (source, validation_errs_kind) = err_kind;
                        let description = match validation_errs_kind {
                            ValidationErrorsKind::Field(field_errs) => field_errs
                                .iter()
                                .map(|err| {
                                    err.message
                                        .as_deref()
                                        .unwrap_or_default()
                                        .to_string()
                                })
                                .collect(),
                            _ => vec![],
                        };
                        FieldError::new(source, description)
                    })
                    .collect::<Vec<FieldError>>();

                (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    VALIDATION_ERROR_MSG.to_string(),
                    Some(errors),
                )
            }

            /* Email handling [connection and sending] */
            ApiErrorResponse::EmailErrors(err) => {
                // Send the error to sentry
                sentry::capture_error(&err);

                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    EMAIL_ERROR_MSG.to_string(),
                    None,
                )
            }
        };

        (status_code, Json(ApiJsonResponse { msg, details })).into_response()
    }
}

#[inline(always)]
fn capitalize(text: &str) -> String {
    let mut char = text.chars();
    match char.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + char.as_str(),
    }
}
