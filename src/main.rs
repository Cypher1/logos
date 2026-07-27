mod commands;
mod config;
mod kb;
mod ui;

use crate::commands::AppContext;
use crate::config::Config;
use crate::kb::{KB, Tuple};
use crate::ui::{LogosUI, UIFocus};
#[cfg(feature = "ollama")]
use anyhow::anyhow;
use anyhow::{Context, Result};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
#[cfg(feature = "ollama")]
use ollama_rs::{
    Ollama,
    generation::chat::{
        ChatMessage, ChatMessageResponse, ChatMessageResponseStream, request::ChatMessageRequest,
    },
    models::ModelOptions,
};
use ratatui::{Terminal, backend::Backend, backend::CrosstermBackend};
use ratatui_textarea::TextArea;
use signal_hook::consts::signal::*;
use std::io::Write;
use std::path::PathBuf;
#[cfg(feature = "ollama")]
use std::sync::mpsc;
#[cfg(feature = "ollama")]
use std::sync::{Arc, Mutex};
#[cfg(feature = "ollama")]
use tokio_stream::StreamExt;

enum InputSignal {
    Continue,
    Break,
    Suspend,
}

struct App<'a, B: Backend> {
    config: Config,
    kb: KB,
    terminal: Terminal<B>,
    ui: LogosUI<'a>,
    #[cfg(feature = "ollama")]
    tx: mpsc::Sender<Result<ChatMessageResponse>>,
    #[cfg(feature = "ollama")]
    rx: mpsc::Receiver<Result<ChatMessageResponse>>,
    #[cfg(feature = "ollama")]
    history: Arc<Mutex<Vec<ChatMessage>>>,
    registry: commands::CommandRegistry,
}

impl<'a, B: Backend + Write> App<'a, B>
where
    <B as Backend>::Error: 'static + Sync + Send,
{
    fn new(config: Config, kb: KB, terminal: Terminal<B>) -> Self {
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
            registry: commands::get_default_registry(),
        }
    }

    fn update_kb_view(&mut self) -> Result<()> {
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

    async fn run(&mut self) -> Result<()> {
        loop {
            self.update_kb_view()?;
            self.terminal.draw(|f| self.ui.render(f))?;

            // TODO: Handle AI requests to continue/suspend?
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

    async fn enter(&mut self) -> Result<()> {
        enable_raw_mode().unwrap();
        execute!(
            self.terminal.backend_mut(),
            EnterAlternateScreen,
            EnableMouseCapture
        )?;
        self.terminal.clear()?;
        Ok(())
    }
    async fn leave(&mut self) -> Result<()> {
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
    async fn handle_messages(&mut self) -> Result<InputSignal> {
        while let Ok(resp) = self.rx.try_recv() {
            match resp {
                Ok(ChatMessageResponse { done: true, .. }) => {}
                Ok(ChatMessageResponse {
                    message: ChatMessage { content: chunk, .. },
                    done: false,
                    ..
                }) => {
                    if let Some(last_msg) = self.ui.messages.last_mut() {
                        if last_msg.starts_with("Ollama: ") {
                            last_msg.push_str(&chunk);
                        } else {
                            self.ui.messages.push(format!("Ollama: {}", chunk));
                        }
                    } else {
                        self.ui.messages.push(format!("Ollama: {}", chunk));
                    }
                }
                Err(e) => {
                    self.ui.messages.push(format!("Error from Ollama: {}", e));
                }
            }
        }
        Ok(InputSignal::Continue)
    }

    async fn handle_input(&mut self) -> Result<InputSignal> {
        if !event::poll(std::time::Duration::from_millis(10))? {
            return Ok(InputSignal::Continue);
        }

        let event = event::read()?;
        match event {
            Event::Mouse(mouse_event) => {
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
            Event::Key(key) => {
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
                                }
                            }
                        },
                        KeyCode::Tab => {
                            self.ui.focus = UIFocus::Chat;
                        }
                        _ => {
                            self.ui.input_textarea.input(event);
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

    fn submit_input(&mut self) -> Result<()> {
        let user_input = self.ui.input_textarea.lines().join("\n");
        if user_input.trim().is_empty() {
            return Ok(());
        }
        if user_input.starts_with(commands::COMMAND_PREFIX) {
            let user_input = user_input
                .strip_prefix(commands::COMMAND_PREFIX)
                .expect("Check prefix should remove without error");
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
            context.execute(
                name,
                args,
            )?;
            return Ok(());
        }
        self.send_message(user_input)
    }

    fn send_message(&mut self, user_input: String) -> Result<()> {
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
                let ollama = Ollama::default();
                let options = ModelOptions::default()
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
                let mut stream: ChatMessageResponseStream = match stream {
                    Ok(s) => s,
                    Err(e) => {
                        let _ = tx.send(Err(anyhow!("{}", e)));
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

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::load()?;
    println!("Loaded config: {:?}", config);

    let kb = KB::new(PathBuf::from("logos.db"))?;
    let tuple = Tuple::new("Alice", "knows", "Bob", 0.9);
    kb.store_tuple(&tuple)?;
    let tuple = Tuple::new("Bob", "knows", "Eve", 0.1);
    kb.store_tuple(&tuple)?;

    let terminal = Terminal::new(CrosstermBackend::new(std::io::stdout()))?;
    let mut app = App::new(config, kb, terminal);

    app.enter().await?;

    app.run().await?;

    app.leave().await?;

    Ok(())
}
