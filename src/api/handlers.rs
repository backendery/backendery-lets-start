use std::sync::Arc;

use axum::{Json, extract::State};
use tracing::instrument;

use crate::{
    AppState,
    api::{
        errors::ApiErrorResponse, models::LetsStartForm, requests::ApiJsonRequest,
        responses::ApiJsonResponse,
    },
};

#[instrument(skip_all)]
pub async fn alive_handler() -> Json<ApiJsonResponse> {
    Json(ApiJsonResponse::message("The server is alive and well :)"))
}

#[instrument(skip_all)]
pub async fn send_message_handler(
    State(state): State<Arc<AppState>>,
    ApiJsonRequest(request): ApiJsonRequest<LetsStartForm>,
) -> Result<Json<ApiJsonResponse>, ApiErrorResponse> {
    state.mailer.send_message(request, &state.configs).await?;

    Ok(Json(ApiJsonResponse::message(
        "The message was successfully sent",
    )))
}
