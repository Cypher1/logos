mod config;
mod kb;
mod ui;

use crate::config::Config;
use crate::kb::{Tuple, KB};
use crate::ui::{LogosUI, UIFocus};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures_util::StreamExt;
use ollama_rs::generation::completion::request::GenerationRequest;
use ollama_rs::Ollama;
use ratatui::{backend::CrosstermBackend, Terminal};
use signal_hook::consts::signal::*;
use std::error::Error;
use std::io::stdout;
use std::sync::mpsc;
use std::thread;
use tokio::runtime::Runtime;

enum OllamaMessage {
    Chunk(String),
    Final(String),
}

fn main() -> Result<(), Box<dyn Error>> {
    // Load configuration
    let config = Config::load()?;
    println!("Loaded config: {:?}", config);

    // Initialize the knowledge base
    let kb = KB::new("logos.db")?;
    // Insert a test tuple
    let tuple = Tuple::new("Alice", "knows", "Bob", 0.9);
    kb.store_tuple(&tuple)?;

    // Set up the terminal
    let mut stdout = stdout();
    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create the UI state
    let mut ui = LogosUI::new();

    // Channel for receiving Ollama responses
    let (tx, rx) = mpsc::channel::<Result<OllamaMessage, Box<dyn Error + Send + Sync>>>();

    // Initial load of knowledge base into UI
    update_kb_view(&mut ui, &kb)?;

    // Main loop
    loop {
        // Update knowledge base view from KB (in case it changed)
        update_kb_view(&mut ui, &kb)?;

        // Draw the UI
        terminal.draw(|f| ui.render(f))?;

        // Check for Ollama responses (non-blocking)
        while let Ok(msg) = rx.try_recv() {
            match msg {
                Ok(OllamaMessage::Chunk(chunk)) => {
                    if let Some(last_msg) = ui.messages.last_mut() {
                        if last_msg.starts_with("Ollama: ") {
                            last_msg.push_str(&chunk);
                        } else {
                            ui.messages.push(format!("Ollama: {}", chunk));
                        }
                    } else {
                        ui.messages.push(format!("Ollama: {}", chunk));
                    }
                }
                Ok(OllamaMessage::Final(response)) => {
                    if let Some(last_msg) = ui.messages.last_mut() {
                        if last_msg.starts_with("Ollama: ") {
                            *last_msg = format!("Ollama: {}", response);
                        } else {
                            ui.messages.push(format!("Ollama: {}", response));
                        }
                    } else {
                        ui.messages.push(format!("Ollama: {}", response));
                    }
                }
                Err(e) => {
                    ui.messages.push(format!("Error from Ollama: {}", e));
                }
            }
        }

        // Handle input (both keyboard and mouse)
        match event::read()? {
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
                        // Click to focus
                        if contains(ui.chat_area) {
                            ui.focus = UIFocus::Chat;
                        } else if contains(ui.kb_area) {
                            ui.focus = UIFocus::KB;
                        } else if contains(ui.planning_area) {
                            ui.focus = UIFocus::Planning;
                        } else if contains(ui.input_area) {
                            ui.focus = UIFocus::Input;
                        }
                    }
                    event::MouseEventKind::ScrollUp => {
                        // Scroll up on whichever panel mouse is over
                        if contains(ui.chat_area) {
                            ui.chat_scroll = ui.chat_scroll.saturating_sub(1);
                        } else if contains(ui.kb_area) {
                            ui.kb_scroll = ui.kb_scroll.saturating_sub(1);
                        } else if contains(ui.planning_area) {
                            ui.planning_scroll = ui.planning_scroll.saturating_sub(1);
                        }
                        ui.cursor_pos = ui.input.len();
                    }
                    event::MouseEventKind::ScrollDown => {
                        // Scroll down on whichever panel mouse is over
                        if contains(ui.chat_area) {
                            ui.chat_scroll = ui.chat_scroll.saturating_add(1);
                        } else if contains(ui.kb_area) {
                            ui.kb_scroll = ui.kb_scroll.saturating_add(1);
                        } else if contains(ui.planning_area) {
                            ui.planning_scroll = ui.planning_scroll.saturating_add(1);
                        }
                    }
                    _ => {}
                }
            }
            Event::Key(key) => {
                if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('z') {
                    // Suspend
                    disable_raw_mode().unwrap();
                    execute!(
                        terminal.backend_mut(),
                        LeaveAlternateScreen,
                        DisableMouseCapture
                    )?;
                    signal_hook::low_level::emulate_default_handler(SIGTSTP).unwrap();

                    // Resume
                    enable_raw_mode().unwrap();
                    execute!(
                        terminal.backend_mut(),
                        EnterAlternateScreen,
                        EnableMouseCapture
                    )?;
                    terminal.clear()?;
                    continue;
                }
                match key.code {
                    // Global keys
                    KeyCode::Esc | KeyCode::Char('\u{4}') => break,
                    KeyCode::Tab => {
                        ui.focus = match ui.focus {
                            UIFocus::Input => UIFocus::Chat,
                            UIFocus::Chat => UIFocus::KB,
                            UIFocus::KB => UIFocus::Planning,
                            UIFocus::Planning => UIFocus::Input,
                        };
                    }
                    KeyCode::BackTab => {
                        ui.focus = match ui.focus {
                            UIFocus::Input => UIFocus::Planning,
                            UIFocus::Chat => UIFocus::Input,
                            UIFocus::KB => UIFocus::Chat,
                            UIFocus::Planning => UIFocus::KB,
                        };
                    }
                    KeyCode::PageUp => match ui.focus {
                        UIFocus::Input | UIFocus::Chat => {
                            ui.chat_scroll = ui.chat_scroll.saturating_sub(5);
                        }
                        UIFocus::KB => {
                            ui.kb_scroll = ui.kb_scroll.saturating_sub(5);
                        }
                        UIFocus::Planning => {
                            ui.planning_scroll = ui.planning_scroll.saturating_sub(5);
                        }
                    },
                    KeyCode::PageDown => match ui.focus {
                        UIFocus::Input | UIFocus::Chat => {
                            ui.chat_scroll = ui.chat_scroll.saturating_add(5);
                        }
                        UIFocus::KB => {
                            ui.kb_scroll = ui.kb_scroll.saturating_add(5);
                        }
                        UIFocus::Planning => {
                            ui.planning_scroll = ui.planning_scroll.saturating_add(5);
                        }
                    },
                    // Focus-dependent keys
                    _ => match ui.focus {
                        UIFocus::Input => match key.code {
                            KeyCode::Char(c) => {
                                ui.input.insert(ui.cursor_pos, c);
                                ui.cursor_pos += 1;
                            }
                            KeyCode::Backspace
                                if ui.cursor_pos > 0 => {
                                    ui.input.remove(ui.cursor_pos - 1);
                                    ui.cursor_pos -= 1;
                                }
                            KeyCode::Left
                                if ui.cursor_pos > 0 => {
                                    ui.cursor_pos -= 1;
                                }
                            KeyCode::Right
                                if ui.cursor_pos < ui.input.len() => {
                                    ui.cursor_pos += 1;
                                }
                            KeyCode::Up
                                // Navigate up in history
                                if !ui.input_history.is_empty() => {
                                    if ui.history_pos == 0 {
                                        ui.input_history.insert(0, ui.input.clone());
                                    }
                                    if ui.history_pos < ui.input_history.len() {
                                        ui.history_pos += 1;
                                        ui.input = ui.input_history[ui.history_pos - 1].clone();
                                        ui.cursor_pos = ui.input.len();
                                    }
                                }
                            KeyCode::Down
                                // Navigate down in history
                                if ui.history_pos > 0 => {
                                    ui.history_pos -= 1;
                                    if ui.history_pos == 0 {
                                        if !ui.input_history.is_empty() {
                                            ui.input = ui.input_history.remove(0);
                                        } else {
                                            ui.input = String::new();
                                        }
                                    } else {
                                        ui.input = ui.input_history[ui.history_pos - 1].clone();
                                    }
                                    ui.cursor_pos = ui.input.len();
                                }
                            KeyCode::Enter
                                if !ui.input_textarea.lines().is_empty() => {
                                    let user_input = ui.input_textarea.lines().join("\n");
                                    ui.messages.push(format!("You: {}", user_input));
                                    if ui.input_history.is_empty()
                                        || ui.input_history[0] != user_input
                                    {
                                        ui.input_history.insert(0, user_input.clone());
                                    }
                                    ui.history_pos = 0;
                                    ui.input_textarea.clear();
                                    ui.cursor_pos = 0;
                                    let tx = tx.clone();
                                    let model = config.model.clone();
                                    thread::spawn(move || {
                                        let rt = match Runtime::new() {
                                            Ok(rt) => rt,
                                            Err(e) => {
                                                let _ = tx.send(Err(Box::new(e)));
                                                return;
                                            }
                                        };

                                        rt.block_on(async move {
                                            let ollama = Ollama::default();
                                            let mut stream = match ollama
                                                .generate_stream(GenerationRequest::new(
                                                    model,
                                                    user_input,
                                                ))
                                                .await
                                            {
                                                Ok(s) => s,
                                                Err(e) => {
                                                    let _ = tx.send(Err(Box::new(e)));
                                                    return;
                                                }
                                            };

                                            let mut full_response = String::new();
                                            while let Some(result) = stream.next().await {
                                                match result {
                                                    Ok(responses) => {
                                                        let mut chunk = String::new();
                                                        for response in responses {
                                                            chunk.push_str(&response.response);
                                                        }
                                                        full_response.push_str(&chunk);
                                                        let _ = tx
                                                            .send(Ok(OllamaMessage::Chunk(chunk)));
                                                    }
                                                    Err(e) => {
                                                        let _ = tx.send(Err(Box::new(e)));
                                                        return;
                                                    }
                                                }
                                            }
                                            let _ =
                                                tx.send(Ok(OllamaMessage::Final(full_response)));
                                        });
                                    });
                                }
                                _ => {},
                        },
                        UIFocus::Chat => match key.code {
                            KeyCode::Up => {
                                ui.chat_scroll = ui.chat_scroll.saturating_sub(1);
                            }
                            KeyCode::Down => {
                                ui.chat_scroll = ui.chat_scroll.saturating_add(1);
                            }
                            _ => {}
                        },
                        UIFocus::KB => match key.code {
                            KeyCode::Up => {
                                ui.kb_scroll = ui.kb_scroll.saturating_sub(1);
                            }
                            KeyCode::Down => {
                                ui.kb_scroll = ui.kb_scroll.saturating_add(1);
                            }
                            KeyCode::Char('/') | KeyCode::Char('r') => {
                                // Manual refresh key for KB
                                update_kb_view(&mut ui, &kb)?;
                            }
                            _ => {}
                        },
                        UIFocus::Planning => match key.code {
                            KeyCode::Up => {
                                ui.planning_scroll = ui.planning_scroll.saturating_sub(1);
                            }
                            KeyCode::Down => {
                                ui.planning_scroll = ui.planning_scroll.saturating_add(1);
                            }
                            _ => {}
                        },
                    },
                }
            }
            _ => {}
        }
    }

    // Restore the terminal
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    disable_raw_mode()?;
    terminal.show_cursor()?;

    Ok(())
}

/// Updates the UI's knowledge base view from the KB
fn update_kb_view(ui: &mut LogosUI, kb: &KB) -> Result<(), Box<dyn Error>> {
    // Clear the current knowledge base display
    ui.knowledge_base.clear();
    let mut count = 0;
    let tuples = kb.get_all_tuples()?;
    for tuple in tuples {
        ui.knowledge_base.push(tuple.to_string());
        count += 1;
    }
    ui.system_state = format!("Loaded {} tuples from KB", count);
    Ok(())
}
