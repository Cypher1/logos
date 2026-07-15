//! User interface components for the Logos system.
//! This module implements the ratatui-based terminal UI with multiple panels.

use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
    Frame,
};

/// UI Panel Focus Options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UIFocus {
    Input,
    Chat,
    KB,
    Planning,
}

/// Main UI structure for Logos system
pub struct LogosUI<'a> {
    /// Chat message history
    pub messages: Vec<String>,
    /// Knowledge base content
    pub knowledge_base: Vec<String>,
    /// Planning/simulation status
    pub planning_status: String,
    /// Current system state
    pub system_state: String,
    /// Current input buffer
    pub input: String,
    /// Cursor position in input buffer
    pub cursor_pos: usize,
    /// Input history for up/down navigation
    pub input_history: Vec<String>,
    /// Current position in history navigation
    pub history_pos: usize,
    /// Currently focused panel
    pub focus: UIFocus,
    /// Chat scroll offset
    pub chat_scroll: usize,
    /// KB scroll offset
    pub kb_scroll: usize,
    /// Planning scroll offset
    pub planning_scroll: usize,
    /// Cached Chat area
    pub chat_area: Rect,
    /// Cached KB area
    pub kb_area: Rect,
    /// Cached Planning area
    pub planning_area: Rect,
    /// Cached Input area
    pub input_area: Rect,
    /// Text area for input (replaced simple string)
    pub input_textarea: ratatui_textarea::TextArea<'a>,
}

impl<'a> LogosUI<'a> {
    /// Create a new LogosUI instance
    pub fn new() -> Self {
        Self {
            messages: vec![
                "Starting Logos: A Foundational Problem-Solving Architecture".to_string(),
                "Welcome to the Logos system interface.".to_string(),
            ],
            knowledge_base: vec!["Loading knowledge entries...".to_string()],
            planning_status: "Ready for new task".to_string(),
            system_state: "Operational".to_string(),
            input: "".to_string(),
            cursor_pos: 0,
            input_history: Vec::new(),
            history_pos: 0,
            focus: UIFocus::Input,
            chat_scroll: 0,
            kb_scroll: 0,
            planning_scroll: 0,
            chat_area: Rect::default(),
            kb_area: Rect::default(),
            planning_area: Rect::default(),
            input_area: Rect::default(),
            input_textarea: ratatui_textarea::TextArea::default(),
        }
    }

