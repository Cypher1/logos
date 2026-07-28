use crate::commands::{AppContext, COMMAND_PREFIX, CommandRegistry, get_default_registry};
use crate::config::Config;
use crate::kb::KB;
use crate::ui::{LogosUI, UIFocus};

use anyhow::{Context, Result};
use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, KeyCode, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};

use ratatui::{Terminal, backend::Backend};
use ratatui_textarea::{CursorMove, TextArea};
use signal_hook::consts::signal::SIGTSTP;

use std::io::Write;
use std::sync::{Arc, Mutex, mpsc};

use ollama_rs::generation::chat::request::ChatMessageRequest;
use ollama_rs::generation::chat::{ChatMessage, ChatMessageResponse};
use tokio_stream::StreamExt;

pub enum InputSignal {
    Continue,
    Break,
    Suspend,
}

pub struct App<'a, B: Backend> {
    pub config: Config,
    pub kb: KB,
    pub terminal: Terminal<B>,
    pub ui: LogosUI<'a>,
    #[cfg(feature = "ollama")]
    pub tx: mpsc::Sender<Result<ollama_rs::generation::chat::ChatMessageResponse>>,
    #[cfg(feature = "ollama")]
    pub rx: mpsc::Receiver<Result<ChatMessageResponse>>,
    #[cfg(feature = "ollama")]
    pub history: Arc<Mutex<Vec<ChatMessage>>>,
    pub registry: CommandRegistry,
}

