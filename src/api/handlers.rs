use std::sync::Arc;

use axum::{Json, extract::State};
use tracing::instrument;

use crate::{
    AppState,
    api::{errors::ApiErrorResponse, models::LetsStartForm, requests::ApiJsonRequest, responses::StaticResponse},
};

#[instrument(skip_all)]
pub async fn alive_handler() -> Json<StaticResponse> {
    Json(StaticResponse::message("The server is alive and well :)"))
}

#[instrument(skip_all)]
#[rustfmt::skip]
pub async fn send_message_handler(
    State(state): State<Arc<AppState>>, ApiJsonRequest(form): ApiJsonRequest<LetsStartForm>,
) -> Result<Json<StaticResponse>, ApiErrorResponse> {
    state.mailer.send_message(form, &state.app_configs).await?;

    Ok(Json(StaticResponse::message("The message was successfully sent")))
}
