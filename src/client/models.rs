use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct MediaSource {
    #[serde(default)]
    pub container: Option<String>,
    #[serde(default)]
    pub bitrate: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AuthResponse {
    pub access_token: String,
    pub user: AuthUser,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AuthUser {
    pub id: String,
    #[allow(dead_code)]
    pub name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ItemsResponse {
    pub items: Vec<Item>,
    #[allow(dead_code)]
    pub total_record_count: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Item {
    pub id: String,
    pub name: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub sort_name: Option<String>,
    #[serde(rename = "Type")]
    pub item_type: Option<String>,
    #[serde(default)]
    pub album: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    pub album_id: Option<String>,
    #[serde(default)]
    pub album_artist: Option<String>,
    #[serde(default)]
    pub artists: Option<Vec<String>>,
    #[serde(default)]
    pub index_number: Option<u32>,
    #[serde(default)]
    #[allow(dead_code)]
    pub parent_index_number: Option<u32>,
    #[serde(default)]
    pub run_time_ticks: Option<u64>,
    #[serde(default)]
    pub collection_type: Option<String>,
    #[serde(default)]
    pub production_year: Option<u32>,
    #[serde(default)]
    pub playlist_item_id: Option<String>,
    #[serde(default)]
    pub genres: Option<Vec<String>>,
    #[serde(default)]
    pub date_created: Option<String>,
    #[serde(default)]
    pub media_sources: Option<Vec<MediaSource>>,
}

impl Item {
    pub fn duration_secs(&self) -> f64 {
        self.run_time_ticks
            .map(|t| t as f64 / 10_000_000.0)
            .unwrap_or(0.0)
    }

    pub fn duration_display(&self) -> String {
        format_duration(self.duration_secs())
    }

    pub fn codec_display(&self) -> Option<String> {
        let source = self.media_sources.as_ref()?.first()?;
        let container = source.container.as_ref()?;
        let bitrate = source.bitrate?;
        Some(format!("{} @ {} kbps", container.to_uppercase(), bitrate / 1000))
    }

    pub fn artist_display(&self) -> String {
        self.artists
            .as_ref()
            .and_then(|a| a.first())
            .or(self.album_artist.as_ref())
            .cloned()
            .unwrap_or_default()
    }
}

pub fn format_duration(secs: f64) -> String {
    let total = secs as u64;
    let m = total / 60;
    let s = total % 60;
    format!("{m}:{s:02}")
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct CreatePlaylistRequest {
    pub name: String,
    pub user_id: String,
    pub media_type: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PlaylistCreationResult {
    #[allow(dead_code)]
    pub id: String,
}
