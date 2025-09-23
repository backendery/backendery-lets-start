use serde::Serialize;

use super::errors::FieldError;

#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
#[must_use]
pub struct ApiJsonResponse {
    pub msg: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Vec<FieldError>>,
}
