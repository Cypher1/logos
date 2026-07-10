//! User interface components for the Logos system.
//! This module implements the ratatui-based terminal UI with multiple panels.

use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

/// Main UI structure for Logos system
pub struct LogosUI {
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
}

impl LogosUI {
    /// Create a new LogosUI instance
    pub fn new() -> Self {
        Self {
            messages: vec![
                "Starting Logos: A Foundational Problem-Solving Architecture".to_string(),
                "Welcome to the Logos system interface.".to_string(),
            ],
            knowledge_base: vec![
                "Knowledge base initialized successfully".to_string(),
                "Loaded 123 knowledge entries".to_string(),
            ],
            planning_status: "Ready for new task".to_string(),
            system_state: "Operational".to_string(),
            input: "".to_string(),
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
            .split(frame.size());

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

        frame.render_widget(block, area);
    }

    /// Render the main content area with all panels
    fn render_main_content(&mut self, frame: &mut Frame, area: Rect) {
        // Split main area horizontally into 3 panels
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(60), // Chat panel (main)
                Constraint::Percentage(20), // Knowledge base
                Constraint::Percentage(20), // Planning/simulation
            ])
            .split(area);

        self.render_chat_panel(frame, chunks[0]);
        self.render_knowledge_base_panel(frame, chunks[1]);
        self.render_planning_panel(frame, chunks[2]);
    }

    /// Render the chat panel showing conversation history
    fn render_chat_panel(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title("Chat Interface")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White));

        frame.render_widget(block, area);

        // Display chat messages
        let text: Text = self
            .messages
            .iter()
            .map(|msg| Line::from(vec![Span::raw(msg.clone())]))
            .collect();

        let paragraph = Paragraph::new(text)
            .block(Block::default().borders(Borders::NONE))
            .wrap(Wrap { trim: true });

        let inner_area = area.inner(&Margin::new(1, 1));
        frame.render_widget(paragraph, inner_area);
    }

    /// Render the knowledge base management panel
    fn render_knowledge_base_panel(&mut self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title("Knowledge Base")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Green));

        frame.render_widget(block, area);

        // Display knowledge entries
        let text: Text = self
            .knowledge_base
            .iter()
            .map(|entry| Line::from(vec![Span::raw(entry.clone())]))
            .collect();

        let paragraph = Paragraph::new(text)
            .block(Block::default().borders(Borders::NONE))
            .wrap(Wrap { trim: true });

        let inner_area = area.inner(&Margin::new(1, 1));
        frame.render_widget(paragraph, inner_area);
    }

    /// Render the planning/simulation tools panel
    fn render_planning_panel(&mut self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title("Planning/Simulation Tools")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Yellow));

        frame.render_widget(block, area);

        // Display planning information
        let text: Text = vec![self.planning_status.clone()]
            .into_iter()
            .map(|msg| Line::from(vec![Span::raw(msg)]))
            .collect();

        let paragraph = Paragraph::new(text)
            .block(Block::default().borders(Borders::NONE))
            .wrap(Wrap { trim: true });

        let inner_area = area.inner(&Margin::new(1, 1));
        frame.render_widget(paragraph, inner_area);
    }

    /// Render the input area at the bottom
    fn render_input_area(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title("Input")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Cyan));

        // Display the current input with a prompt
        let input_line = format!("> {}", self.input);
        let text = Text::from(vec![Line::from(Span::raw(input_line))]);
        let paragraph = Paragraph::new(text)
            .block(block)
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }
}