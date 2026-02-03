use axum::http::{header::AUTHORIZATION, request::Parts, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{extract::FromRequestParts, Json};
use serde_json::json;
use tracing::warn;

use crate::db::{get_api_token_by_value, get_session, DbPool};
use crate::error::AppError;
use crate::AppState;

/// Represents an authenticated request (via session cookie or API token)
pub struct Auth;

/// Represents an authenticated request via session cookie only (no API tokens)
pub struct SessionAuth;

/// Represents an optional authentication status
pub struct MaybeAuth(pub bool);

impl FromRequestParts<AppState> for Auth {
    type Rejection = AuthError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        if check_session(parts, &state.db) {
            return Ok(Auth);
        }

        if check_bearer_token(parts, &state.db)? {
            return Ok(Auth);
        }

        warn!("Unauthorized API access attempt");
        Err(AuthError::Unauthorized)
    }
}

impl FromRequestParts<AppState> for SessionAuth {
    type Rejection = AuthError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        if check_session(parts, &state.db) {
            return Ok(SessionAuth);
        }

        Err(AuthError::Unauthorized)
    }
}

impl FromRequestParts<AppState> for MaybeAuth {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        Ok(MaybeAuth(check_session(parts, &state.db)))
    }
}

fn check_session(parts: &Parts, db: &DbPool) -> bool {
    let cookies = parts
        .headers
        .get_all("cookie")
        .iter()
        .filter_map(|h| h.to_str().ok())
        .flat_map(|s| s.split(';'))
        .filter_map(|s| {
            let mut parts = s.trim().splitn(2, '=');
            Some((parts.next()?, parts.next()?))
        });

    for (name, value) in cookies {
        if name == "session" {
            if let Ok(Some(session)) = get_session(db, value) {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64;
                if session.expires_at > now {
                    return true;
                }
            }
        }
    }
    false
}

fn check_bearer_token(parts: &Parts, db: &DbPool) -> Result<bool, AppError> {
    if let Some(auth_header) = parts.headers.get(AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                return Ok(get_api_token_by_value(db, token)?.is_some());
            }
        }
    }
    Ok(false)
}

pub enum AuthError {
    Unauthorized,
    Internal(String),
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        match self {
            AuthError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "Unauthorized" })),
            )
                .into_response(),
            AuthError::Internal(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": msg })),
            )
                .into_response(),
        }
    }
}

impl From<AppError> for AuthError {
    fn from(err: AppError) -> Self {
        match err {
            AppError::Database(msg) => AuthError::Internal(msg),
            AppError::Unauthorized => AuthError::Unauthorized,
            AppError::NotFound => AuthError::Internal("Not found".to_string()),
            AppError::BadRequest(msg) => AuthError::Internal(msg.to_string()),
        }
    }
}
