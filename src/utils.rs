use std::time::Duration;

use crossterm::event::KeyCode;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCommand {
    Quit,
    ToggleHint,
    InputTask,
    Reset,
    Toggle,
    SwitchMode,
    SetLong,
    SetShort,
    SetTest,
}

impl KeyCommand {
    /// Efficiently converts KeyCode to KeyCommand with zero runtime cost
    pub const fn from_keycode(key: KeyCode) -> Option<Self> {
        match key {
            KeyCode::Char('q') => Some(Self::Quit),
            KeyCode::Char('?') => Some(Self::ToggleHint),
            KeyCode::Char('i') => Some(Self::InputTask),
            KeyCode::Char('r') => Some(Self::Reset),
            KeyCode::Char(' ') => Some(Self::Toggle),
            KeyCode::Char('s') => Some(Self::SwitchMode),
            KeyCode::Char('+') => Some(Self::SetLong),
            KeyCode::Char('-') => Some(Self::SetShort),
            KeyCode::Char('`') => Some(Self::SetTest),
            _ => None,
        }
    }

    /// Returns the description for display in hint table
    pub const fn description(self) -> &'static str {
        match self {
            Self::InputTask => "Input current task name",
            Self::Reset => "Reset timer",
            Self::Toggle => "Start/Pause",
            Self::SwitchMode => "Switch Work/Break",
            Self::SetLong => "Set Long session (50/10)[m]",
            Self::SetShort => "Set Short session (25/5)[m]",
            Self::SetTest => "Set Test session (5/5)[s]",
            Self::ToggleHint => "Close Hint Page",
            Self::Quit => "Quit",
        }
    }

    /// Returns the key display string for hint table
    pub const fn key_display(self) -> &'static str {
        match self {
            Self::InputTask => "i",
            Self::Reset => "r",
            Self::Toggle => "Space",
            Self::SwitchMode => "s",
            Self::SetLong => "+",
            Self::SetShort => "-",
            Self::SetTest => "`",
            Self::ToggleHint => "?",
            Self::Quit => "q",
        }
    }

    /// All available commands for iteration
    pub const ALL: &'static [Self] = &[
        Self::InputTask,
        Self::Reset,
        Self::Toggle,
        Self::SwitchMode,
        Self::SetLong,
        Self::SetShort,
        Self::SetTest,
        Self::ToggleHint,
        Self::Quit,
    ];
}

/// Renders the hint table using the type-safe KeyCommand enum
pub fn render_hint() -> Table<'static> {
    let rows: Vec<Row> = KeyCommand::ALL
        .iter()
        .map(|cmd| Row::new([
            Cell::from(cmd.key_display()),
            Cell::from(cmd.description())
        ]))
        .collect();

    Table::new(rows, [Constraint::Length(10), Constraint::Fill(1)])
        .block(Block::bordered().title("Hint"))
        .flex(Flex::Center)
}
