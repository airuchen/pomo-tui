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
use std::io;

use crate::{timer::TimerState, utils};
use crate::{
    timer::{Preset, Timer},
    utils::create_large_ascii_numbers,
};

#[derive(Debug, Default)]
pub struct App {
    timer: Timer,
    exit: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            timer: Timer::new(),
            exit: false,
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
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            KeyCode::Char(' ') => self.toggle(),
            KeyCode::Char('s') => self.switch_mode(),
            KeyCode::Char('r') => self.reset(),
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
            Span::raw(self.timer.get_current_task().to_string()),
            Span::raw(" "),
            Span::raw(Local::now().format("%H:%M").to_string()),
        ]);

        text.push(state_info);

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
