CREATE TABLE IF NOT EXISTS todos (
    id         TEXT    PRIMARY KEY,
    parent_id  TEXT,
    title      TEXT    NOT NULL,
    done       INTEGER NOT NULL DEFAULT 0,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT    NOT NULL,
    updated_at TEXT    NOT NULL,
    FOREIGN KEY (parent_id) REFERENCES todos(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_todos_parent_id ON todos(parent_id);

CREATE TABLE IF NOT EXISTS todo_sessions (
    todo_id    TEXT NOT NULL,
    session_id TEXT NOT NULL,
    linked_at  TEXT NOT NULL,
    PRIMARY KEY (todo_id, session_id),
    FOREIGN KEY (todo_id) REFERENCES todos(id) ON DELETE CASCADE
);
