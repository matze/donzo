pub mod assets;
pub mod auth;
pub mod db;
pub mod error;
pub mod handlers;
pub mod middleware;
pub mod models;

use std::sync::Arc;

use axum::{
    routing::{delete, get, post, put},
    Router,
};
use db::DbPool;

#[derive(Clone)]
pub struct AppState {
    pub db: DbPool,
    pub password_hash: Arc<String>,
    pub base_path: Arc<String>,
}

pub fn create_app(state: AppState) -> Router {
    let base_path = state.base_path.clone();

    let app_routes = Router::new()
        .route("/", get(handlers::web::index))
        .route("/login", get(handlers::web::login_page))
        .route("/static/{*path}", get(handlers::web::static_file))
        .route("/api/login", post(handlers::auth::login))
        .route("/api/logout", post(handlers::auth::logout))
        .route("/api/tokens", get(handlers::auth::list_tokens))
        .route("/api/tokens", post(handlers::auth::create_token))
        .route("/api/tokens/{id}", delete(handlers::auth::revoke_token))
        .route("/api/todos", get(handlers::api::list_all_todos))
        .route("/api/todos", post(handlers::api::create_new_todo))
        .route("/api/todos/reorder", put(handlers::api::reorder))
        .route("/api/todos/plain", get(handlers::api::plain_text_todos))
        .route("/api/todos/{id}", get(handlers::api::get_single_todo))
        .route("/api/todos/{id}", put(handlers::api::update_existing_todo))
        .route(
            "/api/todos/{id}",
            delete(handlers::api::delete_existing_todo),
        )
        .layer(
            tower::ServiceBuilder::new()
                .layer(tower_http::trace::TraceLayer::new_for_http())
                .layer(tower_http::compression::CompressionLayer::new()),
        )
        .with_state(state);

    tracing::info!("base_path: {base_path:?}");

    if base_path.is_empty() {
        app_routes
    } else {
        Router::new().nest(&*base_path, app_routes)
    }
}
