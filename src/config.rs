use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub server_url: String,
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub token: Option<String>,
    #[serde(default)]
    pub user_id: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server_url: String::new(),
            username: String::new(),
            password: String::new(),
            token: None,
            user_id: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerEntry {
    pub name: String,
    pub server_url: String,
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub token: Option<String>,
    #[serde(default)]
    pub user_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MultiServerConfig {
    pub active: String,
    pub servers: Vec<ServerEntry>,
}

impl From<&ServerEntry> for Config {
    fn from(entry: &ServerEntry) -> Self {
        Config {
            server_url: entry.server_url.clone(),
            username: entry.username.clone(),
            password: entry.password.clone(),
            token: entry.token.clone(),
            user_id: entry.user_id.clone(),
        }
    }
}

pub fn config_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .context("Could not determine config directory")?
        .join("navidrome-tui");
    fs::create_dir_all(&config_dir)?;
    Ok(config_dir.join("config.toml"))
}

pub fn load_config() -> Result<Option<MultiServerConfig>> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let contents = fs::read_to_string(&path)?;

    // Try new multi-server format first
    if let Ok(multi) = toml::from_str::<MultiServerConfig>(&contents) {
        if !multi.servers.is_empty() {
            return Ok(Some(multi));
        }
    }

    // Fall back to old flat format and auto-migrate
    let old: Config = toml::from_str(&contents)?;
    let entry = ServerEntry {
        name: "default".to_string(),
        server_url: old.server_url,
        username: old.username,
        password: old.password,
        token: old.token,
        user_id: old.user_id,
    };
    let multi = MultiServerConfig {
        active: "default".to_string(),
        servers: vec![entry],
    };
    save_multi_config(&multi)?;
    Ok(Some(multi))
}

pub fn save_multi_config(multi: &MultiServerConfig) -> Result<()> {
    let path = config_path()?;
    let contents = toml::to_string_pretty(multi)?;
    fs::write(&path, contents)?;
    Ok(())
}

pub fn save_server_config(server_name: &str, config: &Config) -> Result<()> {
    let path = config_path()?;
    let contents = fs::read_to_string(&path)?;
    let mut multi: MultiServerConfig = toml::from_str(&contents)?;

    if let Some(entry) = multi.servers.iter_mut().find(|s| s.name == server_name) {
        entry.token = config.token.clone();
        entry.user_id = config.user_id.clone();
    }

    save_multi_config(&multi)?;
    Ok(())
}

pub fn select_server(multi: &MultiServerConfig) -> Result<String> {
    if multi.servers.len() == 1 {
        return Ok(multi.servers[0].name.clone());
    }

    println!("Available servers:");
    for (i, server) in multi.servers.iter().enumerate() {
        let marker = if server.name == multi.active { " (active)" } else { "" };
        println!("  [{}] {} — {}{}", i + 1, server.name, server.url_display(), marker);
    }
    println!();
    print!("Select server [Enter for '{}']: ", multi.active);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();

    if input.is_empty() {
        return Ok(multi.active.clone());
    }

    // Try parsing as a number
    if let Ok(n) = input.parse::<usize>() {
        if n >= 1 && n <= multi.servers.len() {
            return Ok(multi.servers[n - 1].name.clone());
        }
    }

    // Try matching by name
    if let Some(entry) = multi.servers.iter().find(|s| s.name == input) {
        return Ok(entry.name.clone());
    }

    // Default to active
    Ok(multi.active.clone())
}

pub fn get_server_config(multi: &MultiServerConfig, name: &str) -> Option<Config> {
    multi.servers.iter().find(|s| s.name == name).map(Config::from)
}

pub fn prompt_config() -> Result<(MultiServerConfig, String)> {
    let mut input = String::new();

    print!("Server name (default: \"default\"): ");
    io::stdout().flush()?;
    io::stdin().read_line(&mut input)?;
    let name = {
        let trimmed = input.trim();
        if trimmed.is_empty() { "default".to_string() } else { trimmed.to_string() }
    };

    print!("Navidrome server URL (e.g. http://localhost:4533): ");
    io::stdout().flush()?;
    input.clear();
    io::stdin().read_line(&mut input)?;
    let server_url = input.trim().trim_end_matches('/').to_string();

    print!("Username: ");
    io::stdout().flush()?;
    input.clear();
    io::stdin().read_line(&mut input)?;
    let username = input.trim().to_string();

    print!("Password: ");
    io::stdout().flush()?;
    input.clear();
    io::stdin().read_line(&mut input)?;
    let password = input.trim().to_string();

    let entry = ServerEntry {
        name: name.clone(),
        server_url,
        username,
        password,
        token: None,
        user_id: None,
    };

    let multi = MultiServerConfig {
        active: name.clone(),
        servers: vec![entry],
    };

    save_multi_config(&multi)?;
    println!("Config saved to ~/.config/navidrome-tui/config.toml");

    Ok((multi, name))
}

impl ServerEntry {
    fn url_display(&self) -> &str {
        &self.server_url
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlaybackState {
    pub item_id: String,
    pub position_secs: f64,
}

pub fn state_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .context("Could not determine config directory")?
        .join("navidrome-tui");
    fs::create_dir_all(&config_dir)?;
    Ok(config_dir.join("state.toml"))
}

pub fn save_playback_state(state: &PlaybackState) -> Result<()> {
    let path = state_path()?;
    let contents = toml::to_string_pretty(state)?;
    fs::write(&path, contents)?;
    Ok(())
}

pub fn load_playback_state() -> Option<PlaybackState> {
    let path = state_path().ok()?;
    if !path.exists() {
        return None;
    }
    let contents = fs::read_to_string(&path).ok()?;
    toml::from_str(&contents).ok()
}
