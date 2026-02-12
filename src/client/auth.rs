use anyhow::{bail, Context, Result};
use reqwest::Client;

use crate::client::models::AuthResponse;
use crate::config::{save_config, Config};

const DEVICE_NAME: &str = "jellyfin-tui";
const DEVICE_ID: &str = "jellyfin-tui-rust";
const CLIENT_NAME: &str = "jellyfin-tui";
const CLIENT_VERSION: &str = "0.1.0";

pub fn media_browser_header(token: Option<&str>) -> String {
    let mut header = format!(
        "MediaBrowser Client=\"{CLIENT_NAME}\", Device=\"{DEVICE_NAME}\", DeviceId=\"{DEVICE_ID}\", Version=\"{CLIENT_VERSION}\""
    );
    if let Some(t) = token {
        header.push_str(&format!(", Token=\"{t}\""));
    }
    header
}

pub async fn authenticate(config: &mut Config) -> Result<()> {
    // If we already have a valid token, try it first
    if let (Some(ref token), Some(ref user_id)) = (&config.token, &config.user_id) {
        let client = Client::new();
        let resp = client
            .get(format!("{}/Users/{user_id}", config.server_url))
            .header("Authorization", media_browser_header(Some(token)))
            .send()
            .await;

        if let Ok(r) = resp {
            if r.status().is_success() {
                return Ok(());
            }
        }
    }

    // Authenticate with username/password
    let client = Client::new();
    let body = serde_json::json!({
        "Username": config.username,
        "Pw": config.password
    });

    let resp = client
        .post(format!("{}/Users/AuthenticateByName", config.server_url))
        .header("Authorization", media_browser_header(None))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .context("Failed to connect to Jellyfin server")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        bail!("Authentication failed (HTTP {status}): {text}");
    }

    let auth: AuthResponse = resp.json().await.context("Failed to parse auth response")?;
    config.token = Some(auth.access_token);
    config.user_id = Some(auth.user.id);
    save_config(config)?;

    Ok(())
}
