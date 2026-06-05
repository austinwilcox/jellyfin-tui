use anyhow::{bail, Context, Result};
use reqwest::Client;
use serde::de::DeserializeOwned;

use crate::client::auth::{auth_query, url_encode};
use crate::client::models::{
    AlbumListPayload, AlbumPayload, ArtistPayload, ArtistsPayload, EmptyPayload, Item,
    PlaylistPayload, PlaylistsPayload, SearchPayload, SongPayload, SubsonicEnvelope,
};
use crate::config::Config;

/// Client for the Subsonic API (Navidrome).
#[derive(Clone)]
pub struct SubsonicClient {
    client: Client,
    pub base_url: String,
    #[allow(dead_code)]
    pub user_id: String,
    username: String,
    password: String,
}

impl SubsonicClient {
    pub fn new(config: &Config) -> Result<Self> {
        if config.server_url.is_empty() {
            bail!("Missing server URL");
        }
        if config.username.is_empty() {
            bail!("Missing username");
        }
        if config.password.is_empty() {
            bail!("Missing password");
        }
        Ok(Self {
            client: Client::new(),
            base_url: config.server_url.clone(),
            user_id: config.user_id.clone().unwrap_or_else(|| config.username.clone()),
            username: config.username.clone(),
            password: config.password.clone(),
        })
    }

    pub async fn re_authenticate(&mut self) -> Result<()> {
        // Subsonic auth is per-request; just verify credentials still work.
        self.ping().await
    }

    /// Build a fully-qualified Subsonic URL with auth params, then append
    /// any extra query parameters.
    fn url(&self, endpoint: &str, extra: &str) -> String {
        let auth = auth_query(&self.username, &self.password);
        if extra.is_empty() {
            format!("{}/rest/{endpoint}?{auth}", self.base_url)
        } else {
            format!("{}/rest/{endpoint}?{auth}&{extra}", self.base_url)
        }
    }

