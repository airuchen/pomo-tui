use serde::Serialize;
use std::fs::rename;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::time::Duration;

use crate::timer::{LogEvent, TimerMode};
use crate::utils;

pub fn append_event(path: &str, event: &LogEvent) -> std::io::Result<()> {
    let f = OpenOptions::new().create(true).append(true).open(path)?;
    let mut w = BufWriter::new(f);

    // write JSON
    serde_json::to_writer(&mut w, event)
        .map_err(|e| std::io::Error::other(format!("Failed to serialize event: {}", e)))?;

    // new line for json
    w.write_all(b"\n")?;

    w.flush()?;
    Ok(())
}

#[derive(Serialize)]
struct WaybarState {
    text: String,
    class: String,
}

pub fn write_waybar_text(
    path: &str,
    mode: &TimerMode,
    paused: bool,
    idle: bool,
    remaining: Duration,
) -> std::io::Result<()> {
    let status = match (idle, paused) {
        (true, _) => "🈚",
        (false, true) => "⏸️",
        (false, false) => "▶️",
    };

    let remaining_text = utils::fmt_duration(remaining);

    let state = WaybarState {
        text: format!("🍅 {mode} : {remaining_text} {status}"),
        class: mode.to_string(),
    };

    // atomic write: *.tmp then rename
    let path_ref = std::path::Path::new(path);
    let tmp = path_ref.with_extension("tmp");

    {
        let mut f = File::create(&tmp)?;
        let json = serde_json::to_string(&state).map_err(|e| {
            std::io::Error::other(format!("Failed to serialize waybar state: {}", e))
        })?;
        f.write_all(json.as_bytes())?;
        f.flush()?;
    }
    rename(tmp, path_ref)?;
    Ok(())
}
