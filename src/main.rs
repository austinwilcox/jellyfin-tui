mod app;
mod client;
mod config;
mod player;
mod tui;
mod ui;

use anyhow::Result;

use crate::client::JellyfinClient;
use crate::config::{load_config, prompt_config};

#[tokio::main]
async fn main() -> Result<()> {
    // Load or create config
    let mut cfg = match load_config()? {
        Some(cfg) => cfg,
        None => {
            println!("Welcome to jellyfin-tui! Let's set up your connection.");
            println!();
            prompt_config()?
        }
    };

    // Authenticate
    println!("Connecting to {}...", cfg.server_url);
    client::auth::authenticate(&mut cfg).await?;
    println!("Authenticated as {}", cfg.username);

    // Create API client
    let api_client = JellyfinClient::new(&cfg)?;

    // Initialize TUI
    let mut terminal = tui::init()?;

    // Run app
    let mut app = app::App::new(api_client);
    let result = app.run(&mut terminal).await;

    // Restore terminal
    tui::restore()?;

    if let Err(e) = result {
        eprintln!("Error: {e}");
    }

    Ok(())
}
