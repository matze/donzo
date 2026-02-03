use std::sync::{Arc, Mutex};

use rusqlite::{Connection, Result};

use crate::error::AppError;
use crate::models::{ApiToken, Session, Todo};

pub type DbPool = Arc<Mutex<Connection>>;

pub fn init_db() -> Result<DbPool> {
    let conn = Connection::open("todos.db")?;

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
    )?;

    // Migration: add position column if it doesn't exist
    let has_position: bool = conn.prepare("SELECT position FROM todos LIMIT 1").is_ok();
    if !has_position {
        conn.execute(
            "ALTER TABLE todos ADD COLUMN position INTEGER DEFAULT 0",
            [],
        )?;
        // Initialize positions based on created_at order
        conn.execute(
            "UPDATE todos SET position = (
                SELECT COUNT(*) FROM todos t2 WHERE t2.created_at <= todos.created_at
            )",
            [],
        )?;
    }

    Ok(Arc::new(Mutex::new(conn)))
}

// Session operations
pub fn create_session(pool: &DbPool, session: &Session) -> Result<(), AppError> {
    let conn = pool.lock().unwrap();
    conn.execute(
        "INSERT INTO sessions (id, expires_at) VALUES (?1, ?2)",
        (&session.id, session.expires_at),
    )?;
    Ok(())
}