    /// Render the complete UI layout
    pub fn render(&mut self, frame: &mut Frame) {
        // Define the main layout with 3 horizontal areas:
        // 1. Top status bar
        // 2. Main content area (chat + panels)
        // 3. Bottom input area
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Status bar
                Constraint::Min(10),   // Main content
                Constraint::Length(3), // Input
            ])
            .split(frame.area());

        self.render_status_bar(frame, chunks[0]);
        self.render_main_content(frame, chunks[1]);
        self.render_input_area(frame, chunks[2]);
    }

    /// Render the status bar at the top
    fn render_status_bar(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title("Logos System Status")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Blue));

        let text = Text::raw(&self.system_state);
        let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }

    /// Render the main content area with all panels
    fn render_main_content(&mut self, frame: &mut Frame, area: Rect) {
        // Split main area horizontally into 3 panels
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(70), // Chat panel (main)
                Constraint::Percentage(30), // Knowledge base + planning
            ])
            .split(area);

        self.chat_area = chunks[0];
        let side_bar = chunks[1];

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(50), // Knowledge base
                Constraint::Percentage(50), // Planning / Simulation
            ])
            .split(side_bar);

        self.kb_area = chunks[0];
        self.planning_area = chunks[1];

        self.render_knowledge_base_panel(frame, self.kb_area);
        self.render_chat_panel(frame, self.chat_area);
        self.render_planning_panel(frame, self.planning_area);
    }

    /// Render the chat panel showing conversation history
    fn render_chat_panel(&mut self, frame: &mut Frame, area: Rect) {
        let is_focused = self.focus == UIFocus::Chat;
        let border_color = if is_focused {
            Color::White
        } else {
            Color::DarkGray
        };
        let border_style = Style::default().fg(border_color);
        let title_style = if is_focused {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let title = if is_focused {
            "Chat Interface (Active)"
        } else {
            "Chat Interface"
        };

        let block = Block::default()
            .title(Span::styled(title, title_style))
            .borders(Borders::ALL)
            .border_style(border_style);

        frame.render_widget(block, area);

        // Display chat messages
        let text: Text = self
            .messages
            .iter()
            .map(|msg| Line::from(vec![Span::raw(msg.clone())]))
            .collect();

        let inner_area = area.inner(Margin::new(1, 1));
        let inner_height = inner_area.height as usize;
        let total_lines = self.messages.len();
        let max_scroll = total_lines.saturating_sub(inner_height);
        self.chat_scroll = self.chat_scroll.min(max_scroll);

        let paragraph = Paragraph::new(text)
            .block(Block::default().borders(Borders::NONE))
            .wrap(Wrap { trim: true })
            .scroll((self.chat_scroll as u16, 0));

        frame.render_widget(paragraph, inner_area);

        // Render scrollbar if content exceeds area height
        if total_lines > inner_height {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("▲"))
                .end_symbol(Some("▼"));
            let mut scrollbar_state = ScrollbarState::new(total_lines).position(self.chat_scroll);

            frame.render_stateful_widget(
                scrollbar,
                area.inner(Margin {
                    vertical: 1,
                    horizontal: 0,
                }),
                &mut scrollbar_state,
            );
        }
    }

    /// Render the knowledge base management panel
    fn render_knowledge_base_panel(&mut self, frame: &mut Frame, area: Rect) {
        let is_focused = self.focus == UIFocus::KB;
        let border_color = if is_focused {
            Color::Green
        } else {
            Color::DarkGray
        };
        let border_style = Style::default().fg(border_color);
        let title_style = if is_focused {
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let title = if is_focused {
            "Knowledge Base (Active)"
        } else {
            "Knowledge Base"
        };

        let block = Block::default()
            .title(Span::styled(title, title_style))
            .borders(Borders::ALL)
            .border_style(border_style);

        frame.render_widget(block, area);

        // Display knowledge entries
        let text: Text = self
            .knowledge_base
            .iter()
            .map(|entry| Line::from(vec![Span::raw(entry.clone())]))
            .collect();

        let inner_area = area.inner(Margin::new(1, 1));
        let inner_height = inner_area.height as usize;
        let total_lines = self.knowledge_base.len();
        let max_scroll = total_lines.saturating_sub(inner_height);
        self.kb_scroll = self.kb_scroll.min(max_scroll);

        let paragraph = Paragraph::new(text)
            .block(Block::default().borders(Borders::NONE))
            .wrap(Wrap { trim: true })
            .scroll((self.kb_scroll as u16, 0));

        frame.render_widget(paragraph, inner_area);

        // Render scrollbar if content exceeds area height
        if total_lines > inner_height {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("▲"))
                .end_symbol(Some("▼"));
            let mut scrollbar_state = ScrollbarState::new(total_lines).position(self.kb_scroll);

            frame.render_stateful_widget(
                scrollbar,
                area.inner(Margin {
                    vertical: 1,
                    horizontal: 0,
                }),
                &mut scrollbar_state,
            );
        }
    }

    /// Render the planning/simulation tools panel
    fn render_planning_panel(&mut self, frame: &mut Frame, area: Rect) {
        let is_focused = self.focus == UIFocus::Planning;
        let border_color = if is_focused {
            Color::Yellow
        } else {
            Color::DarkGray
        };
        let border_style = Style::default().fg(border_color);
        let title_style = if is_focused {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let title = if is_focused {
            "Planning/Simulation Tools (Active)"
        } else {
            "Planning/Simulation Tools"
        };

        let block = Block::default()
            .title(Span::styled(title, title_style))
            .borders(Borders::ALL)
            .border_style(border_style);

        frame.render_widget(block, area);

        // Display planning information
        let lines: Vec<String> = self
            .planning_status
            .lines()
            .map(|s| s.to_string())
            .collect();
        let text: Text = lines
            .iter()
            .map(|line| Line::from(vec![Span::raw(line.clone())]))
            .collect();

        let inner_area = area.inner(Margin::new(1, 1));
        let inner_height = inner_area.height as usize;
        let total_lines = lines.len();
        let max_scroll = total_lines.saturating_sub(inner_height);
        self.planning_scroll = self.planning_scroll.min(max_scroll);

        let paragraph = Paragraph::new(text)
            .block(Block::default().borders(Borders::NONE))
            .wrap(Wrap { trim: true })
            .scroll((self.planning_scroll as u16, 0));

        frame.render_widget(paragraph, inner_area);

        // Render scrollbar if content exceeds area height
        if total_lines > inner_height {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("▲"))
                .end_symbol(Some("▼"));
            let mut scrollbar_state =
                ScrollbarState::new(total_lines).position(self.planning_scroll);

            frame.render_stateful_widget(
                scrollbar,
                area.inner(Margin {
                    vertical: 1,
                    horizontal: 0,
                }),
                &mut scrollbar_state,
            );
        }
    }

    /// Render the input area at the bottom
    fn render_input_area(&mut self, frame: &mut Frame, area: Rect) {
        self.input_area = area;
        let is_focused = self.focus == UIFocus::Input;
        let border_color = if is_focused {
            Color::Cyan
        } else {
            Color::DarkGray
        };
        let border_style = Style::default().fg(border_color);
        let title_style = if is_focused {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let title = if is_focused {
            "Input (Active)"
        } else {
            "Input"
        };

        let block = Block::default()
            .title(Span::styled(title, title_style))
            .borders(Borders::ALL)
            .border_style(border_style);

        // Draw the textarea widget inside the input area
        let mut textarea = self.input_textarea.clone();
        textarea.set_block(block);
        frame.render_widget(&textarea, area);
    }
}
