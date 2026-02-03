use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::{Html, IntoResponse, Redirect, Response};

use crate::assets::{APP_JS, INDEX_HTML, LOGIN_HTML, OUTPUT_CSS};
use crate::middleware::MaybeAuth;
use crate::AppState;

pub async fn index(MaybeAuth(authenticated): MaybeAuth, State(state): State<AppState>) -> Response {
    if !authenticated {
        let login_path = format!("{}/login", state.base_path);
        return Redirect::to(&login_path).into_response();
    }
    Html(inject_base_path(INDEX_HTML, &state.base_path)).into_response()
}

pub async fn login_page(
    MaybeAuth(authenticated): MaybeAuth,
    State(state): State<AppState>,
) -> Response {
    if authenticated {
        let index_path = if state.base_path.is_empty() {
            "/".to_string()
        } else {
            state.base_path.to_string()
        };
        return Redirect::to(&index_path).into_response();
    }
    Html(inject_base_path(LOGIN_HTML, &state.base_path)).into_response()
}

pub async fn static_file(Path(path): Path<String>) -> Response {
    match path.as_str() {
        "app.js" => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/javascript")],
            APP_JS,
        )
            .into_response(),
        "output.css" => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "text/css")],
            OUTPUT_CSS,
        )
            .into_response(),
        _ => StatusCode::NOT_FOUND.into_response(),
    }
}

fn inject_base_path(html: &str, base_path: &str) -> String {
    // Inject a script tag that sets the BASE_PATH variable
    let script = format!(r#"<script>window.BASE_PATH = "{}";</script>"#, base_path);
    let html = html.replace("<head>", &format!("<head>\n    {}", script));

    // Update static asset paths
    html.replace("href=\"/static/", &format!("href=\"{}/static/", base_path))
        .replace("src=\"/static/", &format!("src=\"{}/static/", base_path))
}
