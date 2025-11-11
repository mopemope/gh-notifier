use crate::HistoryManager;
use crate::models::PersistedNotification;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use std::io::stdout;

pub struct TuiApp {
    history_manager: HistoryManager,
    notifications: Vec<PersistedNotification>,
    selected_index: usize,
    app_state: AppState,
    should_quit: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    List,
    Detail,
    Quit,
}

impl TuiApp {
    pub fn new(
        history_manager: HistoryManager,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let notifications = history_manager.get_all_notifications()?;
        Ok(TuiApp {
            history_manager,
            notifications,
            selected_index: 0,
            app_state: AppState::List,
            should_quit: false,
        })
    }

    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Run application
        self.init()?;
        while !self.should_quit {
            terminal.draw(|f| self.draw(f))?;
            self.handle_events()?;
        }

        // Restore terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        Ok(())
    }

    fn init(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.update_notifications()?;
        Ok(())
    }

    fn update_notifications(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.notifications = self.history_manager.get_all_notifications()?;
        Ok(())
    }

    fn draw(&self, f: &mut Frame) {
        match self.app_state {
            AppState::List => self.draw_notification_list(f),
            AppState::Detail => self.draw_notification_detail(f),
            AppState::Quit => self.draw_quit_message(f),
        }
    }

    fn draw_notification_list(&self, f: &mut Frame) {
        let chunks = Layout::vertical([
            Constraint::Length(3), // Title
            Constraint::Min(0),    // Notification list
            Constraint::Length(3), // Help
        ])
        .split(f.area());

        // Title
        let title = Paragraph::new("GitHub Notifications")
            .block(Block::default().borders(Borders::BOTTOM))
            .alignment(ratatui::layout::Alignment::Center)
            .style(Style::default().add_modifier(Modifier::BOLD));
        f.render_widget(title, chunks[0]);

        // Notification list
        let items: Vec<ListItem> = self
            .notifications
            .iter()
            .map(|notification| {
                let status = if notification.is_read {
                    "READ"
                } else {
                    "UNREAD"
                };
                let status_color = if notification.is_read {
                    Color::DarkGray
                } else {
                    Color::Green
                };
                let title = format!("{} - {}", notification.repository, notification.title);
                let info = format!("({}) {}", notification.reason, notification.received_at);

                let content = vec![
                    Line::from(vec![
                        Span::styled(format!("[{}] ", status), Style::default().fg(status_color)),
                        Span::styled(title, Style::default().add_modifier(Modifier::BOLD)),
                    ]),
                    Line::from(Span::styled(info, Style::default().fg(Color::Gray))),
                ];

                ListItem::new(content)
            })
            .collect();

        let mut state = ListState::default();
        state.select(Some(self.selected_index));

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::TOP | Borders::BOTTOM)
                    .title(format!("Notifications ({})", self.notifications.len())),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            );

        f.render_stateful_widget(list, chunks[1], &mut state);

        // Help
        let help_text =
            "Use ↑↓ to navigate, Enter for details, r to mark as read, d to delete, q to quit";
        let help = Paragraph::new(help_text)
            .block(Block::default().borders(Borders::TOP))
            .style(Style::default().fg(Color::Yellow));
        f.render_widget(help, chunks[2]);
    }

    fn draw_notification_detail(&self, f: &mut Frame) {
        if self.selected_index >= self.notifications.len() {
            return;
        }

        let notification = &self.notifications[self.selected_index];
        let chunks = Layout::vertical([
            Constraint::Length(3), // Title
            Constraint::Min(0),    // Details
            Constraint::Length(3), // Help
        ])
        .split(f.area());

        // Title
        let title = Paragraph::new("Notification Details")
            .block(Block::default().borders(Borders::BOTTOM))
            .alignment(ratatui::layout::Alignment::Center)
            .style(Style::default().add_modifier(Modifier::BOLD));
        f.render_widget(title, chunks[0]);

        // Details
        let details = format!(
            "Title: {}\n\nRepository: {}\n\nReason: {}\n\nType: {}\n\nStatus: {}\n\nReceived at: {}\n\nURL: {}\n\nBody:\n{}",
            notification.title,
            notification.repository,
            notification.reason,
            notification.subject_type,
            if notification.is_read {
                "READ"
            } else {
                "UNREAD"
            },
            notification.received_at,
            notification.url,
            notification.body,
        );

        let details_paragraph = Paragraph::new(details)
            .block(
                Block::default()
                    .borders(Borders::TOP | Borders::BOTTOM)
                    .title("Notification Details"),
            )
            .wrap(ratatui::widgets::Wrap { trim: false });

        f.render_widget(details_paragraph, chunks[1]);

        // Help
        let help_text = "Press Esc to return to list, r to mark as read, d to delete, q to quit";
        let help = Paragraph::new(help_text)
            .block(Block::default().borders(Borders::TOP))
            .style(Style::default().fg(Color::Yellow));
        f.render_widget(help, chunks[2]);
    }

    fn draw_quit_message(&self, f: &mut Frame) {
        let text = vec![
            Line::from(""),
            Line::from("Thanks for using GitHub Notifier!"),
            Line::from(""),
        ];
        let paragraph = Paragraph::new(text)
            .block(Block::default())
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(paragraph, f.area());
    }

    fn handle_events(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if event::poll(std::time::Duration::from_millis(50))?
            && let Event::Key(key) = event::read()?
        {
            self.handle_key_event(key)?;
        }
        Ok(())
    }

    fn handle_key_event(
        &mut self,
        key: KeyEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match self.app_state {
            AppState::List => self.handle_list_key_event(key)?,
            AppState::Detail => self.handle_detail_key_event(key)?,
            AppState::Quit => {
                if key.code == KeyCode::Char('q') || key.code == KeyCode::Char('Q') {
                    self.should_quit = true;
                }
            }
        }
        Ok(())
    }

    fn handle_list_key_event(
        &mut self,
        key: KeyEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_quit = true;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.notifications.is_empty() {
                    self.selected_index = (self.selected_index + 1) % self.notifications.len();
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if !self.notifications.is_empty() {
                    self.selected_index = self
                        .selected_index
                        .checked_sub(1)
                        .unwrap_or(self.notifications.len() - 1);
                }
            }
            KeyCode::Enter => {
                if !self.notifications.is_empty() {
                    self.app_state = AppState::Detail;
                }
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.mark_selected_as_read()?;
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                self.delete_selected()?;
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_detail_key_event(
        &mut self,
        key: KeyEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.app_state = AppState::List;
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.mark_selected_as_read()?;
                self.app_state = AppState::List;
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                self.delete_selected()?;
                self.app_state = AppState::List;
            }
            _ => {}
        }
        Ok(())
    }

    fn mark_selected_as_read(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.selected_index < self.notifications.len() {
            let notification_id = &self.notifications[self.selected_index].id;
            self.history_manager.mark_as_read(notification_id)?;
            self.update_notifications()?;
            // Adjust selection index after refresh
            if self.selected_index >= self.notifications.len() && !self.notifications.is_empty() {
                self.selected_index = self.notifications.len() - 1;
            }
        }
        Ok(())
    }

    fn delete_selected(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.selected_index < self.notifications.len() {
            let notification_id = &self.notifications[self.selected_index].id;
            self.history_manager.delete_notification(notification_id)?;
            self.update_notifications()?;
            // Adjust selection index after deletion
            if self.selected_index >= self.notifications.len() && !self.notifications.is_empty() {
                self.selected_index = self.notifications.len() - 1;
            } else if self.notifications.is_empty() {
                self.selected_index = 0;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_tui_app_creation() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let history_manager = HistoryManager::new(&db_path).unwrap();
        let app_result = TuiApp::new(history_manager);

        assert!(app_result.is_ok());
    }
}
