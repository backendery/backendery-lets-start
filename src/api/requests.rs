use axum::{
    Json,
    extract::{FromRequest, Request, rejection::JsonRejection},
};
use serde::de::DeserializeOwned;
use validator::Validate;

use super::errors::ApiErrorResponse;

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Default, Copy, Clone)]
#[must_use]
pub struct ApiJsonRequest<T>(pub T);

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
