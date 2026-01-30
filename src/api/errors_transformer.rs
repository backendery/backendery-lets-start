use axum::{
    Json,
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::api::responses::ApiJsonResponse;

/// Middleware that intercepts error responses from tower layers (like governor)
/// and transforms them into our unified API response format
pub async fn transform_errors_middleware(request: Request<Body>, next: Next) -> Response {
    let response = next.run(request).await;
    let status = response.status();

    // Only transform error status codes that come from middleware
    match status {
        StatusCode::REQUEST_TIMEOUT => {
            let api_response = ApiJsonResponse::error("Request timeout", None);
            (status, Json(api_response)).into_response()
        }

        StatusCode::PAYLOAD_TOO_LARGE => {
            let api_response = ApiJsonResponse::error("Request payload too large", None);
            (status, Json(api_response)).into_response()
        }

        StatusCode::TOO_MANY_REQUESTS => {
            // Extract the original error message from the response body
            let (_parts, body) = response.into_parts();

            let message = match axum::body::to_bytes(body, usize::MAX).await {
                Ok(bytes) => String::from_utf8_lossy(&bytes)
                    .trim()
                    .lines()
                    .next()
                    .filter(|x| !x.is_empty())
                    .map(|x| x.to_string())
                    .unwrap_or_else(|| "Too many requests. Please try again later".to_string()),
                Err(_) => "Too many requests. Please try again later".to_string(),
            };

            let api_response = ApiJsonResponse::error(message, None);
            (status, Json(api_response)).into_response()
        }

        StatusCode::INTERNAL_SERVER_ERROR => {
            let api_response = ApiJsonResponse::error("Internal server error", None);
            (status, Json(api_response)).into_response()
        }

        StatusCode::SERVICE_UNAVAILABLE => {
            let api_response = ApiJsonResponse::error("Service temporarily unavailable", None);
            (status, Json(api_response)).into_response()
        }

        // For all other responses (including successful ones and errors
        // from your handlers), pass through unchanged
        _ => response,
    }
}
