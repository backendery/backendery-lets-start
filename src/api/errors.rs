use axum::extract::rejection::JsonRejection as JsonErrors;
use convert_case::{Case, Casing};
use lettre::{error::Error as OmniError, transport::smtp::Error as SmtpError};
use serde::{ser::SerializeStruct, Serialize};
use thiserror::Error;
use validator::ValidationErrors;

const NUMBERS_OF_FIELDS_TO_SERIALISE: usize = 2;

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Error)]
pub(crate) enum ApiErrorResponse {
    #[error(transparent)]
    JsonErrors(#[from] JsonErrors),

    #[error(transparent)]
    ValidationErrors(#[from] ValidationErrors),

    #[error(transparent)]
    EmailErrors(#[from] EmailErrors),
}

#[derive(Debug, Error)]
pub(crate) enum EmailErrors {
    #[error(transparent)]
    OmniError(#[from] OmniError),

    #[error(transparent)]
    SmtpError(#[from] SmtpError),
}

#[derive(Debug)]
#[must_use]
pub(super) struct FieldError {
    pub(super) field: String,
    pub(super) field_errors: Vec<String>,
}

impl FieldError {
    pub(super) fn new(field: &str, field_errors: Vec<String>) -> Self {
        FieldError { field: String::from(field), field_errors }
    }
}

impl Serialize for FieldError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("FieldError", NUMBERS_OF_FIELDS_TO_SERIALISE)?;

        state.serialize_field("field", &self.field.to_case(Case::Camel))?;
        state.serialize_field("fieldErrors", &self.field_errors)?;

        state.end()
    }
}
