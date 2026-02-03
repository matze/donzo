use std::{net::Ipv4Addr, sync::Arc};

use tracing::info;

use donezo::{auth, create_app, db, AppState};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let port: u16 = std::env::var("DONEZO_PORT")
        .expect("DONEZO_PORT to be set")
        .parse()
        .expect("port number");

    let password = std::env::var("DONEZO_PASSWORD").expect("DONEZO_PASSWORD to be set");

    let base_path = std::env::var("DONEZO_BASE_PATH")
        .ok()
        .map(|path| {
            let path = path.trim_end_matches('/');
            if path.starts_with('/') {
                path.to_string()
            } else {
                format!("/{}", path)
            }
        })
        .unwrap_or_default();

    let password_hash = Arc::new(auth::hash_password(&password));
    let db = db::init_db().expect("initializing database");
    let _ = db::cleanup_expired_sessions(&db);

    let state = AppState {
        db,
        password_hash,
        base_path: Arc::new(base_path),
    };
    let app = create_app(state);
    let addr = (Ipv4Addr::UNSPECIFIED, port);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind to port 3000");

    info!("running on {addr:?}");

    axum::serve(listener, app).await.expect("failed serving");
}
