use std::time::Duration;

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

pub fn fmt_duration(d: Duration) -> String {
    let secs = d.as_secs();
    let minutes = secs / 60;
    let seconds = secs % 60;
    format!("{:02}:{:02}", minutes, seconds)
}

// For TUI
// TODO: can I give a namespace?
const MIN_PAD: &str = "  "; // TODO: Why not String here?
const DIGITS: [[&str; 5]; 10] = [
    [" ████ ", "██  ██", "██  ██", "██  ██", " ████ "], // 0
    ["  ██  ", " ████ ", "  ██  ", "  ██  ", " █████"], // 1
    [" ████ ", "██  ██", "   ██ ", "  ██  ", "██████"], // 2
    ["█████ ", "    ██", "  ███ ", "    ██", "█████ "], // 3
    ["██  ██", "██  ██", "██████", "   ██ ", "   ██ "], // 4
    ["██████", "██    ", "█████ ", "    ██", "█████ "], // 5
    [" ████ ", "██    ", "█████ ", "██  ██", " ████ "], // 6
    ["██████", "   ██ ", "  ██  ", " ██   ", " ██   "], // 7
    [" ████ ", "██  ██", " ████ ", "██  ██", " ████ "], // 8
    [" ████ ", "██  ██", " █████", "    ██", " ████ "], // 9
];
const COLON: [&str; 5] = ["     ", " ██  ", "     ", " ██  ", "     "];

fn glyph(ch: char) -> Option<&'static [&'static str; 5]> {
    // TODO: I don't get the return type here
    match ch {
        '0'..='9' => Some(&DIGITS[(ch as u8 - b'0') as usize]),
        ':' => Some(&COLON),
        _ => None,
    }
}

pub fn create_large_ascii_numbers(time_text: &str, color: Color) -> Vec<Line<'static>> {
    let style = Style::default().fg(color).add_modifier(Modifier::BOLD);
    let mut lines = [
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
    ];

    for ch in time_text.chars() {
        if let Some(rows) = glyph(ch) {
            for (i, row) in rows.iter().enumerate() {
                lines[i].push_str(row);
                lines[i].push_str(MIN_PAD);
            }
        }
    }

    lines
        .into_iter()
        .map(|s| Line::from(Span::styled(s, style)))
        .collect()
}
