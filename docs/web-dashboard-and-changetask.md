# Web Dashboard & ChangeTask — Design & Implementation

## Overview

Two features added in this session:

1. **Web dashboard** — a browser-based UI served from the existing Axum HTTP server at `http://127.0.0.1:1881/`
2. **Auto-split session on task change** — changing the task name mid-session terminates the current session and starts a new one with the remaining time, ensuring clean 1:1 mapping between sessions and tasks

## Web Dashboard

### Architecture

A single self-contained HTML file (`src/static/index.html`) with inline CSS and JS, embedded at compile time via `include_str!` and served by Axum at `GET /`. No build step, no dependencies, no framework.

### New REST API Endpoints

Added to `src/server/http.rs`:

| Route | Method | Description |
|-------|--------|-------------|
| `/` | GET | Serve the HTML dashboard |
| `/todos` | GET | All todos (flat list, frontend builds tree) |
| `/todos` | POST | Create todo `{"title": "...", "parent_id": null}` |
| `/todos/:id` | PUT | Update title `{"title": "..."}` |
| `/todos/:id` | DELETE | Delete (cascade) |
| `/todos/:id/toggle` | POST | Toggle done/not-done |
| `/todos/:id/stats` | GET | Session count + total work seconds for a todo |
| `/stats/daily?days=N` | GET | Daily aggregated stats (default 30 days) |

### New DB Queries

Added to `src/db/todos.rs`:

- `get_todo_stats(pool, todo_id)` — joins `todo_sessions` with `sessions` view, returns `TodoStats { session_count, total_work_secs }` for completed sessions
- `get_daily_stats(pool, days)` — groups completed work sessions by date, returns `Vec<DailyStats { date, session_count, total_work_secs }>`

### Dashboard Layout

```
┌─────────────────────────────────────────────────────┐
│  POMO-TUI                          [timer: 23:45]   │
│                                    [Work · Running]  │
│                                [start/pause/reset]   │
├──────────────────────┬──────────────────────────────┤
│  TODOS               │  RECENT SESSIONS             │
│  hierarchical tree   │  list with colored dots      │
│  click = set task    │  green=completed red=term    │
│  +child, del, toggle │  shows duration + time       │
├──────────────────────┴──────────────────────────────┤
│  THIS WEEK — daily bar chart (last 7 days)          │
└─────────────────────────────────────────────────────┘
```

### Polling Intervals

| Data | Interval |
|------|----------|
| Timer status | 1 second |
| Sessions + daily stats | 30 seconds |
| Todos | On user action only (add/edit/delete/toggle) |
| Todo highlight sync | On every timer poll if task name changed |

### Aesthetic

Terminal-inspired dark theme: `#0a0e14` background, JetBrains Mono font, amber (`#e8b84a`) for active timer and selected todo, green for break mode and completed sessions, red for terminated sessions.

## Auto-Split Session on Task Change

### Problem

Previously, changing the task name mid-session via `i` (Input mode) just updated the `task_name` string. The session kept its original UUID and the `Started` event still had the old task name. This made time tracking per-task inaccurate.

### Solution

New `Request::ChangeTask(String)` variant and `Timer::change_task()` method:

1. If the timer is **running** (not idle/paused):
   - Emit `Terminated` event for the current session
   - Snapshot remaining time
   - Update task name
   - Generate new UUID
   - Emit `Started` event with new task name and remaining time
2. If the timer is **idle or paused**: just update the name (same as `SetTask`)

The TUI's Input mode (`Enter` key) now checks `cached_status`: if running, sends `ChangeTask`; otherwise sends `SetTask`.

### Files Changed

| File | Change |
|------|--------|
| `src/protocol/messages.rs` | Added `Request::ChangeTask(String)` |
| `src/timer.rs` | Added `change_task()` method |
| `src/server/core.rs` | Handle `ChangeTask` in `process_request` |
| `src/client/tcp.rs` | Added `change_task_name()` convenience method |
| `src/tui.rs` | Input mode sends `ChangeTask` when timer is running |

### Delete Confirmation

The `d` key in Todo mode now uses a two-press pattern:
- First `d`: highlights the item in red with "press d to confirm"
- Second `d`: deletes
- Any other key: cancels

State tracked via `pending_delete: Option<Uuid>` on `ServerApp`.

## All Files Modified/Created

| File | Status |
|------|--------|
| `src/static/index.html` | **New** — dashboard |
| `src/server/http.rs` | New endpoints + dashboard route |
| `src/db/todos.rs` | `get_todo_stats`, `get_daily_stats`, `TodoStats`, `DailyStats` |
| `src/protocol/messages.rs` | `Request::ChangeTask` |
| `src/timer.rs` | `Timer::change_task()` |
| `src/server/core.rs` | Handle `ChangeTask` |
| `src/client/tcp.rs` | `change_task_name()` |
| `src/tui.rs` | `ChangeTask` in Input mode, `pending_delete` for delete confirmation |