impl<'a, B: Backend + Write> App<'a, B>
where
    <B as Backend>::Error: 'static + Sync + Send,
{
    pub fn new(config: Config, kb: KB, terminal: Terminal<B>) -> Self {
        #[cfg(feature = "ollama")]
        let (tx, rx) = mpsc::channel();
        #[cfg(feature = "ollama")]
        let history = Arc::new(Mutex::new(vec![]));

        App {
            config,
            kb,
            terminal,
            ui: LogosUI::new(),
            #[cfg(feature = "ollama")]
            tx,
            #[cfg(feature = "ollama")]
            rx,
            #[cfg(feature = "ollama")]
            history,
            registry: get_default_registry(),
        }
    }

    pub fn update_kb_view(&mut self) -> Result<()> {
        self.ui.knowledge_base.clear();
        let mut count = 0;
        self.kb
            .for_each_tuple(|tuple| {
                self.ui.knowledge_base.push(tuple.to_string());
                count += 1;
            })
            .context("loading kb")?;
        self.ui.system_state = format!("Loaded {} tuples from KB", count);
        Ok(())
    }

    pub async fn run(&mut self) -> Result<()> {
        loop {
            self.update_kb_view()?;
            self.terminal.draw(|f| self.ui.render(f))?;

            #[cfg(feature = "ollama")]
            let _ = self.handle_messages().await?;

            match self.handle_input().await? {
                InputSignal::Continue => {}
                InputSignal::Break => break,
                InputSignal::Suspend => {
                    self.leave().await?;
                    signal_hook::low_level::emulate_default_handler(SIGTSTP).unwrap();
                    self.enter().await?;
                }
            }
        }
        Ok(())
    }

    pub async fn enter(&mut self) -> Result<()> {
        enable_raw_mode().unwrap();
        execute!(
            self.terminal.backend_mut(),
            EnterAlternateScreen,
            EnableMouseCapture
        )?;
        self.terminal.clear()?;
        Ok(())
    }

    pub async fn leave(&mut self) -> Result<()> {
        disable_raw_mode().unwrap();
        execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        self.terminal.show_cursor()?;
        Ok(())
    }

    #[cfg(feature = "ollama")]
    pub async fn handle_messages(&mut self) -> Result<InputSignal> {
        while let Ok(resp) = self.rx.try_recv() {
            match resp {
                Ok(ollama_rs::generation::chat::ChatMessageResponse { done: true, .. }) => {}
                Ok(ollama_rs::generation::chat::ChatMessageResponse {
                    message: ollama_rs::generation::chat::ChatMessage { content: chunk, .. },
                    done: false,
                    ..
                }) => {
                    if let Some(last_msg) = self.ui.messages.last_mut() {
                        if last_msg.starts_with("Logos: ") {
                            last_msg.push_str(&chunk);
                        } else {
                            self.ui.messages.push(format!("Logos: {}", chunk));
                        }
                    } else {
                        self.ui.messages.push(format!("Logos: {}", chunk));
                    }
                }
                Err(e) => {
                    self.ui.messages.push(format!("Error from Logos: {}", e));
                }
            }
        }
        Ok(InputSignal::Continue)
    }

    pub async fn handle_input(&mut self) -> Result<InputSignal> {
        if !event::poll(std::time::Duration::from_millis(10))? {
            return Ok(InputSignal::Continue);
        }

        let event = event::read()?;
        match event {
            event::Event::Mouse(mouse_event) => {
                let col = mouse_event.column;
                let row = mouse_event.row;

                let contains = |rect: ratatui::layout::Rect| -> bool {
                    col >= rect.x
                        && col < rect.x + rect.width
                        && row >= rect.y
                        && row < rect.y + rect.height
                };

                match mouse_event.kind {
                    event::MouseEventKind::Down(event::MouseButton::Left) => {
                        if contains(self.ui.chat_area) {
                            self.ui.focus = UIFocus::Chat;
                        } else if contains(self.ui.kb_area) {
                            self.ui.focus = UIFocus::KB;
                        } else if contains(self.ui.planning_area) {
                            self.ui.focus = UIFocus::Planning;
                        } else if contains(self.ui.input_area) {
                            self.ui.focus = UIFocus::Input;
                        }
                    }
                    event::MouseEventKind::ScrollUp => {
                        if contains(self.ui.chat_area) {
                            self.ui.chat_scroll = self.ui.chat_scroll.saturating_sub(1);
                        } else if contains(self.ui.kb_area) {
                            self.ui.kb_scroll = self.ui.kb_scroll.saturating_sub(1);
                        } else if contains(self.ui.planning_area) {
                            self.ui.planning_scroll = self.ui.planning_scroll.saturating_sub(1);
                        }
                    }
                    event::MouseEventKind::ScrollDown => {
                        if contains(self.ui.chat_area) {
                            self.ui.chat_scroll = self.ui.chat_scroll.saturating_add(1);
                        } else if contains(self.ui.kb_area) {
                            self.ui.kb_scroll = self.ui.kb_scroll.saturating_add(1);
                        } else if contains(self.ui.planning_area) {
                            self.ui.planning_scroll = self.ui.planning_scroll.saturating_add(1);
                        }
                    }
                    _ => {}
                }
            }
            event::Event::Key(key) => {
                if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('c') {
                    return Ok(InputSignal::Break);
                }
                if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('z') {
                    return Ok(InputSignal::Suspend);
                }

                match self.ui.focus {
                    UIFocus::Input => match key.code {
                        KeyCode::Enter => self.submit_input()?,
                        KeyCode::Up => {
                            let pos = match self.ui.history_pos {
                                None => {
                                    let user_input = self.ui.input_textarea.lines().join("\n");
                                    if !user_input.trim().is_empty() {
                                        self.ui.input_history.insert(0, user_input);
                                        1
                                    } else {
                                        0
                                    }
                                }
                                Some(mut pos) => {
                                    if pos < self.ui.input_history.len() {
                                        pos += 1;
                                    }
                                    pos
                                }
                            };
                            self.ui.history_pos = Some(pos);
                            if let Some(hist) = self.ui.input_history.get(pos) {
                                self.ui.input_textarea = TextArea::from(hist.lines());
                                self.ui.input_textarea.move_cursor(CursorMove::Bottom);
                                self.ui.input_textarea.move_cursor(CursorMove::End);
                            }
                        }
                        KeyCode::Down => match self.ui.history_pos {
                            None => {
                                let user_input = self.ui.input_textarea.lines().join("\n");
                                if !user_input.trim().is_empty() {
                                    self.ui
                                        .input_history
                                        .insert(0, self.ui.input_textarea.lines().join("\n"));
                                }
                                self.ui.input_textarea.clear();
                                self.ui.history_pos = None;
                            }
                            Some(0) => {
                                self.ui.input_textarea.clear();
                                self.ui.history_pos = None;
                            }
                            Some(pos) => {
                                self.ui.history_pos = Some(pos - 1);
                                if let Some(hist) = self.ui.input_history.get(pos - 1) {
                                    self.ui.input_textarea = TextArea::from(hist.lines());
                                    self.ui.input_textarea.move_cursor(CursorMove::Bottom);
                                    self.ui.input_textarea.move_cursor(CursorMove::End);
                                }
                            }
                        },
                        KeyCode::Tab => {
                            if key.modifiers == KeyModifiers::CONTROL {
                                let next_focus = match self.ui.focus {
                                    UIFocus::Input => UIFocus::Chat,
                                    UIFocus::Chat => UIFocus::KB,
                                    UIFocus::KB => UIFocus::Planning,
                                    UIFocus::Planning => UIFocus::Input,
                                };
                                self.ui.focus = next_focus;
                            } else if self.ui.focus == UIFocus::Input {
                                let current_text = self.ui.input_textarea.lines().join("\n");
                                if let Some(prefix) = current_text.strip_prefix(COMMAND_PREFIX) {
                                    let tail = current_text
                                        .strip_prefix(&format!("{}{}", COMMAND_PREFIX, prefix))
                                        .unwrap_or_default();
                                    // TODO: Add UI for selecting a match.
                                    // For now select the first match.
                                    let matches = self.registry.find_matches(prefix);
                                    if !matches.is_empty() {
                                        // Take the first match as simple autocomplete
                                        let best_match = &matches[0];
                                        self.ui.input_textarea = TextArea::from(
                                            format!("{}{} {}", COMMAND_PREFIX, best_match, tail,)
                                                .lines(),
                                        );
                                        let new_pos = COMMAND_PREFIX.chars().count() + best_match.chars().count();
                                        self.ui.input_textarea.move_cursor(CursorMove::Jump(0, new_pos as u16));
                                    }
                                }
                            }
                        }
                        _ => {
                            self.ui.input_textarea.input(key);
                        }
                    },
                    UIFocus::Chat => match key.code {
                        KeyCode::Tab => {
                            self.ui.focus = UIFocus::KB;
                        }
                        KeyCode::Up => {
                            self.ui.chat_scroll = self.ui.chat_scroll.saturating_sub(1);
                        }
                        KeyCode::Down => {
                            self.ui.chat_scroll = self.ui.chat_scroll.saturating_add(1);
                        }
                        _ => {}
                    },
                    UIFocus::KB => match key.code {
                        KeyCode::Tab => {
                            self.ui.focus = UIFocus::Planning;
                        }
                        KeyCode::Up => {
                            self.ui.kb_scroll = self.ui.kb_scroll.saturating_sub(1);
                        }
                        KeyCode::Down => {
                            self.ui.kb_scroll = self.ui.kb_scroll.saturating_add(1);
                        }
                        KeyCode::Char('/') | KeyCode::Char('r') => {
                            self.update_kb_view()?;
                        }
                        _ => {}
                    },
                    UIFocus::Planning => match key.code {
                        KeyCode::Tab => {
                            self.ui.focus = UIFocus::Input;
                        }
                        KeyCode::Up => {
                            self.ui.planning_scroll = self.ui.planning_scroll.saturating_sub(1);
                        }
                        KeyCode::Down => {
                            self.ui.planning_scroll = self.ui.planning_scroll.saturating_add(1);
                        }
                        _ => {}
                    },
                }
            }
            _ => {}
        }
        Ok(InputSignal::Continue)
    }

    pub fn submit_input(&mut self) -> Result<()> {
        let user_input = self.ui.input_textarea.lines().join("\n");
        if user_input.trim().is_empty() {
            return Ok(());
        }
        if let Some(user_input) = user_input.strip_prefix(COMMAND_PREFIX) {
            let mut args = vec![];
            let name = if user_input.contains(" ") {
                let mut parts: Vec<&str> = user_input.split(" ").collect();
                let name = parts.remove(0);
                args.extend(parts);
                name
            } else {
                user_input
            };
            let mut context = AppContext {
                registry: &mut self.registry,
                kb: &mut self.kb,
                ui: &mut self.ui,
            };
            context.execute(name, args)?;
            return Ok(());
        }
        self.send_message(user_input)
    }

    pub fn send_message(&mut self, user_input: String) -> Result<()> {
        self.ui.messages.push(format!("You: {}", user_input));
        if self.ui.input_history.is_empty() || self.ui.input_history[0] != user_input {
            self.ui.input_history.insert(0, user_input.clone());
        }
        self.ui.history_pos = None;
        self.ui.input_textarea.clear();

        #[cfg(feature = "ollama")]
        {
            let tx = self.tx.clone();
            let config = self.config.clone();
            let history_ref = self.history.clone();
            tokio::spawn(async move {
                let ollama = ollama_rs::Ollama::default();
                let options = ollama_rs::models::ModelOptions::default()
                    .temperature(config.temperature)
                    .num_predict(config.max_tokens)
                    .top_p(config.top_p)
                    .top_k(config.top_k)
                    .repeat_penalty(config.repeat_penalty)
                    .stop(config.stop);
                let request =
                    ChatMessageRequest::new(config.model, vec![ChatMessage::user(user_input)])
                        .options(options);
                let stream = ollama
                    .send_chat_messages_with_history_stream(history_ref, request)
                    .await;
                let mut stream: ollama_rs::generation::chat::ChatMessageResponseStream =
                    match stream {
                        Ok(s) => s,
                        Err(e) => {
                            let _ = tx.send(Err(anyhow::anyhow!("{}", e)));
                            return;
                        }
                    };

                while let Some(result) = stream.next().await {
                    if let Ok(result) = result {
                        let _ = tx.send(Ok(result));
                    }
                }
            });
        }
        Ok(())
    }
}
