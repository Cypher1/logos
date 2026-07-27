mod app;
mod commands;
mod config;
mod kb;
mod ui;

use crate::app::App;
use crate::config::Config;
use crate::kb::{KB, Tuple};
use anyhow::Result;

use ratatui::{Terminal, backend::CrosstermBackend};
use std::path::PathBuf;

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
