use std::sync::Arc;

use axum::{extract::State, Json};
use tracing::instrument;

use crate::{
    api::{
        errors::ApiErrorResponse, models::LetsStartForm, requests::ApiJsonRequest,
        responses::ApiJsonResponse,
    },
    AppState,
};

#[instrument]
pub async fn alive_handler() -> Json<ApiJsonResponse> {
    let response =
        ApiJsonResponse { msg: "The server is alive and well :)".to_string(), details: None };

    Json(response)
}

#[instrument]
pub async fn send_message_handler(
    State(state): State<Arc<AppState>>,
    ApiJsonRequest(request): ApiJsonRequest<LetsStartForm>,
) -> Result<Json<ApiJsonResponse>, ApiErrorResponse> {
    state.mailer.send_message(request, &state.configs).await?;

    let response =
        ApiJsonResponse { msg: "The message was successfully sent".to_string(), details: None };

    Ok(Json(response))
}
