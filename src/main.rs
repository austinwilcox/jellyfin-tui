mod app;
mod client;
mod config;
mod player;
mod tui;
mod ui;

use anyhow::Result;

use crate::client::JellyfinClient;
use crate::config::{config_path, load_config, prompt_config, select_server, get_server_config, save_multi_config};

#[tokio::main]
async fn main() -> Result<()> {
    // Handle subcommands
    if let Some(cmd) = std::env::args().nth(1) {
        match cmd.as_str() {
            "config" => {
                let path = config_path()?;
                let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
                let status = std::process::Command::new(&editor)
                    .arg(&path)
                    .status()
                    .map_err(|e| anyhow::anyhow!("Failed to open editor '{}': {}", editor, e))?;
                std::process::exit(status.code().unwrap_or(1));
            }
            _ => {
                eprintln!("Unknown command: {cmd}");
                eprintln!("Usage: jellyfin-tui [config]");
                std::process::exit(1);
            }
        }
    }

    // Load or create config
    let (mut multi, server_name) = match load_config()? {
        Some(multi) => {
            let name = select_server(&multi)?;
            (multi, name)
        }
        None => {
            println!("Welcome to jellyfin-tui! Let's set up your connection.");
            println!();
            prompt_config()?
        }
    };

    let mut cfg = get_server_config(&multi, &server_name)
        .unwrap_or_else(|| bail_missing_server(&server_name));

    // Authenticate
    println!("Connecting to {}...", cfg.server_url);
    client::auth::authenticate(&mut cfg, &server_name).await?;
    println!("Authenticated as {}", cfg.username);

    // Update active server
    multi.active = server_name.clone();
    save_multi_config(&multi)?;

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

fn bail_missing_server(name: &str) -> ! {
    eprintln!("Server '{}' not found in config", name);
    std::process::exit(1);
}
