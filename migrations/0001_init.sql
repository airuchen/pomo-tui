CREATE TABLE IF NOT EXISTS events (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id     TEXT    NOT NULL,
    event_type     TEXT    NOT NULL,
    timer_type     TEXT,
    task           TEXT    NOT NULL,
    at             TEXT    NOT NULL,
    remaining_secs INTEGER,
    work_secs      INTEGER
);

CREATE INDEX IF NOT EXISTS idx_events_session_id ON events(session_id);

CREATE VIEW IF NOT EXISTS sessions AS
SELECT
    e.session_id,
    (SELECT timer_type FROM events
     WHERE session_id = e.session_id AND event_type = 'Started'
     LIMIT 1)                          AS timer_type,
    (SELECT task FROM events
     WHERE session_id = e.session_id AND event_type = 'Started'
     LIMIT 1)                          AS task,
    MIN(e.at)                          AS started_at,
    MAX(e.at)                          AS ended_at,
    MAX(e.work_secs)                   AS work_secs,
    (SELECT event_type FROM events
     WHERE session_id = e.session_id
     ORDER BY at DESC, id DESC LIMIT 1) AS final_event
FROM events e
GROUP BY e.session_id;