pub fn get_session(pool: &DbPool, id: &str) -> Result<Option<Session>, AppError> {
    let conn = pool.lock().unwrap();
    let mut stmt = conn.prepare("SELECT id, created_at, expires_at FROM sessions WHERE id = ?1")?;
    let mut rows = stmt.query([id])?;

    if let Some(row) = rows.next()? {
        Ok(Some(Session {
            id: row.get(0)?,
            created_at: row.get(1)?,
            expires_at: row.get(2)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn delete_session(pool: &DbPool, id: &str) -> Result<(), AppError> {
    let conn = pool.lock().unwrap();
    conn.execute("DELETE FROM sessions WHERE id = ?1", [id])?;
    Ok(())
}

pub fn cleanup_expired_sessions(pool: &DbPool) -> Result<(), AppError> {
    let conn = pool.lock().unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    conn.execute("DELETE FROM sessions WHERE expires_at < ?1", [now])?;
    Ok(())
}

// API Token operations
pub fn create_api_token(
    pool: &DbPool,
    token: &str,
    name: Option<&str>,
) -> Result<ApiToken, AppError> {
    let conn = pool.lock().unwrap();
    conn.execute(
        "INSERT INTO api_tokens (token, name) VALUES (?1, ?2)",
        (token, name),
    )?;
    let id = conn.last_insert_rowid();

    let mut stmt =
        conn.prepare("SELECT id, token, name, created_at FROM api_tokens WHERE id = ?1")?;
    let token = stmt.query_row([id], |row| {
        Ok(ApiToken {
            id: row.get(0)?,
            token: row.get(1)?,
            name: row.get(2)?,
            created_at: row.get(3)?,
        })
    })?;

    Ok(token)
}

pub fn get_api_token_by_value(pool: &DbPool, token: &str) -> Result<Option<ApiToken>, AppError> {
    let conn = pool.lock().unwrap();
    let mut stmt =
        conn.prepare("SELECT id, token, name, created_at FROM api_tokens WHERE token = ?1")?;
    let mut rows = stmt.query([token])?;

    if let Some(row) = rows.next()? {
        Ok(Some(ApiToken {
            id: row.get(0)?,
            token: row.get(1)?,
            name: row.get(2)?,
            created_at: row.get(3)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn list_api_tokens(pool: &DbPool) -> Result<Vec<ApiToken>, AppError> {
    let conn = pool.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT id, token, name, created_at FROM api_tokens ORDER BY created_at DESC")?;
    let tokens = stmt
        .query_map([], |row| {
            Ok(ApiToken {
                id: row.get(0)?,
                token: row.get(1)?,
                name: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(tokens)
}

pub fn delete_api_token(pool: &DbPool, id: i64) -> Result<bool, AppError> {
    let conn = pool.lock().unwrap();
    let rows = conn.execute("DELETE FROM api_tokens WHERE id = ?1", [id])?;
    Ok(rows > 0)
}

// Todo operations
pub fn create_todo(pool: &DbPool, title: &str) -> Result<Todo, AppError> {
    let conn = pool.lock().unwrap();

    // Get max position and add 1
    let max_pos: i64 = conn
        .query_row("SELECT COALESCE(MAX(position), 0) FROM todos", [], |row| {
            row.get(0)
        })
        .unwrap_or(0);

    conn.execute(
        "INSERT INTO todos (title, position) VALUES (?1, ?2)",
        (title, max_pos + 1),
    )?;
    let id = conn.last_insert_rowid();

    let mut stmt = conn.prepare(
        "SELECT id, title, completed, position, created_at, updated_at FROM todos WHERE id = ?1",
    )?;
    let todo = stmt.query_row([id], |row| {
        Ok(Todo {
            id: row.get(0)?,
            title: row.get(1)?,
            completed: row.get::<_, i32>(2)? != 0,
            position: row.get(3)?,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
        })
    })?;

    Ok(todo)
}

pub fn list_todos(pool: &DbPool) -> Result<Vec<Todo>, AppError> {
    let conn = pool.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, title, completed, position, created_at, updated_at FROM todos ORDER BY position ASC",
    )?;
    let todos = stmt
        .query_map([], |row| {
            Ok(Todo {
                id: row.get(0)?,
                title: row.get(1)?,
                completed: row.get::<_, i32>(2)? != 0,
                position: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(todos)
}

pub fn get_todo(pool: &DbPool, id: i64) -> Result<Option<Todo>, AppError> {
    let conn = pool.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, title, completed, position, created_at, updated_at FROM todos WHERE id = ?1",
    )?;
    let mut rows = stmt.query([id])?;

    if let Some(row) = rows.next()? {
        Ok(Some(Todo {
            id: row.get(0)?,
            title: row.get(1)?,
            completed: row.get::<_, i32>(2)? != 0,
            position: row.get(3)?,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn update_todo(
    pool: &DbPool,
    id: i64,
    title: Option<&str>,
    completed: Option<bool>,
) -> Result<Option<Todo>, AppError> {
    let conn = pool.lock().unwrap();

    let mut updates = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(t) = title {
        updates.push("title = ?");
        params.push(Box::new(t.to_string()));
    }
    if let Some(c) = completed {
        updates.push("completed = ?");
        params.push(Box::new(c as i32));
    }

    if updates.is_empty() {
        return get_todo_internal(&conn, id);
    }

    updates.push("updated_at = strftime('%s', 'now')");
    params.push(Box::new(id));

    let query = format!("UPDATE todos SET {} WHERE id = ?", updates.join(", "));

    let params_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    conn.execute(&query, params_refs.as_slice())?;

    get_todo_internal(&conn, id)
}

pub fn reorder_todos(pool: &DbPool, ids: &[i64]) -> Result<(), AppError> {
    let conn = pool.lock().unwrap();

    for (position, id) in ids.iter().enumerate() {
        conn.execute(
            "UPDATE todos SET position = ?, updated_at = strftime('%s', 'now') WHERE id = ?",
            (position as i64, id),
        )?;
    }

    Ok(())
}

fn get_todo_internal(conn: &Connection, id: i64) -> Result<Option<Todo>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, title, completed, position, created_at, updated_at FROM todos WHERE id = ?1",
    )?;
    let mut rows = stmt.query([id])?;

    if let Some(row) = rows.next()? {
        Ok(Some(Todo {
            id: row.get(0)?,
            title: row.get(1)?,
            completed: row.get::<_, i32>(2)? != 0,
            position: row.get(3)?,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn delete_todo(pool: &DbPool, id: i64) -> Result<bool, AppError> {
    let conn = pool.lock().unwrap();
    let rows = conn.execute("DELETE FROM todos WHERE id = ?1", [id])?;
    Ok(rows > 0)
}

pub fn list_open_todos(pool: &DbPool) -> Result<Vec<Todo>, AppError> {
    let conn = pool.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, title, completed, position, created_at, updated_at FROM todos WHERE completed = 0 ORDER BY position ASC",
    )?;
    let todos = stmt
        .query_map([], |row| {
            Ok(Todo {
                id: row.get(0)?,
                title: row.get(1)?,
                completed: row.get::<_, i32>(2)? != 0,
                position: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(todos)
}
