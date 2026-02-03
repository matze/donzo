use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use axum_extra::extract::{
    cookie::{Cookie, SameSite},
    CookieJar,
};
use serde_json::json;
use tracing::info;

use crate::auth::{generate_session_id, generate_token, verify_password};
use crate::db::{create_api_token, create_session, delete_api_token, delete_session, list_api_tokens};
use crate::error::AppError;
use crate::middleware::SessionAuth;
use crate::models::{CreateApiToken, LoginRequest, Session};
use crate::AppState;

pub async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(req): Json<LoginRequest>,
) -> Result<(CookieJar, impl IntoResponse), AppError> {
    if !verify_password(&req.password, &state.password_hash) {
        return Err(AppError::Unauthorized);
    }

    let session_id = generate_session_id();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let expires_at = now + 7 * 24 * 60 * 60;

    let session = Session {
        id: session_id.clone(),
        created_at: now,
        expires_at,
    };

    create_session(&state.db, &session)?;
    info!("User logged in");

    let cookie = Cookie::build(("session", session_id))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Strict)
        .max_age(time::Duration::days(7));

    Ok((jar.add(cookie), Json(json!({ "success": true }))))
}

pub async fn logout(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<(CookieJar, impl IntoResponse), AppError> {
    if let Some(session_cookie) = jar.get("session") {
        delete_session(&state.db, session_cookie.value())?;
    }
    info!("User logged out");

    let cookie = Cookie::build(("session", ""))
        .path("/")
        .http_only(true)
        .max_age(time::Duration::seconds(0));

    Ok((jar.remove(cookie), Json(json!({ "success": true }))))
}

pub async fn list_tokens(
    _auth: SessionAuth,
    State(state): State<AppState>,
) -> Result<Json<Vec<crate::models::ApiToken>>, AppError> {
    let tokens = list_api_tokens(&state.db)?;
    Ok(Json(tokens))
}

pub async fn create_token(
    _auth: SessionAuth,
    State(state): State<AppState>,
    Json(req): Json<CreateApiToken>,
) -> Result<Json<crate::models::ApiToken>, AppError> {
    let token_value = generate_token();
    let token = create_api_token(&state.db, &token_value, req.name.as_deref())?;
    info!(name = ?req.name, "Created API token");
    Ok(Json(token))
}

pub async fn revoke_token(
    _auth: SessionAuth,
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    if delete_api_token(&state.db, id)? {
        info!(id, "Revoked API token");
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound)
    }
}
