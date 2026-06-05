use serde::Deserialize;

/// Universal item representation used throughout the app.
///
/// Originally modelled after Jellyfin's item shape. Now populated from
/// Subsonic API responses (see `subsonic` module below) via `From` impls.
#[derive(Debug, Clone, Default)]
pub struct Item {
    pub id: String,
    pub name: String,
    /// One of: "Audio", "MusicAlbum", "MusicArtist", "Playlist"
    pub item_type: Option<String>,
    pub album: Option<String>,
    #[allow(dead_code)]
    pub album_id: Option<String>,
    pub album_artist: Option<String>,
    pub artists: Option<Vec<String>>,
    pub index_number: Option<u32>,
    /// Duration in seconds.
    pub duration_seconds: Option<f64>,
    #[allow(dead_code)]
    pub collection_type: Option<String>,
    pub production_year: Option<u32>,
    /// For playlist tracks: the index of the song within the playlist (used
    /// for removal via Subsonic's `songIndexToRemove`).
    pub playlist_item_id: Option<String>,
    pub genres: Option<Vec<String>>,
    pub date_created: Option<String>,
    /// File suffix/container (e.g. "flac", "mp3"), if known.
    pub container: Option<String>,
    /// Bitrate in kbps, if known.
    pub bitrate_kbps: Option<u64>,
    /// True if the user has starred (favorited) this item.
    pub starred: bool,
    /// User rating 0..=5 (0 = unrated).
    pub user_rating: u8,
}

impl Item {
    pub fn duration_secs(&self) -> f64 {
        self.duration_seconds.unwrap_or(0.0)
    }

    pub fn duration_display(&self) -> String {
        format_duration(self.duration_secs())
    }

