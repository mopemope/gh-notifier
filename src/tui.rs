use crate::HistoryManager;
use crate::models::PersistedNotification;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use std::io;

pub struct TuiApp {
    history_manager: HistoryManager,
    notifications: Vec<PersistedNotification>,
    selected_index: usize,
    show_read: bool,
}

impl TuiApp {
    pub fn new(
        history_manager: HistoryManager,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let notifications = history_manager.get_all_notifications()?;
        let show_read = true;

        Ok(TuiApp {
            history_manager,
            notifications,
            selected_index: 0,
            show_read,
        })
    }

    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Run the main loop
        self.main_loop(&mut terminal)?;

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

    fn main_loop<B: Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        loop {
            terminal.draw(|f| self.ui(f))?;

            if let Event::Key(key) = event::read()?
                && self.handle_key_event(key)?
            {
                break; // Exit the main loop
            }
        }

        Ok(())
    }

    fn handle_key_event(
        &mut self,
        key: KeyEvent,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return Ok(true), // Exit
            KeyCode::Char('j') | KeyCode::Down => {
                if self.notifications.is_empty() {
                    self.selected_index = 0;
                } else {
                    self.selected_index = (self.selected_index + 1) % self.notifications.len();
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if !self.notifications.is_empty() {
                    if self.selected_index == 0 {
                        self.selected_index = self.notifications.len() - 1;
                    } else {
                        self.selected_index -= 1;
                    }
                }
            }
            KeyCode::Enter => {
                // Mark selected notification as read
                if let Some(notification) = self.notifications.get(self.selected_index) {
                    self.history_manager.mark_as_read(&notification.id)?;
                    // Refresh notifications after marking as read
                    self.notifications = self.history_manager.get_all_notifications()?;
                }
            }
            KeyCode::Char('r') => {
                // Refresh notifications list
                self.notifications = self.history_manager.get_all_notifications()?;
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Toggle show read/unread
                self.show_read = !self.show_read;
                self.notifications = if self.show_read {
                    self.history_manager.get_all_notifications()?
                } else {
                    self.history_manager.get_unread_notifications()?
                };
                if self.selected_index >= self.notifications.len() && !self.notifications.is_empty()
                {
                    self.selected_index = self.notifications.len() - 1;
                } else if self.notifications.is_empty() {
                    self.selected_index = 0;
                }
            }
            _ => {}
        }
        Ok(false) // Don't exit
    }

    fn ui(&self, f: &mut Frame) {
        let size = f.area();

        // Create the main layout
        let chunks = Layout::vertical([
            Constraint::Length(3), // Title
            Constraint::Min(10),   // Notification list
            Constraint::Length(3), // Help bar
        ])
        .split(size);

        // Title block
        let title_block = Block::default()
            .borders(Borders::ALL)
            .title("GitHub Notifier - Notification TUI")
            .title_alignment(ratatui::layout::Alignment::Center);
        let title = Paragraph::new("GitHub Notifier - Persistent Notification Viewer")
            .block(title_block)
            .alignment(ratatui::layout::Alignment::Center)
            .style(Style::default().fg(Color::Cyan));
        f.render_widget(title, chunks[0]);

        // Notification list
        let items: Vec<ListItem> = self
            .notifications
            .iter()
            .enumerate()
            .map(|(i, notification)| {
                let status = if notification.is_read {
                    "READ "
                } else {
                    "UNREAD"
                };
                let read_status_color = if notification.is_read {
                    Color::DarkGray
                } else {
                    Color::Yellow
                };

                let line1 = format!(
                    "[{}] {} - {} ({})",
                    status, notification.repository, notification.title, notification.reason
                );

                let line2 = format!(
                    "  Type: {} | Received: {}",
                    notification.subject_type, notification.received_at
                );

                let content = format!("{}\n{}", line1, line2);

                // Highlight selected item
                let style = if i == self.selected_index {
                    Style::default()
                        .fg(Color::White)
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(read_status_color)
                };

                ListItem::new(content).style(style)
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(format!(
                "Notifications ({} total)",
                self.notifications.len()
            )))
            .highlight_style(
                Style::default()
                    .bg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            );

        // Create list state for selection
        let mut state = ListState::default();
        state.select(Some(self.selected_index));

        f.render_stateful_widget(list, chunks[1], &mut state);

        // Help bar
        let help_text =
            "q:Quit | j/k:Down/Up | Enter:Mark as Read | r:Refresh | Ctrl+u:Toggle Read/Unread";
        let help_block = Block::default().borders(Borders::ALL).title("Help");
        let help = Paragraph::new(help_text)
            .block(help_block)
            .alignment(ratatui::layout::Alignment::Center)
            .style(Style::default().fg(Color::Gray));
        f.render_widget(help, chunks[2]);
    }
}
