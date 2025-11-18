use serde::Serialize;

use super::errors::FieldError;

#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
#[must_use]
pub struct ApiJsonResponse<T = ApiMessage> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<ApiMeta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<FieldError>>,
}

#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl ApiMeta {
    pub fn with_message(message: impl Into<String>) -> Self {
        Self { message: Some(message.into()) }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiMessage {
    pub message: String,
}

impl ApiMessage {
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl<T> ApiJsonResponse<T> {
    pub fn with_data(data: T) -> Self {
        Self { data: Some(data), meta: None, errors: None }
    }
}

impl ApiJsonResponse<ApiMessage> {
    pub fn message(message: impl Into<String>) -> Self {
        Self::with_data(ApiMessage::new(message))
    }

    pub fn error(message: impl Into<String>, errors: Option<Vec<FieldError>>) -> Self {
        Self { data: None, meta: Some(ApiMeta::with_message(message)), errors }
    }
}
