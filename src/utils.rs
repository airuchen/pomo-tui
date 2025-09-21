use std::time::Duration;

use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Cell, Row, Table},
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
const ASCII_ART_HEIGHT: usize = 5;
const DIGITS: [[&str; ASCII_ART_HEIGHT]; 10] = [
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
const COLON: [&str; ASCII_ART_HEIGHT] = ["     ", " ██  ", "     ", " ██  ", "     "];

fn glyph(ch: char) -> Option<&'static [&'static str; ASCII_ART_HEIGHT]> {
    // TODO: I don't get the return type here
    match ch {
        '0'..='9' => Some(&DIGITS[(ch as u8 - b'0') as usize]),
        ':' => Some(&COLON),
        _ => None,
    }
}

pub fn create_large_ascii_numbers(time_text: &str, color: Color) -> Vec<Line<'static>> {
    let style = Style::default().fg(color).add_modifier(Modifier::BOLD);
    let estimated_width = time_text.len() * 8;
    let mut lines: [String; ASCII_ART_HEIGHT] = [
        String::with_capacity(estimated_width),
        String::with_capacity(estimated_width),
        String::with_capacity(estimated_width),
        String::with_capacity(estimated_width),
        String::with_capacity(estimated_width),
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

/// Create a centered rect using up certain percentage of the available rect
pub fn centered_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [area] = area.layout(&vertical);
    let [area] = area.layout(&horizontal);
    area
}

const KEYMAP: &[(&str, &str)] = &[
    ("q", "Quit"),
    ("?", "Toggle Keymap"),
    ("i", "Input current task name"),
    ("r", "Reset timer"),
    ("Space", "Start/Pause"),
    ("s", "Switch Work/Break"),
    ("+", "Set Long session (50/10)[m]"),
    ("-", "Set Short session (25/5)[m]"),
    ("`", "Set Test session (5/5)[s]"),
];

// TODO: how can I bound the keymap with their function?
pub fn render_keymap() -> Table<'static> {
    let rows: Vec<Row> = KEYMAP
        .iter()
        .map(|(k, d)| Row::new([Cell::from(*k), Cell::from(*d)]))
        .collect();

    Table::new(rows, [Constraint::Length(10), Constraint::Fill(1)])
        .block(Block::bordered().title("Keymap"))
        .flex(Flex::Center)
}
