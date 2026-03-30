# Todo List — Design & Implementation

## Overview

An org-mode style hierarchical todo list integrated into the TUI, persisted in SQLite, with automatic pomodoro session linking.

## Design Decision: Direct DB Access from TUI

Todos are a **client-side organizational concern** — the server/timer doesn't need todo data for its tick loop or state mutations. Rather than routing CRUD through the TCP protocol (which would add ~8 new Request/Response variants), the TUI accesses SQLite directly via its own `SqlitePool` handle. SQLite WAL mode ensures safe concurrent access between the server (writing events) and TUI (writing todos).

## Database Schema

Migration: `migrations/0002_todos.sql`

### `todos` table

| Column | Type | Description |
|--------|------|-------------|
| `id` | TEXT PK | UUID |
| `parent_id` | TEXT (nullable) | UUID of parent, NULL for root items |
| `title` | TEXT | Todo title |
| `done` | INTEGER | 0 = open, 1 = done |
| `sort_order` | INTEGER | Ordering among siblings (auto-incremented) |
| `created_at` | TEXT | RFC 3339 timestamp |
| `updated_at` | TEXT | RFC 3339 timestamp |

Foreign key on `parent_id` with `ON DELETE CASCADE` — deleting a parent removes all descendants.

### `todo_sessions` table (many-to-many)

| Column | Type | Description |
|--------|------|-------------|
| `todo_id` | TEXT | FK → todos.id |
| `session_id` | TEXT | Session UUID from events table |
| `linked_at` | TEXT | RFC 3339 timestamp |

Composite primary key `(todo_id, session_id)`. `INSERT OR IGNORE` prevents duplicates.

## Architecture

### Module layout

```
src/
├── db/
│   ├── mod.rs          # Pool init, migrations
│   ├── events.rs       # Session event persistence
│   └── todos.rs        # Todo CRUD + session linking
├── todo.rs             # TodoTree in-memory model
├── tui.rs              # Todo/TodoInput modes, rendering, key handling
├── utils.rs            # KeyCommand::OpenTodo
└── main.rs             # Wires SqlitePool into ServerApp
```

### Data flow

```
SQLite (source of truth)
    ↕  get_all_todos / insert / update / delete
TodoTree (in-memory, rebuilt on each mutation)
    ↓  visible_items() — DFS respecting expanded flags
TUI rendering (popup overlay)
```

1. On entering Todo mode (`t` key), `reload_todos()` fetches all rows and rebuilds the tree.
2. Each mutation (add/edit/delete/toggle) writes to DB immediately, then rebuilds the tree.
3. Expanded/collapsed state is UI-only (not persisted) but preserved across reloads within a session.

### In-memory tree — `src/todo.rs`

`TodoTree` holds a `HashMap<Uuid, TodoItem>` and a `Vec<Uuid>` of root IDs. Built from flat `TodoRow` list via two passes:
1. Create all items
2. Wire parent→child relationships, sort by `sort_order`

`visible_items()` does a depth-first traversal, skipping collapsed subtrees, returning `Vec<(depth, &TodoItem)>` for the renderer.

### TUI modes — `src/tui.rs`

`AppMode` now has four variants:

| Mode | Purpose |
|------|---------|
| `Normal` | Timer view, key commands |
| `Input` | Freeform task name entry |
| `Todo` | Todo list navigation and actions |
| `TodoInput` | Editing/adding a todo item title |

`TodoInputAction` enum tracks whether the input is for `AddSibling`, `AddChild`, or `EditTitle`.

### Session auto-linking

1. User presses `Enter` on a todo → title sent as `SetTask`, `active_todo_id` stored
2. TUI main loop detects idle transition (session ended) via `cached_status`
3. If `active_todo_id` is set, queries latest `session_id` from events table and calls `link_todo_session`
4. `active_todo_id` clears when user manually sets task name via `i`

### Rendering

Todo popup uses the same overlay pattern as the hint popup (`centered_area` + `Clear`):

```
+--- Todos [a:add A:child x:done d:del e:edit Enter:select Esc:close] ---+
|   [x] Write documentation                                               |
|   v [ ] API docs                                                         |
|       [ ] REST endpoints                                                 |
|       [ ] TCP protocol                                                   |
|   [ ] User guide                                                    [2p] |
| > [ ] Fix bug in timer reset                                        [1p] |
|   [ ] Refactor protocol module                                           |
+--------------------------------------------------------------------------+
```

- 2-space indent per depth level
- `>` = collapsed with children, `v` = expanded with children
- `[x]` / `[ ]` = done / open
- `[Np]` suffix = N linked pomodoro sessions
- Cursor line highlighted with `bg(DarkGray)`
- Active todo (linked to timer) shown in yellow bold

## DB layer — `src/db/todos.rs`

| Function | Description |
|----------|-------------|
| `insert_todo(pool, parent_id, title)` | Creates todo with auto-incremented sort_order, returns UUID |
| `update_todo_title(pool, id, title)` | Updates title and `updated_at` |
| `toggle_todo_done(pool, id)` | Flips `done` between 0 and 1 |
| `delete_todo(pool, id)` | Deletes todo (CASCADE removes children + session links) |
| `get_all_todos(pool)` | Returns all todos ordered by `sort_order` |
| `link_todo_session(pool, todo_id, session_id)` | Links todo to session (`INSERT OR IGNORE`) |
| `get_session_count_for_todo(pool, todo_id)` | Count of linked sessions |
| `get_latest_session_id(pool)` | Most recent session_id from events table |

## Tests

- **DB layer** (`src/db/todos.rs`): insert, child insert, sort order, toggle done, update title, delete cascade, session linking
- **Tree model** (`src/todo.rs`): empty tree, flat list, nested collapsed by default, expand/collapse, sort order
