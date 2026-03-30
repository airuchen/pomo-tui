# Pomo-TUI API Reference

Servers start on:
- **TCP**: `127.0.0.1:1880` (TUI clients, newline-delimited JSON)
- **HTTP**: `127.0.0.1:1881` (REST API)

```bash
cargo run -- --server
```

---

## HTTP REST API

### Timer control

| Method | Route | Description |
|--------|-------|-------------|
| GET | `/ping` | Health check |
| GET | `/timer/status` | Get current timer state |
| POST | `/timer/start` | Start (or resume from idle/paused) |
| POST | `/timer/pause` | Pause running timer |
| POST | `/timer/resume` | Resume paused timer |
| POST | `/timer/reset` | Reset to idle |
| POST | `/timer/switch` | Toggle Work ↔ Break mode |
| PUT | `/timer/task` | Set task name (next session) |
| PUT | `/timer/preset` | Set preset |
| GET | `/timer/history` | Get session history |

### Todo management

| Method | Route | Description |
|--------|-------|-------------|
| GET | `/todos` | List all todos |
| POST | `/todos` | Create a todo |
| PUT | `/todos/{id}` | Update todo title |
| DELETE | `/todos/{id}` | Delete todo (and children) |
| POST | `/todos/{id}/toggle` | Toggle done state |
| POST | `/todos/{id}/priority` | Cycle priority (A → B → C → A) |
| GET | `/todos/{id}/stats` | Get session stats for a todo |

### Stats

| Method | Route | Description |
|--------|-------|-------------|
| GET | `/stats/daily` | Daily completed session counts |

### Dashboard

| Method | Route | Description |
|--------|-------|-------------|
| GET | `/` | Web dashboard (HTML) |

---

### Examples

```bash
# Status
curl http://127.0.0.1:1881/timer/status

# Start / pause / resume / reset
curl -X POST http://127.0.0.1:1881/timer/start
curl -X POST http://127.0.0.1:1881/timer/pause
curl -X POST http://127.0.0.1:1881/timer/resume
curl -X POST http://127.0.0.1:1881/timer/reset
curl -X POST http://127.0.0.1:1881/timer/switch

# Set task name
curl -X PUT http://127.0.0.1:1881/timer/task \
  -H "Content-Type: application/json" \
  -d '{"task": "Deep work"}'

# Set preset  ("Short" | "Long" | "Test")
curl -X PUT http://127.0.0.1:1881/timer/preset \
  -H "Content-Type: application/json" \
  -d '{"preset": "Long"}'

# Session history (last 20 by default, max 100)
curl http://127.0.0.1:1881/timer/history
curl http://127.0.0.1:1881/timer/history?limit=10

# List all todos
curl http://127.0.0.1:1881/todos

# Create a root todo
curl -X POST http://127.0.0.1:1881/todos \
  -H "Content-Type: application/json" \
  -d '{"title": "My task"}'

# Create a child todo
curl -X POST http://127.0.0.1:1881/todos \
  -H "Content-Type: application/json" \
  -d '{"title": "Subtask", "parent_id": "<uuid>"}'

# Update todo title
curl -X PUT http://127.0.0.1:1881/todos/<uuid> \
  -H "Content-Type: application/json" \
  -d '{"title": "Updated title"}'

# Toggle done
curl -X POST http://127.0.0.1:1881/todos/<uuid>/toggle

# Cycle priority
curl -X POST http://127.0.0.1:1881/todos/<uuid>/priority

# Delete todo
curl -X DELETE http://127.0.0.1:1881/todos/<uuid>

# Todo session stats
curl http://127.0.0.1:1881/todos/<uuid>/stats

# Daily stats (last 30 days by default)
curl http://127.0.0.1:1881/stats/daily
curl http://127.0.0.1:1881/stats/daily?days=7
```

### Status response

```json
{
  "Status": {
    "mode": "Work",
    "remaining": 1487,
    "preset": "Short",
    "is_paused": false,
    "is_idle": false,
    "is_running": true,
    "task": "Deep work"
  }
}
```

### Todo item response

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "parent_id": null,
  "title": "My task",
  "done": false,
  "priority": "B",
  "sort_order": 0,
  "created_at": "2026-03-30T09:00:00",
  "updated_at": "2026-03-30T09:00:00"
}
```

Priority values: `"A"` (high) → `"B"` (normal) → `"C"` (low). Sorted A first within each level.

### Todo stats response

```json
{
  "session_count": 3,
  "total_work_secs": 4500
}
```

Only counts completed work sessions linked to the todo.

### Daily stats response

```json
[
  { "date": "2026-03-30", "session_count": 4, "total_work_secs": 6000 },
  { "date": "2026-03-29", "session_count": 2, "total_work_secs": 3000 }
]
```

---

## TCP Protocol

Newline-delimited JSON — send one JSON object per line, receive one JSON response per line.

```bash
# Interactive
nc 127.0.0.1 1880

# One-shot
echo '{"Ping":null}' | nc 127.0.0.1 1880
echo '{"GetStatus":null}' | nc 127.0.0.1 1880
echo '{"Start":null}' | nc 127.0.0.1 1880
echo '{"Pause":null}' | nc 127.0.0.1 1880
echo '{"Resume":null}' | nc 127.0.0.1 1880
echo '{"Reset":null}' | nc 127.0.0.1 1880
echo '{"SwitchMode":null}' | nc 127.0.0.1 1880
echo '{"SetTask":"Deep work"}' | nc 127.0.0.1 1880
echo '{"ChangeTask":"New task"}' | nc 127.0.0.1 1880
echo '{"SetPreset":"Long"}' | nc 127.0.0.1 1880
```

### SetTask vs ChangeTask

| Command | Behaviour |
|---------|-----------|
| `SetTask` | Sets the task name; takes effect on the next session start |
| `ChangeTask` | Changes task mid-session: terminates the current session and starts a new one with the remaining time. If idle or paused, just updates the name |

### Preset values

| Value | Work | Break |
|-------|------|-------|
| `"Short"` | 25 min | 5 min |
| `"Long"` | 50 min | 10 min |
| `"Test"` | 5 sec | 5 sec |
