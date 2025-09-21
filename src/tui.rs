use chrono::Local;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Clear, Paragraph, Widget},
};
use std::fmt;
use std::io;

use crate::{
    logging::{append_event, write_waybar_text},
    timer::{Preset, Timer, TimerMode},
    utils::{self, centered_area, create_large_ascii_numbers, render_keymap, KeyCommand},
};

const POPUP_WIDTH_PERCENT: u16 = 60;
const POPUP_HEIGHT_PERCENT: u16 = 70;
const TIMER_AREA_WIDTH_PERCENT: u16 = 100;
const TIMER_AREA_HEIGHT_PERCENT: u16 = 50;
const HISTORY_FILE_PATH: &str = "history.json";
const WAYBAR_STATE_FILE_PATH: &str = "pomo_waybar_state.json";

#[derive(Debug, Default, Copy, Clone, PartialEq)]
enum AppMode {
    #[default]
    Normal,
    Input,
}

impl AppMode {
    pub const fn toggle(self) -> Self {
        match self {
            Self::Normal => Self::Input,
            Self::Input => Self::Normal,
        }
    }
}

impl fmt::Display for AppMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppMode::Normal => f.write_str("Normal"),
            AppMode::Input => f.write_str("Input"),
        }
    }
}

#[derive(Debug, Default)]
pub struct TaskInput {
    /// Current value of the input box
    input: String,
    /// Position of cursor in the editor area.
    character_index: usize,
}

impl TaskInput {
    const fn new() -> Self {
        Self {
            input: String::new(),
            character_index: 0,
        }
    }

    fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.character_index.saturating_sub(1);
        self.character_index = self.clamp_cursor(cursor_moved_left);
    }

    fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.character_index.saturating_add(1);
        self.character_index = self.clamp_cursor(cursor_moved_right);
    }

    fn enter_char(&mut self, new_char: char) {
        let index = self.byte_index();
        self.input.insert(index, new_char);
        self.move_cursor_right();
    }

    /// Converts character index to byte index for Unicode-safe string manipulation.
    /// This is needed because Rust strings are UTF-8 encoded, where characters
    /// can be multiple bytes, but String::insert() requires byte indices.
    fn byte_index(&self) -> usize {
        self.input
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.character_index)
            .unwrap_or(self.input.len())
    }

    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.input.chars().count())
    }

    fn delete_char(&mut self) {
        let is_not_cursor_leftmost = self.character_index != 0;
        if is_not_cursor_leftmost {
            // Method "remove" is not used on the saved text for deleting the selected char.
            // Reason: Using remove on String works on bytes instead of the chars.
            // Using remove would require special care because of char boundaries.
            let current_index = self.character_index;
            let from_left_to_current_index = current_index - 1;

            // Getting all characters before the selected character.
            let before_char_to_delete = self.input.chars().take(from_left_to_current_index);
            // Getting all characters after the selected character.
            let after_char_to_delete = self.input.chars().skip(current_index);

            // Put all charaters together except the selected one.
            // TODO: what is chain? what is collect?
            self.input = before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left();
        }
    }

    fn reset_cursor(&mut self) {
        self.character_index = 0;
    }

    fn confirm_task(&mut self) -> String {
        let task = self.input.clone();
        self.input.clear();
        self.reset_cursor();
        task
    }

    fn break_input(&mut self) {
        self.input.clear();
        self.reset_cursor();
    }
}