    pub fn codec_display(&self) -> Option<String> {
        let container = self.container.as_ref()?;
        let bitrate = self.bitrate_kbps?;
        Some(format!("{} @ {} kbps", container.to_uppercase(), bitrate))
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

// =====================================================================
// Subsonic API response types
// =====================================================================
//
// The Subsonic API wraps everything in `{"subsonic-response": {...}}`.
// We accept either a success payload or an error.

#[derive(Debug, Deserialize)]
pub struct SubsonicEnvelope<T> {
    #[serde(rename = "subsonic-response")]
    pub response: SubsonicResponse<T>,
}

#[derive(Debug, Deserialize)]
pub struct SubsonicResponse<T> {
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub error: Option<SubsonicError>,
    #[serde(flatten)]
    pub data: T,
}

#[derive(Debug, Deserialize)]
pub struct SubsonicError {
    #[serde(default)]
    pub code: i32,
    #[serde(default)]
    pub message: String,
}

// ----- Song / Album / Artist -----

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Song {
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub album: Option<String>,
    #[serde(default)]
    pub album_id: Option<String>,
    #[serde(default)]
    pub artist: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    pub artist_id: Option<String>,
    #[serde(default)]
    pub track: Option<u32>,
    #[serde(default)]
    pub year: Option<u32>,
    #[serde(default)]
    pub genre: Option<String>,
    #[serde(default)]
    pub duration: Option<u64>,
    #[serde(default)]
    pub bit_rate: Option<u64>,
    #[serde(default)]
    pub suffix: Option<String>,
    #[serde(default)]
    pub created: Option<String>,
    #[serde(default)]
    pub disc_number: Option<u32>,
    /// Starred timestamp (ISO-8601). Presence = favorited.
    #[serde(default)]
    pub starred: Option<String>,
    #[serde(default)]
    pub user_rating: Option<u8>,
}

impl Song {
    pub fn into_item(self) -> Item {
        let genres = self.genre.as_ref().map(|g| vec![g.clone()]);
        Item {
            id: self.id,
            name: self.title,
            item_type: Some("Audio".to_string()),
            album: self.album,
            album_id: self.album_id,
            album_artist: self.artist.clone(),
            artists: self.artist.map(|a| vec![a]),
            index_number: self.track,
            duration_seconds: self.duration.map(|d| d as f64),
            collection_type: None,
            production_year: self.year,
            playlist_item_id: None,
            genres,
            date_created: self.created,
            container: self.suffix,
            bitrate_kbps: self.bit_rate,
            starred: self.starred.is_some(),
            user_rating: self.user_rating.unwrap_or(0).min(5),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Album {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub artist: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    pub artist_id: Option<String>,
    #[serde(default)]
    pub year: Option<u32>,
    #[serde(default)]
    pub created: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    pub song_count: Option<u32>,
    #[serde(default)]
    pub duration: Option<u64>,
    #[serde(default, rename = "song")]
    pub songs: Option<Vec<Song>>,
    #[serde(default)]
    pub starred: Option<String>,
    #[serde(default)]
    pub user_rating: Option<u8>,
}

impl Album {
    pub fn into_item(self) -> Item {
        Item {
            id: self.id,
            name: self.name,
            item_type: Some("MusicAlbum".to_string()),
            album: None,
            album_id: None,
            album_artist: self.artist.clone(),
            artists: self.artist.map(|a| vec![a]),
            index_number: None,
            duration_seconds: self.duration.map(|d| d as f64),
            collection_type: None,
            production_year: self.year,
            playlist_item_id: None,
            genres: None,
            date_created: self.created,
            container: None,
            bitrate_kbps: None,
            starred: self.starred.is_some(),
            user_rating: self.user_rating.unwrap_or(0).min(5),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Artist {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub album_count: Option<u32>,
    #[serde(default, rename = "album")]
    pub albums: Option<Vec<Album>>,
}

impl Artist {
    pub fn into_item(self) -> Item {
        Item {
            id: self.id,
            name: self.name,
            item_type: Some("MusicArtist".to_string()),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistMeta {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub song_count: Option<u32>,
    #[serde(default)]
    pub duration: Option<u64>,
    #[serde(default)]
    pub created: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    pub changed: Option<String>,
    #[serde(default, rename = "entry")]
    pub entries: Option<Vec<Song>>,
}

impl PlaylistMeta {
    pub fn into_item(self) -> Item {
        Item {
            id: self.id,
            name: self.name,
            item_type: Some("Playlist".to_string()),
            duration_seconds: self.duration.map(|d| d as f64),
            date_created: self.created,
            ..Default::default()
        }
    }
}

// ----- Response payloads -----

#[derive(Debug, Deserialize, Default)]
pub struct ArtistsPayload {
    #[serde(default)]
    pub artists: ArtistsIndex,
}

#[derive(Debug, Deserialize, Default)]
pub struct ArtistsIndex {
    #[serde(default)]
    pub index: Vec<ArtistIndexEntry>,
}

#[derive(Debug, Deserialize, Default)]
pub struct ArtistIndexEntry {
    #[serde(default)]
    #[allow(dead_code)]
    pub name: String,
    #[serde(default)]
    pub artist: Vec<Artist>,
}

#[derive(Debug, Deserialize, Default)]
pub struct ArtistPayload {
    #[serde(default)]
    pub artist: Artist,
}

#[derive(Debug, Deserialize, Default)]
pub struct AlbumPayload {
    #[serde(default)]
    pub album: Album,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AlbumListPayload {
    #[serde(default)]
    pub album_list2: AlbumListInner,
}

#[derive(Debug, Deserialize, Default)]
pub struct AlbumListInner {
    #[serde(default)]
    pub album: Vec<Album>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SearchPayload {
    #[serde(default)]
    pub search_result3: SearchInner,
}

#[derive(Debug, Deserialize, Default)]
pub struct SearchInner {
    #[serde(default)]
    pub artist: Vec<Artist>,
    #[serde(default)]
    pub album: Vec<Album>,
    #[serde(default)]
    pub song: Vec<Song>,
}

#[derive(Debug, Deserialize, Default)]
pub struct PlaylistsPayload {
    #[serde(default)]
    pub playlists: PlaylistsInner,
}

#[derive(Debug, Deserialize, Default)]
pub struct PlaylistsInner {
    #[serde(default)]
    pub playlist: Vec<PlaylistMeta>,
}

#[derive(Debug, Deserialize, Default)]
pub struct PlaylistPayload {
    #[serde(default)]
    pub playlist: PlaylistMeta,
}

#[derive(Debug, Deserialize, Default)]
pub struct SongPayload {
    #[serde(default)]
    pub song: Song,
}

#[derive(Debug, Deserialize, Default)]
pub struct EmptyPayload {}
