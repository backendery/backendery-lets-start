mod errors;
pub mod handlers;
mod models;

use std::{borrow::Cow, error::Error};

use axum::{
    async_trait,
    extract::{rejection::JsonRejection, FromRequest, Request},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

use serde::{de::DeserializeOwned, Serialize};
use validator::{Validate, ValidationError, ValidationErrorsKind};

use super::api::errors::{ApiErrorResponse, FieldError};
#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Default, Copy, Clone)]
#[must_use]
pub(crate) struct ApiJsonRequest<T>(pub T);

#[async_trait]
impl<S, T> FromRequest<S> for ApiJsonRequest<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Validate,
    Json<T>: FromRequest<S, Rejection = JsonRejection>,
{
    type Rejection = ApiErrorResponse;

    async fn from_request(rq: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(payload) = Json::<T>::from_request(rq, state).await?;
        payload.validate()?;

        Ok(ApiJsonRequest(payload))
    }
}

#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
#[must_use]
pub(crate) struct ApiJsonResponse {
    msg: String,
    errors: Option<Vec<FieldError>>,
}

impl IntoResponse for ApiErrorResponse {
    fn into_response(self) -> Response {
        let (code, msg, errors) = match self {
            /* Json handling */
            ApiErrorResponse::JsonErrors(err) => match err {
                JsonRejection::JsonDataError(err) => {
                    (StatusCode::BAD_REQUEST, format!("{}", err), None)
                }
                JsonRejection::JsonSyntaxError(err) => {
                    (StatusCode::BAD_REQUEST, format!("{}", err), None)
                }
                _ => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    String::from("Hmm-m! Something is wrong with JSON"),
                    None,
                ),
            },

            /* Validator handling */
            ApiErrorResponse::ValidationErrors(err) => {
                let errors = err
                    .errors()
                    .iter()
                    .map(|err_kind| {
                        let (name, kind) = err_kind;
                        FieldError::new(
                            name,
                            match kind {
                                ValidationErrorsKind::Field(field_errs) => {
                                    validation_errs_to_str_vec(field_errs)
                                }
                                _ => vec![],
                            },
                        )
                    })
                    .collect::<Vec<FieldError>>();
                (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    String::from("Hmm-m! Failed to validate JSON"),
                    Some(errors),
                )
            }

            /* Email handling [connection and sending] */
            ApiErrorResponse::EmailErrors(err) => {
                // Send the error to sentry
                sentry::capture_error(&err);
                // Output the error to the console
                tracing::error!("{}", err.source().unwrap_or(&err));
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    String::from("Uh-oh! Failed to send a message"),
                    None,
                )
            }
        };

        (code, Json(ApiJsonResponse { msg, errors })).into_response()
    }
}

fn validation_errs_to_str_vec(errs: &[ValidationError]) -> Vec<String> {
    errs.iter()
        .map(|err| {
            Cow::Borrowed(&err.message)
                .as_deref()
                .unwrap_or("Missing error description")
                .to_string()
        })
        .collect::<Vec<String>>()
}