#[derive(Debug, Default)]
pub struct App {
    timer: Timer,
    exit: bool,
    app_mode: AppMode,
    task_input: TaskInput,
    show_hint: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            timer: Timer::new(),
            exit: false,
            app_mode: AppMode::default(),
            task_input: TaskInput::new(),
            show_hint: false,
        }
    }
    /// runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            // 1) Update the timer
            self.timer.update();

            // 2) Check for input events
            if event::poll(std::time::Duration::from_millis(100))? {
                self.handle_events()?;
            }

            // 3) drain and persist events
            for event in self.timer.drain_events() {
                if let Err(e) = append_event(HISTORY_FILE_PATH, &event) {
                    log::error!("Failed to append event: {}", e);
                }
            }

            // 4) Render TUI
            terminal.draw(|frame| self.draw(frame, self.show_hint))?;

            // 5) save state
            if let Err(e) = write_waybar_text(
                WAYBAR_STATE_FILE_PATH,
                self.timer.get_mode(),
                self.timer.is_paused(),
                self.timer.is_idle(),
                self.timer.get_remaining(),
            ) {
                log::error!("Failed to write waybar state: {}", e);
            };
        }

        // Persist data before exit
        self.timer.persist_termination();
        for event in self.timer.drain_events() {
            if let Err(e) = append_event(HISTORY_FILE_PATH, &event) {
                log::error!("Failed to append event: {}", e);
            }
        }
        Ok(())
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            // it's important to check that the event is a key press event as
            // crossterm also emits key release and repeat events on Windows.
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => {}
        };
        Ok(())
    }

    fn draw(&self, frame: &mut Frame, show_hint: bool) {
        let area = frame.area();

        let layout = Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]);
        let [instructions, content] = area.layout(&layout);

        frame.render_widget(Line::from("?: Keymap, q: Quit").centered(), instructions);

        frame.render_widget(self, content);

        if show_hint {
            let popup_area = centered_area(area, POPUP_WIDTH_PERCENT, POPUP_HEIGHT_PERCENT);

            // clears out any background in the area before rendering the popup
            frame.render_widget(Clear, popup_area);

            let keymap_table = render_keymap();
            frame.render_widget(keymap_table, popup_area);
        }
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match self.app_mode {
            // Normal mode - use efficient KeyCommand lookup
            AppMode::Normal => {
                if let Some(command) = KeyCommand::from_keycode(key_event.code) {
                    self.execute_command(command);
                }
            }

            // Input mode for entering task name
            AppMode::Input => match key_event.code {
                KeyCode::Enter => {
                    self.timer.set_task_name(&self.task_input.confirm_task());
                    self.app_mode = AppMode::Normal;
                }
                KeyCode::Char(to_insert) => self.task_input.enter_char(to_insert),
                KeyCode::Backspace => self.task_input.delete_char(),
                KeyCode::Left => self.task_input.move_cursor_left(),
                KeyCode::Right => self.task_input.move_cursor_right(),
                KeyCode::Esc => {
                    self.app_mode = AppMode::Normal;
                    self.task_input.break_input();
                }
                _ => {}
            },
        }
    }

    /// Executes a KeyCommand with direct dispatch for optimal performance
    fn execute_command(&mut self, command: KeyCommand) {
        match command {
            KeyCommand::Quit => self.exit(),
            KeyCommand::ToggleKeymap => self.show_hint = !self.show_hint,
            KeyCommand::InputTask => self.app_mode = self.app_mode.toggle(),
            KeyCommand::Reset => self.timer.reset(),
            KeyCommand::Toggle => self.timer.toggle(),
            KeyCommand::SwitchMode => self.timer.switch_mode(),
            KeyCommand::SetLong => self.timer.set_preset(Preset::Long),
            KeyCommand::SetShort => self.timer.set_preset(Preset::Short),
            KeyCommand::SetTest => self.timer.set_preset(Preset::Test),
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // TODO: still don't get what <'static> do...
        let render_color = match (self.timer.get_mode(), self.timer.is_paused()) {
            (_, true) => Color::DarkGray,
            (TimerMode::Work, _) => Color::Yellow,
            (TimerMode::Break, _) => Color::Green,
        };
        let remaining_time = utils::fmt_duration(self.timer.get_remaining());
        let mut text: Vec<Line<'static>> =
            create_large_ascii_numbers(&remaining_time, render_color);
        let state_info = Line::from(vec![
            Span::raw(self.timer.get_mode().to_string()),
            Span::raw(" "),
            Span::raw(Local::now().format("%H:%M").to_string()),
        ]);
        text.push(state_info);

        let task_info = match self.app_mode {
            AppMode::Input => {
                let mut input_text = self.task_input.input.clone();
                input_text.insert(self.task_input.byte_index(), '|');

                Line::from(vec![
                    Span::styled("Enter the task name: ", Style::default().fg(Color::Green)),
                    Span::styled(input_text, Style::default().fg(Color::Green)),
                ])
            }
            AppMode::Normal => Line::from(vec![Span::styled(
                self.timer.get_task_name().to_string(),
                Style::default().fg(render_color).bold(),
            )]),
        };
        text.push(task_info);

        Paragraph::new(Text::from(text)).centered().render(
            centered_area(area, TIMER_AREA_WIDTH_PERCENT, TIMER_AREA_HEIGHT_PERCENT),
            buf,
        );
    }
}
