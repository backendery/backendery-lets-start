use axum::extract::rejection::JsonRejection as JsonErrors;
use convert_case::{Case, Casing};
use lettre::{error::Error as CommonError, transport::smtp::Error as SmtpError};
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
    CommonError(#[from] CommonError),

    #[error(transparent)]
    SmtpError(#[from] SmtpError),
}

#[derive(Debug)]
#[must_use]
pub(super) struct FieldError {
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