    async fn get_json<T: DeserializeOwned + Default>(&self, endpoint: &str, params: &str) -> Result<T> {
        let url = self.url(endpoint, params);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .with_context(|| format!("Request to {endpoint} failed"))?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            bail!("{endpoint}: HTTP {status}: {body}");
        }
        let env: SubsonicEnvelope<T> = serde_json::from_str(&body)
            .with_context(|| format!("Failed to parse {endpoint} response: {body}"))?;
        if env.response.status != "ok" {
            let msg = env
                .response
                .error
                .map(|e| format!("(code {}) {}", e.code, e.message))
                .unwrap_or_else(|| "unknown error".to_string());
            bail!("{endpoint}: {msg}");
        }
        Ok(env.response.data)
    }

    pub async fn ping(&self) -> Result<()> {
        let _: EmptyPayload = self.get_json("ping.view", "").await?;
        Ok(())
    }

    pub async fn get_artists(&mut self) -> Result<Vec<Item>> {
        let payload: ArtistsPayload = self.get_json("getArtists.view", "").await?;
        let mut items = Vec::new();
        for index in payload.artists.index {
            for a in index.artist {
                items.push(a.into_item());
            }
        }
        Ok(items)
    }

    pub async fn get_artist_albums(&self, artist_id: &str) -> Result<Vec<Item>> {
        let params = format!("id={}", url_encode(artist_id));
        let payload: ArtistPayload = self.get_json("getArtist.view", &params).await?;
        let mut albums: Vec<_> = payload.artist.albums.unwrap_or_default();
        // Sort by year ascending (None last), then name
        albums.sort_by(|a, b| {
            let ay = a.year.unwrap_or(u32::MAX);
            let by = b.year.unwrap_or(u32::MAX);
            ay.cmp(&by).then_with(|| a.name.cmp(&b.name))
        });
        Ok(albums.into_iter().map(|a| a.into_item()).collect())
    }

    pub async fn get_artist_tracks(&self, artist_id: &str) -> Result<Vec<Item>> {
        // Subsonic has no direct "all songs by artist" endpoint, so we fetch
        // each album's tracks. They come back from getArtist sorted by year.
        let params = format!("id={}", url_encode(artist_id));
        let payload: ArtistPayload = self.get_json("getArtist.view", &params).await?;
        let mut albums = payload.artist.albums.unwrap_or_default();
        albums.sort_by(|a, b| {
            let ay = a.year.unwrap_or(u32::MAX);
            let by = b.year.unwrap_or(u32::MAX);
            ay.cmp(&by).then_with(|| a.name.cmp(&b.name))
        });

        let mut all = Vec::new();
        for album in albums {
            let songs = self.get_album_songs(&album.id).await.unwrap_or_default();
            all.extend(songs);
        }
        Ok(all)
    }

    async fn get_album_songs(&self, album_id: &str) -> Result<Vec<Item>> {
        let params = format!("id={}", url_encode(album_id));
        let payload: AlbumPayload = self.get_json("getAlbum.view", &params).await?;
        let mut songs = payload.album.songs.unwrap_or_default();
        songs.sort_by(|a, b| {
            a.disc_number.unwrap_or(1).cmp(&b.disc_number.unwrap_or(1))
                .then_with(|| a.track.unwrap_or(0).cmp(&b.track.unwrap_or(0)))
        });
        Ok(songs.into_iter().map(|s| s.into_item()).collect())
    }

    pub async fn get_album_tracks(&self, album_id: &str) -> Result<Vec<Item>> {
        self.get_album_songs(album_id).await
    }

    pub async fn search(&self, query: &str) -> Result<Vec<Item>> {
        let params = format!(
            "query={}&artistCount=20&albumCount=20&songCount=50",
            url_encode(query)
        );
        let payload: SearchPayload = self.get_json("search3.view", &params).await?;
        let mut items = Vec::new();
        for a in payload.search_result3.artist {
            items.push(a.into_item());
        }
        for a in payload.search_result3.album {
            items.push(a.into_item());
        }
        for s in payload.search_result3.song {
            items.push(s.into_item());
        }
        Ok(items)
    }

    pub async fn get_recent_tracks(&self, limit: u32) -> Result<Vec<Item>> {
        // Subsonic's "newest" lists albums by date added. Expand to songs.
        let params = format!("type=newest&size={}", limit.min(500));
        let payload: AlbumListPayload = self.get_json("getAlbumList2.view", &params).await?;
        let mut all = Vec::new();
        for album in payload.album_list2.album {
            if (all.len() as u32) >= limit {
                break;
            }
            let songs = self.get_album_songs(&album.id).await.unwrap_or_default();
            for s in songs {
                if (all.len() as u32) >= limit {
                    break;
                }
                all.push(s);
            }
        }
        Ok(all)
    }

    pub async fn get_playlists(&self) -> Result<Vec<Item>> {
        let payload: PlaylistsPayload = self.get_json("getPlaylists.view", "").await?;
        let mut items: Vec<Item> = payload
            .playlists
            .playlist
            .into_iter()
            .map(|p| p.into_item())
            .collect();
        items.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        Ok(items)
    }

    pub async fn get_playlist_tracks(&self, playlist_id: &str) -> Result<Vec<Item>> {
        let params = format!("id={}", url_encode(playlist_id));
        let payload: PlaylistPayload = self.get_json("getPlaylist.view", &params).await?;
        let entries = payload.playlist.entries.unwrap_or_default();
        let items = entries
            .into_iter()
            .enumerate()
            .map(|(idx, s)| {
                let mut item = s.into_item();
                // Subsonic removes by index, not by entry ID — store the
                // playlist-track index here.
                item.playlist_item_id = Some(idx.to_string());
                item
            })
            .collect();
        Ok(items)
    }

    pub async fn create_playlist(&self, name: &str) -> Result<Item> {
        let params = format!("name={}", url_encode(name));
        let payload: PlaylistPayload = self.get_json("createPlaylist.view", &params).await?;
        Ok(payload.playlist.into_item())
    }

    pub async fn add_to_playlist(&self, playlist_id: &str, item_ids: &[String]) -> Result<()> {
        // updatePlaylist supports multiple songIdToAdd params.
        let mut params = format!("playlistId={}", url_encode(playlist_id));
        for id in item_ids {
            params.push_str("&songIdToAdd=");
            params.push_str(&url_encode(id));
        }
        let _: EmptyPayload = self.get_json("updatePlaylist.view", &params).await?;
        Ok(())
    }

    /// `entry_ids` here are the playlist-track indices (as strings),
    /// matching what `get_playlist_tracks` stored in `playlist_item_id`.
    pub async fn remove_from_playlist(&self, playlist_id: &str, entry_ids: &[String]) -> Result<()> {
        // Convert to indices and sort descending so removing earlier ones
        // doesn't shift the indices of later removals.
        let mut indices: Vec<u32> = entry_ids
            .iter()
            .filter_map(|s| s.parse::<u32>().ok())
            .collect();
        indices.sort_unstable_by(|a, b| b.cmp(a));

        let mut params = format!("playlistId={}", url_encode(playlist_id));
        for i in indices {
            params.push_str(&format!("&songIndexToRemove={i}"));
        }
        let _: EmptyPayload = self.get_json("updatePlaylist.view", &params).await?;
        Ok(())
    }

    pub async fn delete_playlist(&self, playlist_id: &str) -> Result<()> {
        let params = format!("id={}", url_encode(playlist_id));
        let _: EmptyPayload = self.get_json("deletePlaylist.view", &params).await?;
        Ok(())
    }

    pub async fn get_item_by_id(&self, item_id: &str) -> Result<Item> {
        let params = format!("id={}", url_encode(item_id));
        let payload: SongPayload = self.get_json("getSong.view", &params).await?;
        Ok(payload.song.into_item())
    }

    /// Build a stream URL with embedded auth (token+salt). mpv will fetch
    /// this directly.
    pub fn stream_url(&self, item_id: &str) -> String {
        let auth = auth_query(&self.username, &self.password);
        format!(
            "{}/rest/stream.view?{auth}&id={}",
            self.base_url,
            url_encode(item_id)
        )
    }
}
