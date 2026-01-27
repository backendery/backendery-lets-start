use serde::Serialize;
use std::borrow::Cow;

use super::errors::FieldError;

/// Type alias for responses with static string literals (most common case)
/// Use this in handler return types: `Json<StaticResponse>`
pub type StaticResponse<T = StaticMessage> = ApiJsonResponse<'static, T>;

/// Type alias for static message type
pub type StaticMessage = ApiMessage<'static>;

#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
#[must_use]
pub struct ApiJsonResponse<'lt, T = ApiMessage<'lt>> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<ApiMeta<'lt>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<FieldError>>,
}

#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiMeta<'lt> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<Cow<'lt, str>>,
}

impl<'lt> ApiMeta<'lt> {
    pub fn with_message(message: impl Into<Cow<'lt, str>>) -> Self {
        Self { message: Some(message.into()) }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiMessage<'lt> {
    pub message: Cow<'lt, str>,
}

impl<'lt> ApiMessage<'lt> {
    pub fn new(message: impl Into<Cow<'lt, str>>) -> Self {
        Self { message: message.into() }
    }
}

impl<'lt, T> ApiJsonResponse<'lt, T> {
    pub fn with_data(data: T) -> Self {
        Self { data: Some(data), meta: None, errors: None }
    }
}

impl<'lt> ApiJsonResponse<'lt, ApiMessage<'lt>> {
    /// Create a response with a message
    ///
    /// # Zero-allocation for static strings
    /// ```
    /// ApiJsonResponse::message("Static string") // No heap allocation!
    /// ApiJsonResponse::message(format!("Dynamic {}", x)) // Allocates when needed
    /// ```
    pub fn message(message: impl Into<Cow<'lt, str>>) -> Self {
        Self::with_data(ApiMessage::new(message))
    }

    /// Create an error response with optional field errors
    pub fn error(message: impl Into<Cow<'lt, str>>, errors: Option<Vec<FieldError>>) -> Self {
        Self { data: None, meta: Some(ApiMeta::with_message(message)), errors }
    }
}
