use axum::{
    extract::rejection::JsonRejection as JsonErrors,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use convert_case::{Case, Casing};
use lettre::{error::Error as CommonError, transport::smtp::Error as SmtpError};
use serde::{ser::SerializeStruct, Serialize};
use thiserror::Error;
use validator::{ValidationErrors, ValidationErrorsKind};

use super::responses::ApiJsonResponse;

const NUMBERS_OF_FIELDS_TO_SERIALISE: usize = 2;

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Error)]
pub enum ApiErrorResponse {
    #[error(transparent)]
    JsonErrors(#[from] JsonErrors),

    #[error(transparent)]
    ValidationErrors(#[from] ValidationErrors),

    #[error(transparent)]
    EmailErrors(#[from] EmailErrors),
}

#[derive(Debug, Error)]
pub enum EmailErrors {
    #[error(transparent)]
    CommonError(#[from] CommonError),

    #[error(transparent)]
    SmtpError(#[from] SmtpError),
}

#[derive(Debug)]
#[must_use]
pub struct FieldError {
    pub(super) source: String,
    pub(super) description: Vec<String>,
}

impl FieldError {
    pub(super) fn new(source: &str, description: Vec<String>) -> Self {
        FieldError { source: source.to_string(), description }
    }
}

impl Serialize for FieldError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state =
            serializer.serialize_struct("FieldError", NUMBERS_OF_FIELDS_TO_SERIALISE)?;

        state.serialize_field("source", &self.source.to_case(Case::Camel))?;
        state.serialize_field("description", &self.description)?;

        state.end()
    }
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
                                .map(|err| err.message.as_deref().unwrap_or_default().to_string())
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
