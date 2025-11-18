use askama::Error as TemplateError;
use axum::{
    Json,
    extract::rejection::JsonRejection as JsonErrors,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use convert_case::{Case, Casing};
use lettre::{error::Error as CommonError, transport::smtp::Error as SmtpError};
use serde::{Serialize, ser::SerializeStruct};
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

    #[error(transparent)]
    TemplateError(#[from] TemplateError),
}

#[derive(Debug)]
#[must_use]
pub struct FieldError {
    pub(super) source: String,
    pub(super) description: Vec<String>,
}

impl FieldError {
    pub(super) fn new(source: impl Into<String>, description: Vec<String>) -> Self {
        FieldError { source: source.into(), description }
    }

    pub(super) fn prepend_source(&mut self, prefix: &str) {
        if prefix.is_empty() {
            return;
        }

        if self.source.is_empty() {
            self.source = prefix.to_string();
        } else {
            self.source = format!("{prefix}.{}", self.source);
        }
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

        let (status_code, response) = match self {
            /* Json handling */
            ApiErrorResponse::JsonErrors(err) => {
                let errors = vec![FieldError::new(
                    "$body",
                    vec![capitalize(err.to_string().split(" at line").next().unwrap_or_default())],
                )];

                (
                    StatusCode::BAD_REQUEST,
                    ApiJsonResponse::error(JSON_ERROR_MSG, Some(errors)),
                )
            }

            /* Validator handling */
            ApiErrorResponse::ValidationErrors(err) => {
                let mut errors = collect_field_errors(&err);
                if errors.is_empty() {
                    errors.push(FieldError::new("$schema", vec!["invalid payload".into()]));
                }

                (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    ApiJsonResponse::error(VALIDATION_ERROR_MSG, Some(errors)),
                )
            }

            /* Email handling [connection and sending] */
            ApiErrorResponse::EmailErrors(err) => {
                // Send the error to sentry
                sentry::capture_error(&err);

                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ApiJsonResponse::error(EMAIL_ERROR_MSG, None),
                )
            }
        };

        (status_code, Json(response)).into_response()
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

fn collect_field_errors(errors: &ValidationErrors) -> Vec<FieldError> {
    let mut collected = Vec::new();

    for (source, kind) in errors.errors() {
        match kind {
            ValidationErrorsKind::Field(field_errs) => {
                let description = field_errs
                    .iter()
                    .map(|err| {
                        err.message.as_deref().unwrap_or_else(|| err.code.as_ref()).to_string()
                    })
                    .collect::<Vec<_>>();
                collected.push(FieldError::new(normalize_source(source), description));
            }
            ValidationErrorsKind::Struct(struct_errs) => {
                let nested = collect_field_errors(struct_errs);
                if nested.is_empty() {
                    collected.push(FieldError::new(
                        normalize_source(source),
                        vec!["invalid value".into()],
                    ));
                    continue;
                }

                for mut err in nested {
                    err.prepend_source(source);
                    collected.push(err);
                }
            }
            ValidationErrorsKind::List(list_errs) => {
                for (idx, item_errs) in list_errs {
                    for mut err in collect_field_errors(item_errs) {
                        let prefix = if source.is_empty() {
                            format!("[{idx}]")
                        } else {
                            format!("{source}[{idx}]")
                        };
                        err.prepend_source(&prefix);
                        collected.push(err);
                    }
                }
            }
        }
    }

    collected
}

fn normalize_source(source: &str) -> String {
    if source.is_empty() { "$schema".to_string() } else { source.to_string() }
}
