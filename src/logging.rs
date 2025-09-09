use std::fs::OpenOptions;
use std::io::{BufWriter, Write};

use crate::timer::LogEvent;

pub fn append_event(path: &str, event: &LogEvent) -> std::io::Result<()> {
    let mut f = OpenOptions::new().create(true).append(true).open(path)?;
    let mut w = BufWriter::new(f);

    // write JSON
    serde_json::to_writer(&mut w, event).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;


    // new line for json
    w.write_all(b"\n")?;

    w.flush()?;
    Ok(())
}
