use anyhow::{bail, Context, Result};
use rand::Rng;
use reqwest::Client;

use crate::client::models::SubsonicEnvelope;
use crate::config::{save_server_config, Config};

pub const CLIENT_NAME: &str = "navidrome-tui";
pub const API_VERSION: &str = "1.16.1";

/// Generate a random hex salt for Subsonic token authentication.
pub fn random_salt() -> String {
    let mut rng = rand::thread_rng();
    let bytes: [u8; 8] = rng.gen();
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Compute the Subsonic auth token = md5(password + salt).
pub fn auth_token(password: &str, salt: &str) -> String {
    let mut input = String::with_capacity(password.len() + salt.len());
    input.push_str(password);
    input.push_str(salt);
    let digest = md5::compute(input.as_bytes());
    format!("{:x}", digest)
}

/// Returns the query-string fragment for Subsonic auth, e.g.
/// `u=alice&t=...&s=...&v=1.16.1&c=navidrome-tui&f=json`
pub fn auth_query(username: &str, password: &str) -> String {
    let salt = random_salt();
    let token = auth_token(password, &salt);
    format!(
        "u={}&t={}&s={}&v={}&c={}&f=json",
        url_encode(username),
        token,
        salt,
        API_VERSION,
        CLIENT_NAME,
    )
}

pub fn url_encode(s: &str) -> String {
    let mut result = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(b as char);
            }
            _ => result.push_str(&format!("%{:02X}", b)),
        }
    }
    result
}

/// Validate credentials by pinging the server. Persists nothing sensitive
/// beyond what was already provided.
pub async fn authenticate(config: &mut Config, server_name: &str) -> Result<()> {
    if config.server_url.is_empty() {
        bail!("Server URL is empty in config");
    }
    if config.username.is_empty() {
        bail!("Username is empty in config");
    }
    if config.password.is_empty() {
        bail!("Password is empty in config (re-run `navidrome-tui config` to set it)");
    }

    let client = Client::new();
    let url = format!(
        "{}/rest/ping.view?{}",
        config.server_url,
        auth_query(&config.username, &config.password)
    );
    let resp = client
        .get(&url)
        .send()
        .await
        .context("Failed to connect to Navidrome server")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        bail!("Authentication failed (HTTP {status}): {text}");
    }

    let text = resp.text().await.context("Failed to read auth response")?;
    let env: SubsonicEnvelope<serde_json::Value> = serde_json::from_str(&text)
        .with_context(|| format!("Failed to parse auth response: {text}"))?;

    if env.response.status != "ok" {
        let msg = env
            .response
            .error
            .map(|e| format!("(code {}) {}", e.code, e.message))
            .unwrap_or_else(|| "unknown error".to_string());
        bail!("Authentication rejected by server: {msg}");
    }

    // Subsonic has no separate token to store; the password is the
    // shared secret. We do however clear out any stale fields.
    config.token = None;
    config.user_id = Some(config.username.clone());
    save_server_config(server_name, config)?;
    Ok(())
}
