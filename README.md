# Donezo

A self-hosted todo app built with Rust (Axum) + SQLite + Vanilla JS.

## Prerequisites

- Rust toolchain
- Tailwind CSS CLI

## Quick Start

```bash
cargo build
DONEZO_PASSWORD=yourpassword DONEZO_PORT=3000 cargo run
```

Open `http://localhost:3000` and log in with your password.

## Configuration

All configuration is via environment variables:

| Variable | Required | Description |
|---|---|---|
| `DONEZO_PASSWORD` | Yes | Login password |
| `DONEZO_PORT` | Yes | Port to listen on |
| `DONEZO_BASE_PATH` | No | Base path prefix (e.g. `/todo`) for reverse proxy setups |

## API

Authenticate API requests with a Bearer token (create one in the web UI under
token management).

```bash
# List todos
curl -H "Authorization: Bearer <token>" http://localhost:3000/api/todos

# Create a todo
curl -X POST -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{"title": "Buy milk"}' \
  http://localhost:3000/api/todos

# Plain-text export
curl -H "Authorization: Bearer <token>" http://localhost:3000/api/todos/plain
```

## License

MIT
