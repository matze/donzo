use axum::extract::{Path, State};
use axum::{http::StatusCode, response::IntoResponse, Json};
use tracing::info;

use crate::db::{
    create_todo, delete_todo, get_todo, list_open_todos, list_todos, reorder_todos, update_todo,
};
use crate::error::AppError;
use crate::middleware::Auth;
use crate::models::{CreateTodo, ReorderTodos, Todo, UpdateTodo};
use crate::AppState;

pub async fn list_all_todos(
    _auth: Auth,
    State(state): State<AppState>,
) -> Result<Json<Vec<Todo>>, AppError> {
    let todos = list_todos(&state.db)?;
    info!(count = todos.len(), "Listed todos");
    Ok(Json(todos))
}

pub async fn create_new_todo(
    _auth: Auth,
    State(state): State<AppState>,
    Json(req): Json<CreateTodo>,
) -> Result<(StatusCode, Json<Todo>), AppError> {
    if req.title.trim().is_empty() {
        return Err(AppError::BadRequest("Title cannot be empty"));
    }

    let todo = create_todo(&state.db, &req.title)?;
    info!(id = todo.id, title = %todo.title, "Created todo");
    Ok((StatusCode::CREATED, Json(todo)))
}

pub async fn get_single_todo(
    _auth: Auth,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Todo>, AppError> {
    match get_todo(&state.db, id)? {
        Some(todo) => Ok(Json(todo)),
        None => Err(AppError::NotFound),
    }
}

pub async fn update_existing_todo(
    _auth: Auth,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateTodo>,
) -> Result<Json<Todo>, AppError> {
    if let Some(ref title) = req.title {
        if title.trim().is_empty() {
            return Err(AppError::BadRequest("Title cannot be empty"));
        }
    }

    match update_todo(&state.db, id, req.title.as_deref(), req.completed)? {
        Some(todo) => {
            info!(id = todo.id, completed = todo.completed, "Updated todo");
            Ok(Json(todo))
        }
        None => Err(AppError::NotFound),
    }
}

pub async fn delete_existing_todo(
    _auth: Auth,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    if delete_todo(&state.db, id)? {
        info!(id, "Deleted todo");
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound)
    }
}

pub async fn reorder(
    _auth: Auth,
    State(state): State<AppState>,
    Json(req): Json<ReorderTodos>,
) -> Result<Json<Vec<Todo>>, AppError> {
    reorder_todos(&state.db, &req.ids)?;
    let todos = list_todos(&state.db)?;
    info!("Reordered todos");
    Ok(Json(todos))
}

pub async fn plain_text_todos(
    _auth: Auth,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let todos = list_open_todos(&state.db)?;
    let text: String = todos.iter().map(|t| format!("{}\n", t.title)).collect();

    Ok((
        StatusCode::OK,
        [("content-type", "text/plain; charset=utf-8")],
        text,
    ))
}
