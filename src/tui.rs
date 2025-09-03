use chrono::Local;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    symbols::border,
    text::{Line, Span, Text},
    widgets::{Block, Paragraph, Widget},
};
use std::fmt;
use std::io;

use crate::{timer::TimerState, utils};
use crate::{
    timer::{Preset, Timer},
    utils::create_large_ascii_numbers,
};

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
    /// Task name
    pub task: String,
}

impl TaskInput {
    const fn new() -> Self {
        Self {
            input: String::new(),
            character_index: 0,
            task: String::new(),
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

    /// Returns the type index based on the character position.
    ///
    /// Since each character in a string can container multiple bytes, it's a necessary to
    /// calculate the type index based on the index of the character.
    fn byte_index(&self) -> usize {
        // TODO: What are we doing here?
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

    const fn reset_cursor(&mut self) {
        self.character_index = 0;
    }

    fn confirm_task(&mut self) {
        self.task = self.input.clone();
        self.input.clear();
        self.reset_cursor();
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
}

impl App {
    pub fn new() -> Self {
        Self {
            timer: Timer::new(),
            exit: false,
            app_mode: AppMode::default(),
            task_input: TaskInput::new(),
        }
    }
    /// runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.timer.update();

            // Check for events with a timeout to allow timer update
            if event::poll(std::time::Duration::from_millis(100))? {
                self.handle_events()?;
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

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match self.app_mode {
            AppMode::Normal => match key_event.code {
                KeyCode::Char('q') => self.exit(),
                KeyCode::Char(' ') => self.toggle(),
                KeyCode::Char('s') => self.switch_mode(),
                KeyCode::Char('r') => self.reset(),
                KeyCode::Char('t') => {
                    self.app_mode = self.app_mode.toggle();
                }
                KeyCode::Char('+') => {
                    self.timer.set_preset(Preset::Long);
                }
                KeyCode::Char('-') => {
                    self.timer.set_preset(Preset::Short);
                }
                KeyCode::Char('t') => {
                    self.timer.set_preset(Preset::Test);
                }

                _ => {}
            },
            AppMode::Input if key_event.kind == KeyEventKind::Press => match key_event.code {
                KeyCode::Enter => {
                    self.task_input.confirm_task();
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
            AppMode::Input => {}
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }
}

impl App {
    // TODO: this seems just a wrapper to call the timer's interface, do I want this?
    fn toggle(&mut self) {
        self.timer.toggle();
    }

    fn switch_mode(&mut self) {
        self.timer.switch_mode();
    }

    fn reset(&mut self) {
        self.timer.reset();
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = Line::from("Pomo TUI".bold());
        let instructions = Line::from(vec![" Decrement ".into(), "<Left>".blue().bold()]);
        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .border_set(border::THICK);

        // TODO: still don't get what <'static> do...
        let render_color = match (self.timer.get_state(), self.timer.is_paused()) {
            (_, true) => Color::DarkGray,
            (TimerState::Work, _) => Color::Yellow,
            (TimerState::Break, _) => Color::Green,
        };
        let remaining_time = utils::fmt_duration(self.timer.get_remaining());
        let mut text: Vec<Line<'static>> =
            create_large_ascii_numbers(&remaining_time, render_color);
        let state_info = Line::from(vec![
            Span::raw(self.timer.get_state().to_string()),
            Span::raw(" "),
            Span::raw(Local::now().format("%H:%M").to_string()),
        ]);
        text.push(state_info);

        // For Normal mode, we print the task name
        // For Input mode, we print "Enter the task name: <user_input>"
        let task_info = match self.app_mode {
            AppMode::Input => Line::from(vec![
                Span::styled("Enter the task name: ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    self.task_input.input.to_string(),
                    Style::default().fg(Color::Yellow),
                ),
            ]),
            AppMode::Normal => Line::from(vec![Span::raw(self.task_input.task.to_string())]),
        };
        text.push(task_info);

        let chunks = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                Constraint::Percentage(30),
                Constraint::Percentage(40),
                Constraint::Percentage(30),
            ])
            .split(area);

        Paragraph::new(Text::from(text))
            .centered()
            // .block(block)
            .render(chunks[1], buf);
    }
}
