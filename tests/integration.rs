use std::sync::{Arc, Mutex};

use reqwest::{Client, StatusCode};
use rusqlite::Connection;
use serde_json::{json, Value};
use tokio::net::TcpListener;

use donezo::{auth, create_app, AppState};

struct TestServer {
    addr: String,
    client: Client,
}

impl TestServer {
    async fn new() -> Self {
        // Create in-memory database for testing
        let conn = Connection::open_in_memory().expect("Failed to create in-memory database");
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                created_at INTEGER DEFAULT (strftime('%s', 'now')),
                expires_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS api_tokens (
                id INTEGER PRIMARY KEY,
                token TEXT UNIQUE NOT NULL,
                name TEXT,
                created_at INTEGER DEFAULT (strftime('%s', 'now'))
            );

            CREATE TABLE IF NOT EXISTS todos (
                id INTEGER PRIMARY KEY,
                title TEXT NOT NULL,
                completed INTEGER DEFAULT 0,
                position INTEGER DEFAULT 0,
                created_at INTEGER DEFAULT (strftime('%s', 'now')),
                updated_at INTEGER DEFAULT (strftime('%s', 'now'))
            );
            ",
        )
        .expect("Failed to create tables");

        let db = Arc::new(Mutex::new(conn));
        let password_hash = Arc::new(auth::hash_password("testpassword"));
        let base_path = Arc::new(String::new());

        let state = AppState {
            db,
            password_hash,
            base_path,
        };
        let app = create_app(state);

        // Bind to random available port
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("Failed to bind");
        let addr = format!("http://{}", listener.local_addr().unwrap());

        // Spawn server in background
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        // Create client with cookie store
        let client = Client::builder()
            .cookie_store(true)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("Failed to create client");

        TestServer { addr, client }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.addr, path)
    }
}

#[tokio::test]
async fn test_unauthenticated_redirect_to_login() {
    let server = TestServer::new().await;

    let resp = server.client.get(server.url("/")).send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::SEE_OTHER);
    assert_eq!(resp.headers().get("location").unwrap(), "/login");
}

