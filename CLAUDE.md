# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
# Build
cargo build

# Run (auto-starts embedded server + TUI)
cargo run

# Run server-only mode
cargo run -- --server

# Run with custom addresses
cargo run -- --tcp-addr 127.0.0.1:1880 --http-addr 127.0.0.1:1881

# Run all tests
cargo test

# Run a single test by name
cargo test <test_name>

# Run tests in a specific module
cargo test --lib timer

# Check (faster than build, no binary)
cargo check

# Lint
cargo clippy

# Pre-commit runs cargo fmt automatically
```

## Architecture

`pomo-tui` uses a **client–server architecture** where the timer state lives in `PomoServer` and clients communicate over TCP or HTTP.

**Startup behavior** (`src/main.rs`):
- No `--server` flag: checks if a server is already listening on `tcp_addr`. If yes, connects as TUI client. If no, spawns an embedded server + TUI in the same process.
- `--server` flag: runs only the TCP + HTTP servers (headless), useful for persistent background timer.

**Core modules:**

| Module | File(s) | Role |
|---|---|---|
| `timer` | `src/timer.rs` | Pure timer logic — `Timer` struct, `TimerMode` (Work/Break), `Preset` (Short/Long/Test), `LogEvent` queue |
| `server::core` | `src/server/core.rs` | `PomoServer` wraps `Timer` in `Arc<Mutex<>>`, runs a 1-second background tick loop, dispatches `Request` → `Response` |
| `server::tcp` | `src/server/tcp.rs` | Newline-delimited JSON over TCP; one task per connected client |
| `server::http` | `src/server/http.rs` | Axum REST API on `http_addr`; routes map to `PomoServer::process_request` |
| `client::tcp` | `src/client/tcp.rs` | `PomoClient` — async TCP client that sends `Request` / receives `Response` as newline-delimited JSON |
| `protocol` | `src/protocol/messages.rs` | Shared `Request` / `Response` enums (serde JSON) used by both TCP client and server |
| `tui` | `src/tui.rs` | `ServerApp` — ratatui event loop; polls server status every 100 ms, renders ASCII clock, handles key input |
| `utils` | `src/utils.rs` | ASCII number rendering helpers, `KeyCommand` enum, layout utilities |
| `todo` | `src/todo.rs` | `TodoItem`, `TodoTree` — in-memory tree structure with depth-first traversal, expand/collapse, priority sorting |
| `db` | `src/db/mod.rs`, `src/db/events.rs`, `src/db/todos.rs` | SQLite persistence — pool init, migrations (`sqlx::migrate!`), session/event logging, todo CRUD and session linking |
| `logging` | `src/logging.rs` | File-based logging setup |

**Data flow:**
1. `Timer::update()` is called every second by `PomoServer`'s background task — handles completion, mode switching, and desktop notifications.
2. `PomoServer::process_request()` is the single entry point for all state mutations.
3. `ServerApp` (TUI) polls `GetStatus` every loop iteration (~100 ms), caches the result, and dispatches key presses as `Request`s via `PomoClient`.

**Protocol:**
- TCP: newline-delimited JSON (`serde_json`), with a 5-second read timeout per request.
- HTTP: REST via Axum — timer routes (`GET /timer/status`, `POST /timer/start|pause|resume|reset|switch`, `PUT /timer/task|preset`, `GET /timer/history`), todo routes (`GET|POST /todos`, `PUT|DELETE /todos/{id}`, `POST /todos/{id}/toggle|priority`, `GET /todos/{id}/stats`), stats (`GET /stats/daily`), dashboard (`GET /`).

**Timer presets:**
- `Short`: 25 min work / 5 min break
- `Long`: 50 min work / 10 min break
- `Test`: 5 sec / 5 sec (for development)

**TUI modes:**
- `Normal`: key commands (space=toggle, r=reset, i=task input, s=switch mode, +/-/`=preset, ?=hint, q=quit)
- `Input`: freeform task name entry, confirmed with Enter or cancelled with Esc

**Persistence:**
Timer events are stored in SQLite at `~/.local/share/pomo-tui/pomo.db` (WAL mode, single connection). Schema migrations live in `migrations/` and run automatically via `sqlx::migrate!` on startup.
