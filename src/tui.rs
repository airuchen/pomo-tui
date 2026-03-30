// Copyright (c) 2025 Yu-Wen Chen
// Licensed under the MIT License (see LICENSE file)

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

use sqlx::SqlitePool;
use uuid::Uuid;

use crate::{
    client::PomoClient,
    db,
    timer::TimerStatus,
    todo::TodoTree,
    utils::{self, KeyCommand, centered_area, create_large_ascii_numbers, render_hint},
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
    Todo,
    TodoInput,
}

impl fmt::Display for AppMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppMode::Normal => f.write_str("Normal"),
            AppMode::Input => f.write_str("Input"),
            AppMode::Todo => f.write_str("Todo"),
            AppMode::TodoInput => f.write_str("TodoInput"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum TodoInputAction {
    AddSibling,
    AddChild,
    EditTitle,
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

#[derive(Debug)]
pub struct ServerApp {
    pomo_client: PomoClient,
    cached_status: Option<TimerStatus>,
    exit: bool,
    app_mode: AppMode,
    task_input: TaskInput,
    show_hint: bool,
    // Todo state
    pool: Option<SqlitePool>,
    todo_tree: TodoTree,
    todo_cursor: usize,
    todo_input: TaskInput,
    todo_input_action: Option<TodoInputAction>,
    active_todo_id: Option<Uuid>,
    pending_delete: Option<Uuid>,
    prev_session_id: Option<String>,
}

impl ServerApp {
    pub fn new(pomo_client: PomoClient, pool: Option<SqlitePool>) -> Self {
        Self {
            pomo_client,
            cached_status: None,
            exit: false,
            app_mode: AppMode::default(),
            task_input: TaskInput::new(),
            show_hint: false,
            pool,
            todo_tree: TodoTree::default(),
            todo_cursor: 0,
            todo_input: TaskInput::new(),
            todo_input_action: None,
            active_todo_id: None,
            pending_delete: None,
            prev_session_id: None,
        }
    }
    /// runs the application's main loop until the user quits
    pub async fn run(&mut self, terminal: &mut DefaultTerminal) -> anyhow::Result<()> {
        while !self.exit {
            // Update cached status
            let prev_idle = self.cached_status.as_ref().map(|s| s.is_idle);
            match self
                .pomo_client
                .send_request(crate::protocol::Request::GetStatus)
                .await
            {
                Ok(crate::protocol::messages::Response::Status(status)) => {
                    // Detect session completion: was not idle, now is idle
                    if prev_idle == Some(false) && status.is_idle {
                        self.on_session_ended().await;
                    }
                    self.cached_status = Some(status);
                }
                Ok(_) => {}
                Err(_) => {}
            }

            // 1) Check for input events
            if event::poll(std::time::Duration::from_millis(100))? {
                self.handle_events().await?;
            }

            terminal.draw(|frame| self.draw(frame))?;
        }

        Ok(())
    }

    async fn on_session_ended(&mut self) {
        if let (Some(todo_id), Some(pool)) = (self.active_todo_id, &self.pool) {
            if let Ok(Some(session_id)) = db::todos::get_latest_session_id(pool).await {
                if self.prev_session_id.as_deref() != Some(&session_id) {
                    let _ =
                        db::todos::link_todo_session(pool, &todo_id.to_string(), &session_id).await;
                    self.prev_session_id = Some(session_id);
                }
            }
        }
    }

    async fn handle_events(&mut self) -> anyhow::Result<()> {
        match event::read()? {
            // it's important to check that the event is a key press event as
            // crossterm also emits key release and repeat events on Windows.
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event).await?;
            }
            _ => {}
        };
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        let area = frame.area();

        let layout = Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]);
        let [instructions, content] = area.layout(&layout);

        frame.render_widget(
            Line::from("?: Hint, t: Todos, q: Quit").centered(),
            instructions,
        );

        frame.render_widget(self, content);

        if self.show_hint {
            let popup_area = centered_area(area, POPUP_WIDTH_PERCENT, POPUP_HEIGHT_PERCENT);
            frame.render_widget(Clear, popup_area);
            let hint_table = render_hint();
            frame.render_widget(hint_table, popup_area);
        }

        if self.app_mode == AppMode::Todo || self.app_mode == AppMode::TodoInput {
            let popup_area = centered_area(area, 80, 80);
            frame.render_widget(Clear, popup_area);
            self.render_todo_popup(frame, popup_area);
        }
    }

    fn render_todo_popup(&self, frame: &mut Frame, area: Rect) {
        use ratatui::widgets::{Block, Borders};

        let title = if self.app_mode == AppMode::TodoInput {
            " Todos (editing) "
        } else {
            " Todos [a:add A:child x:done d:del e:edit p:priority Enter:select Esc:close] "
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let visible = self.todo_tree.visible_items();

        if visible.is_empty() && self.app_mode != AppMode::TodoInput {
            let empty_msg = Paragraph::new(Text::from(Line::from(Span::styled(
                "No todos yet. Press 'a' to add one.",
                Style::default().fg(Color::DarkGray),
            ))))
            .centered();
            frame.render_widget(empty_msg, inner);
            return;
        }

        let mut lines: Vec<Line<'static>> = Vec::new();

        for (i, (depth, item)) in visible.iter().enumerate() {
            let indent = "  ".repeat(*depth);
            let expand_marker = if !item.children.is_empty() {
                if item.expanded { "v " } else { "> " }
            } else {
                "  "
            };
            let done_marker = if item.done { "[x] " } else { "[ ] " };
            let priority_tag = match item.priority.as_str() {
                "A" => "[#A] ",
                "C" => "[#C] ",
                _ => "", // B is default, hidden
            };
            let session_suffix = if item.session_count > 0 {
                format!(" [{}p]", item.session_count)
            } else {
                String::new()
            };

            let is_active = self.active_todo_id == Some(item.id);
            let is_pending_delete = self.pending_delete == Some(item.id);
            let text = if is_pending_delete {
                format!(
                    "{}{}{}{}{}{}  <- press d to confirm",
                    indent, expand_marker, done_marker, priority_tag, item.title, session_suffix
                )
            } else {
                format!(
                    "{}{}{}{}{}{}",
                    indent, expand_marker, done_marker, priority_tag, item.title, session_suffix
                )
            };

            let style = if is_pending_delete {
                Style::default().bg(Color::Red).fg(Color::White).bold()
            } else if i == self.todo_cursor {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            } else if item.done {
                Style::default().fg(Color::DarkGray)
            } else if is_active {
                Style::default().fg(Color::Yellow).bold()
            } else if item.priority == "A" {
                Style::default().fg(Color::LightRed)
            } else if item.priority == "C" {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default().fg(Color::White)
            };

            lines.push(Line::from(Span::styled(text, style)));
        }

        // Show input line if in TodoInput mode
        if self.app_mode == AppMode::TodoInput {
            let mut input_text = self.todo_input.input.clone();
            input_text.insert(self.todo_input.byte_index(), '|');
            let prefix = match self.todo_input_action {
                Some(TodoInputAction::AddSibling) => "New todo: ",
                Some(TodoInputAction::AddChild) => "New child: ",
                Some(TodoInputAction::EditTitle) => "Edit: ",
                None => "Input: ",
            };
            lines.push(Line::from(vec![
                Span::styled(prefix, Style::default().fg(Color::Green)),
                Span::styled(input_text, Style::default().fg(Color::Green)),
            ]));
        }

        let paragraph = Paragraph::new(Text::from(lines));
        frame.render_widget(paragraph, inner);
    }

    async fn handle_key_event(&mut self, key_event: KeyEvent) -> anyhow::Result<()> {
        match self.app_mode {
            // Normal mode - use efficient KeyCommand lookup
            AppMode::Normal => {
                // Intuitively close the hint
                if self.show_hint && key_event.code == KeyCode::Esc {
                    self.show_hint = false;
                    return Ok(());
                }
                if let Some(command) = KeyCommand::from_keycode(key_event.code) {
                    self.execute_command(command).await?
                }
                Ok(())
            }

            // Input mode for entering task name
            AppMode::Input => match key_event.code {
                KeyCode::Enter => {
                    let task = self.task_input.confirm_task();
                    // If timer is running, split the session; otherwise just set the name
                    let is_running = self
                        .cached_status
                        .as_ref()
                        .map(|s| s.is_running && !s.is_paused)
                        .unwrap_or(false);
                    if is_running {
                        let _ = self.pomo_client.change_task_name(task).await;
                    } else {
                        let _ = self.pomo_client.set_task_name(task).await;
                    }
                    self.active_todo_id = None; // manual task name clears todo link
                    self.app_mode = AppMode::Normal;
                    Ok(())
                }
                KeyCode::Char(to_insert) => {
                    self.task_input.enter_char(to_insert);
                    Ok(())
                }
                KeyCode::Backspace => {
                    self.task_input.delete_char();
                    Ok(())
                }
                KeyCode::Left => {
                    self.task_input.move_cursor_left();
                    Ok(())
                }
                KeyCode::Right => {
                    self.task_input.move_cursor_right();
                    Ok(())
                }
                KeyCode::Esc => {
                    self.app_mode = AppMode::Normal;
                    self.task_input.break_input();
                    Ok(())
                }
                _ => Ok(()),
            },

            // Todo list mode
            AppMode::Todo => {
                self.handle_todo_key(key_event).await?;
                Ok(())
            }

            // Todo input mode (adding/editing todo items)
            AppMode::TodoInput => match key_event.code {
                KeyCode::Enter => {
                    let text = self.todo_input.confirm_task();
                    if !text.is_empty() {
                        self.commit_todo_input(&text).await?;
                    }
                    self.app_mode = AppMode::Todo;
                    self.todo_input_action = None;
                    Ok(())
                }
                KeyCode::Char(to_insert) => {
                    self.todo_input.enter_char(to_insert);
                    Ok(())
                }
                KeyCode::Backspace => {
                    self.todo_input.delete_char();
                    Ok(())
                }
                KeyCode::Left => {
                    self.todo_input.move_cursor_left();
                    Ok(())
                }
                KeyCode::Right => {
                    self.todo_input.move_cursor_right();
                    Ok(())
                }
                KeyCode::Esc => {
                    self.todo_input.break_input();
                    self.app_mode = AppMode::Todo;
                    self.todo_input_action = None;
                    Ok(())
                }
                _ => Ok(()),
            },
        }
    }

    async fn handle_todo_key(&mut self, key_event: KeyEvent) -> anyhow::Result<()> {
        let visible_count = self.todo_tree.visible_items().len();

        // Second `d` confirms delete; any other key cancels
        if key_event.code == KeyCode::Char('d') {
            if let Some(pending_id) = self.pending_delete.take() {
                // Confirm: cursor still on the same item
                if self.todo_tree.id_at_cursor(self.todo_cursor) == Some(pending_id) {
                    if let Some(pool) = &self.pool {
                        db::todos::delete_todo(pool, &pending_id.to_string()).await?;
                        if self.active_todo_id == Some(pending_id) {
                            self.active_todo_id = None;
                        }
                        self.reload_todos().await?;
                        let visible_count = self.todo_tree.visible_items().len();
                        if self.todo_cursor >= visible_count && visible_count > 0 {
                            self.todo_cursor = visible_count - 1;
                        }
                    }
                }
                return Ok(());
            }
            // First `d`: mark for deletion
            if let Some(id) = self.todo_tree.id_at_cursor(self.todo_cursor) {
                self.pending_delete = Some(id);
            }
            return Ok(());
        }

        // Any other key clears pending delete
        self.pending_delete = None;

        match key_event.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if visible_count > 0 && self.todo_cursor < visible_count - 1 {
                    self.todo_cursor += 1;
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.todo_cursor > 0 {
                    self.todo_cursor -= 1;
                }
            }
            KeyCode::Char('l') | KeyCode::Right => {
                if let Some(id) = self.todo_tree.id_at_cursor(self.todo_cursor) {
                    self.todo_tree.expand(id);
                }
            }
            KeyCode::Char('h') | KeyCode::Left => {
                if let Some(id) = self.todo_tree.id_at_cursor(self.todo_cursor) {
                    self.todo_tree.collapse(id);
                }
            }
            KeyCode::Char('a') => {
                self.todo_input_action = Some(TodoInputAction::AddSibling);
                self.app_mode = AppMode::TodoInput;
            }
            KeyCode::Char('A') => {
                if self.todo_tree.id_at_cursor(self.todo_cursor).is_some() {
                    self.todo_input_action = Some(TodoInputAction::AddChild);
                    self.app_mode = AppMode::TodoInput;
                }
            }
            KeyCode::Char('x') => {
                if let Some(id) = self.todo_tree.id_at_cursor(self.todo_cursor) {
                    if let Some(pool) = &self.pool {
                        db::todos::toggle_todo_done(pool, &id.to_string()).await?;
                        self.reload_todos().await?;
                    }
                }
            }
            KeyCode::Char('p') => {
                if let Some(id) = self.todo_tree.id_at_cursor(self.todo_cursor) {
                    if let Some(pool) = &self.pool {
                        db::todos::cycle_todo_priority(pool, &id.to_string()).await?;
                        self.reload_todos().await?;
                    }
                }
            }
            KeyCode::Char('e') => {
                if let Some(id) = self.todo_tree.id_at_cursor(self.todo_cursor) {
                    if let Some(item) = self.todo_tree.items.get(&id) {
                        self.todo_input.input = item.title.clone();
                        self.todo_input.character_index = item.title.chars().count();
                        self.todo_input_action = Some(TodoInputAction::EditTitle);
                        self.app_mode = AppMode::TodoInput;
                    }
                }
            }
            KeyCode::Enter => {
                // Select todo as current task
                if let Some(id) = self.todo_tree.id_at_cursor(self.todo_cursor) {
                    if let Some(item) = self.todo_tree.items.get(&id) {
                        let title = item.title.clone();
                        let _ = self.pomo_client.set_task_name(title).await;
                        self.active_todo_id = Some(id);
                        self.app_mode = AppMode::Normal;
                    }
                }
            }
            KeyCode::Esc | KeyCode::Char('t') => {
                self.app_mode = AppMode::Normal;
            }
            _ => {}
        }
        Ok(())
    }

    async fn commit_todo_input(&mut self, text: &str) -> anyhow::Result<()> {
        let Some(pool) = &self.pool else {
            return Ok(());
        };

        match self.todo_input_action {
            Some(TodoInputAction::AddSibling) => {
                let parent_id = self.todo_tree.parent_of_visible(self.todo_cursor);
                let parent_str = parent_id.flatten().map(|id| id.to_string());
                db::todos::insert_todo(pool, parent_str.as_deref(), text).await?;
            }
            Some(TodoInputAction::AddChild) => {
                if let Some(parent_id) = self.todo_tree.id_at_cursor(self.todo_cursor) {
                    let parent_str = parent_id.to_string();
                    db::todos::insert_todo(pool, Some(&parent_str), text).await?;
                    // Auto-expand parent to show the new child
                    self.todo_tree.expand(parent_id);
                }
            }
            Some(TodoInputAction::EditTitle) => {
                if let Some(id) = self.todo_tree.id_at_cursor(self.todo_cursor) {
                    db::todos::update_todo_title(pool, &id.to_string(), text).await?;
                }
            }
            None => {}
        }

        self.reload_todos().await?;
        Ok(())
    }

    async fn reload_todos(&mut self) -> anyhow::Result<()> {
        let Some(pool) = &self.pool else {
            return Ok(());
        };

        let rows = db::todos::get_all_todos(pool).await?;
        // Preserve expanded state
        let expanded_ids: std::collections::HashSet<Uuid> = self
            .todo_tree
            .items
            .iter()
            .filter(|(_, item)| item.expanded)
            .map(|(id, _)| *id)
            .collect();

        self.todo_tree = TodoTree::from_rows(rows);

        // Restore expanded state
        for id in expanded_ids {
            self.todo_tree.expand(id);
        }

        // Load session counts
        for (id, item) in self.todo_tree.items.iter_mut() {
            if let Ok(count) = db::todos::get_session_count_for_todo(pool, &id.to_string()).await {
                item.session_count = count;
            }
        }

        Ok(())
    }

    /// Executes a KeyCommand with direct dispatch for optimal performance
    async fn execute_command(&mut self, command: KeyCommand) -> anyhow::Result<()> {
        match command {
            KeyCommand::Quit => self.exit(),
            KeyCommand::ToggleHint => self.show_hint = !self.show_hint,
            KeyCommand::InputTask => {
                self.app_mode = match self.app_mode {
                    AppMode::Normal => AppMode::Input,
                    AppMode::Input => AppMode::Normal,
                    other => other,
                };
            }
            KeyCommand::OpenTodo => {
                if self.pool.is_some() {
                    self.reload_todos().await?;
                    self.app_mode = AppMode::Todo;
                }
            }
            KeyCommand::Reset => {
                self.pomo_client
                    .send_request(crate::protocol::Request::Reset)
                    .await?;
            }
            KeyCommand::Toggle => {
                if let Some(status) = &self.cached_status {
                    if status.is_paused || status.is_idle {
                        self.pomo_client
                            .send_request(crate::protocol::Request::Start)
                            .await?;
                    } else {
                        self.pomo_client
                            .send_request(crate::protocol::Request::Pause)
                            .await?;
                    }
                }
            }
            KeyCommand::SwitchMode => {
                self.pomo_client
                    .send_request(crate::protocol::Request::SwitchMode)
                    .await?;
            }
            KeyCommand::SetLong => {
                self.pomo_client
                    .send_request(crate::protocol::Request::SetPreset(
                        crate::timer::Preset::Long,
                    ))
                    .await?;
            }
            KeyCommand::SetShort => {
                self.pomo_client
                    .send_request(crate::protocol::Request::SetPreset(
                        crate::timer::Preset::Short,
                    ))
                    .await?;
            }
            KeyCommand::SetTest => {
                self.pomo_client
                    .send_request(crate::protocol::Request::SetPreset(
                        crate::timer::Preset::Test,
                    ))
                    .await?;
            }
        }
        Ok(())
    }

    fn exit(&mut self) {
        self.exit = true;
    }
}

impl Widget for &ServerApp {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let (render_color, remaining_time, mode_text, task_name) = match &self.cached_status {
            Some(status) => {
                let color = if status.is_paused {
                    Color::DarkGray
                } else {
                    match status.mode.as_str() {
                        "Work" => Color::Yellow,
                        "Break" => Color::Green,
                        _ => Color::White,
                    }
                };

                let time = utils::fmt_duration(std::time::Duration::from_secs(status.remaining));
                (color, time, status.mode.clone(), status.task.clone())
            }

            None => (
                Color::DarkGray,
                "00:00".to_string(),
                "Connecting...".to_string(),
                "".to_string(),
            ),
        };

        let mut text: Vec<Line<'static>> =
            create_large_ascii_numbers(&remaining_time, render_color);
        let state_info = Line::from(vec![
            Span::raw(mode_text),
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
            _ => Line::from(vec![Span::styled(
                task_name,
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