#[tokio::test]
async fn test_login_page_accessible() {
    let server = TestServer::new().await;

    let resp = server
        .client
        .get(server.url("/login"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.text().await.unwrap();
    assert!(body.contains("Login"));
}

#[tokio::test]
async fn test_static_assets() {
    let server = TestServer::new().await;

    // Test app.js
    let resp = server
        .client
        .get(server.url("/static/app.js"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert!(resp
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap()
        .contains("javascript"));

    // Test output.css
    let resp = server
        .client
        .get(server.url("/static/output.css"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert!(resp
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap()
        .contains("css"));

    // Test 404 for unknown static file
    let resp = server
        .client
        .get(server.url("/static/unknown.txt"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_login_wrong_password() {
    let server = TestServer::new().await;

    let resp = server
        .client
        .post(server.url("/api/login"))
        .json(&json!({"password": "wrongpassword"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_login_success() {
    let server = TestServer::new().await;

    let resp = server
        .client
        .post(server.url("/api/login"))
        .json(&json!({"password": "testpassword"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["success"], true);
}

#[tokio::test]
async fn test_todos_unauthenticated() {
    let server = TestServer::new().await;

    let resp = server
        .client
        .get(server.url("/api/todos"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_todo_crud() {
    let server = TestServer::new().await;

    // Login first
    let resp = server
        .client
        .post(server.url("/api/login"))
        .json(&json!({"password": "testpassword"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // List todos (should be empty)
    let resp = server
        .client
        .get(server.url("/api/todos"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let todos: Vec<Value> = resp.json().await.unwrap();
    assert!(todos.is_empty());

    // Create a todo
    let resp = server
        .client
        .post(server.url("/api/todos"))
        .json(&json!({"title": "Buy groceries"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let todo: Value = resp.json().await.unwrap();
    assert_eq!(todo["title"], "Buy groceries");
    assert_eq!(todo["completed"], false);
    let todo_id = todo["id"].as_i64().unwrap();

    // Get the todo
    let resp = server
        .client
        .get(server.url(&format!("/api/todos/{}", todo_id)))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let todo: Value = resp.json().await.unwrap();
    assert_eq!(todo["title"], "Buy groceries");

    // Update the todo title
    let resp = server
        .client
        .put(server.url(&format!("/api/todos/{}", todo_id)))
        .json(&json!({"title": "Buy groceries today"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let todo: Value = resp.json().await.unwrap();
    assert_eq!(todo["title"], "Buy groceries today");

    // List todos (should have one open todo)
    let resp = server
        .client
        .get(server.url("/api/todos"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let todos: Vec<Value> = resp.json().await.unwrap();
    assert_eq!(todos.len(), 1);

    // Mark todo as completed
    let resp = server
        .client
        .put(server.url(&format!("/api/todos/{}", todo_id)))
        .json(&json!({"completed": true}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let todo: Value = resp.json().await.unwrap();
    assert_eq!(todo["completed"], true);

    // List todos (should still contain the completed todo)
    let resp = server
        .client
        .get(server.url("/api/todos"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let todos: Vec<Value> = resp.json().await.unwrap();
    assert_eq!(todos.len(), 1);
    assert_eq!(todos[0]["completed"], true);
}

#[tokio::test]
async fn test_todo_not_found() {
    let server = TestServer::new().await;

    // Login
    server
        .client
        .post(server.url("/api/login"))
        .json(&json!({"password": "testpassword"}))
        .send()
        .await
        .unwrap();

    // Get non-existent todo
    let resp = server
        .client
        .get(server.url("/api/todos/9999"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // Update non-existent todo
    let resp = server
        .client
        .put(server.url("/api/todos/9999"))
        .json(&json!({"title": "Test"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // Delete non-existent todo
    let resp = server
        .client
        .delete(server.url("/api/todos/9999"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_todo_empty_title_rejected() {
    let server = TestServer::new().await;

    // Login
    server
        .client
        .post(server.url("/api/login"))
        .json(&json!({"password": "testpassword"}))
        .send()
        .await
        .unwrap();

    // Create todo with empty title
    let resp = server
        .client
        .post(server.url("/api/todos"))
        .json(&json!({"title": "   "}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_plain_text_todos() {
    let server = TestServer::new().await;

    // Login
    server
        .client
        .post(server.url("/api/login"))
        .json(&json!({"password": "testpassword"}))
        .send()
        .await
        .unwrap();

    // Create some todos
    server
        .client
        .post(server.url("/api/todos"))
        .json(&json!({"title": "Buy groceries"}))
        .send()
        .await
        .unwrap();

    let resp = server
        .client
        .post(server.url("/api/todos"))
        .json(&json!({"title": "Fix bike"}))
        .send()
        .await
        .unwrap();
    let todo: Value = resp.json().await.unwrap();
    let fix_bike_id = todo["id"].as_i64().unwrap();

    server
        .client
        .post(server.url("/api/todos"))
        .json(&json!({"title": "Call mom"}))
        .send()
        .await
        .unwrap();

    // Mark one as completed
    server
        .client
        .put(server.url(&format!("/api/todos/{}", fix_bike_id)))
        .json(&json!({"completed": true}))
        .send()
        .await
        .unwrap();

    // Get plain text (should only show open todos)
    let resp = server
        .client
        .get(server.url("/api/todos/plain"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert!(resp
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap()
        .contains("text/plain"));

    let body = resp.text().await.unwrap();
    assert!(body.contains("Buy groceries"));
    assert!(body.contains("Call mom"));
    assert!(!body.contains("Fix bike")); // Completed, should not appear
}

#[tokio::test]
async fn test_api_tokens() {
    let server = TestServer::new().await;

    // Login
    server
        .client
        .post(server.url("/api/login"))
        .json(&json!({"password": "testpassword"}))
        .send()
        .await
        .unwrap();

    // List tokens (should be empty)
    let resp = server
        .client
        .get(server.url("/api/tokens"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let tokens: Vec<Value> = resp.json().await.unwrap();
    assert!(tokens.is_empty());

    // Create a token
    let resp = server
        .client
        .post(server.url("/api/tokens"))
        .json(&json!({"name": "Test Token"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let token: Value = resp.json().await.unwrap();
    assert_eq!(token["name"], "Test Token");
    let token_value = token["token"].as_str().unwrap().to_string();
    let token_id = token["id"].as_i64().unwrap();
    assert_eq!(token_value.len(), 64); // 64 character token

    // List tokens (should have one)
    let resp = server
        .client
        .get(server.url("/api/tokens"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let tokens: Vec<Value> = resp.json().await.unwrap();
    assert_eq!(tokens.len(), 1);

    // Revoke the token
    let resp = server
        .client
        .delete(server.url(&format!("/api/tokens/{}", token_id)))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // List tokens (should be empty)
    let resp = server
        .client
        .get(server.url("/api/tokens"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let tokens: Vec<Value> = resp.json().await.unwrap();
    assert!(tokens.is_empty());
}

#[tokio::test]
async fn test_api_token_authentication() {
    let server = TestServer::new().await;

    // Login to create a token
    server
        .client
        .post(server.url("/api/login"))
        .json(&json!({"password": "testpassword"}))
        .send()
        .await
        .unwrap();

    // Create a token
    let resp = server
        .client
        .post(server.url("/api/tokens"))
        .json(&json!({"name": "API Token"}))
        .send()
        .await
        .unwrap();
    let token: Value = resp.json().await.unwrap();
    let token_value = token["token"].as_str().unwrap();

    // Create a new client without cookies
    let new_client = Client::builder()
        .cookie_store(false)
        .build()
        .expect("Failed to create client");

    // Try to access todos without auth (should fail)
    let resp = new_client
        .get(server.url("/api/todos"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // Access with Bearer token (should succeed)
    let resp = new_client
        .get(server.url("/api/todos"))
        .header("Authorization", format!("Bearer {}", token_value))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Create a todo with Bearer token
    let resp = new_client
        .post(server.url("/api/todos"))
        .header("Authorization", format!("Bearer {}", token_value))
        .json(&json!({"title": "API created todo"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Access plain text with Bearer token
    let resp = new_client
        .get(server.url("/api/todos/plain"))
        .header("Authorization", format!("Bearer {}", token_value))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.text().await.unwrap();
    assert!(body.contains("API created todo"));
}

#[tokio::test]
async fn test_logout() {
    let server = TestServer::new().await;

    // Login
    let resp = server
        .client
        .post(server.url("/api/login"))
        .json(&json!({"password": "testpassword"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Access todos (should succeed)
    let resp = server
        .client
        .get(server.url("/api/todos"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Logout
    let resp = server
        .client
        .post(server.url("/api/logout"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Access todos after logout (should fail)
    let resp = server
        .client
        .get(server.url("/api/todos"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_authenticated_user_redirected_from_login() {
    let server = TestServer::new().await;

    // Login
    server
        .client
        .post(server.url("/api/login"))
        .json(&json!({"password": "testpassword"}))
        .send()
        .await
        .unwrap();

    // Try to access login page (should redirect to /)
    let resp = server
        .client
        .get(server.url("/login"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::SEE_OTHER);
    assert_eq!(resp.headers().get("location").unwrap(), "/");
}

#[tokio::test]
async fn test_authenticated_access_to_index() {
    let server = TestServer::new().await;

    // Login
    server
        .client
        .post(server.url("/api/login"))
        .json(&json!({"password": "testpassword"}))
        .send()
        .await
        .unwrap();

    // Access index page (should succeed)
    let resp = server.client.get(server.url("/")).send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.text().await.unwrap();
    assert!(body.contains("Tasks"));
}
