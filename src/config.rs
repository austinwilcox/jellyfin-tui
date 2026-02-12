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

fn config_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .context("Could not determine config directory")?
        .join("jellyfin-tui");
    fs::create_dir_all(&config_dir)?;
    Ok(config_dir.join("config.toml"))
}

pub fn load_config() -> Result<Option<Config>> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let contents = fs::read_to_string(&path)?;
    let config: Config = toml::from_str(&contents)?;
    Ok(Some(config))
}

pub fn save_config(config: &Config) -> Result<()> {
    let path = config_path()?;
    let contents = toml::to_string_pretty(config)?;
    fs::write(&path, contents)?;
    Ok(())
}

pub fn prompt_config() -> Result<Config> {
    let mut config = Config::default();

    print!("Jellyfin server URL (e.g. http://localhost:8096): ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    config.server_url = input.trim().trim_end_matches('/').to_string();

    print!("Username: ");
    io::stdout().flush()?;
    input.clear();
    io::stdin().read_line(&mut input)?;
    config.username = input.trim().to_string();

    print!("Password: ");
    io::stdout().flush()?;
    input.clear();
    io::stdin().read_line(&mut input)?;
    config.password = input.trim().to_string();

    save_config(&config)?;
    println!("Config saved to ~/.config/jellyfin-tui/config.toml");

    Ok(config)
}
