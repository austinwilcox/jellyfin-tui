use anyhow::{bail, Context, Result};
use reqwest::Client;

use crate::client::auth::media_browser_header;
use crate::client::models::{CreatePlaylistRequest, Item, ItemsResponse, PlaylistCreationResult};
use crate::config::Config;

#[derive(Clone)]
pub struct JellyfinClient {
    client: Client,
    pub base_url: String,
    pub token: String,
    pub user_id: String,
    music_library_id: Option<String>,
}

impl JellyfinClient {
    pub fn new(config: &Config) -> Result<Self> {
        let token = config.token.clone().context("No auth token")?;
        let user_id = config.user_id.clone().context("No user ID")?;
        Ok(Self {
            client: Client::new(),
            base_url: config.server_url.clone(),
            token,
            user_id,
            music_library_id: None,
        })
    }

    fn auth_header(&self) -> String {
        media_browser_header(Some(&self.token))
    }

    pub async fn find_music_library(&mut self) -> Result<String> {
        if let Some(ref id) = self.music_library_id {
            return Ok(id.clone());
        }

        let url = format!(
            "{}/Users/{}/Views",
            self.base_url, self.user_id
        );
        let data: ItemsResponse = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?
            .json()
            .await?;

        for item in &data.items {
            if item.collection_type.as_deref() == Some("music") {
                self.music_library_id = Some(item.id.clone());
                return Ok(item.id.clone());
            }
        }

        bail!("No music library found on this Jellyfin server")
    }

    pub async fn get_artists(&mut self) -> Result<Vec<Item>> {
        let lib_id = self.find_music_library().await?;
        let url = format!(
            "{}/Artists?ParentId={}&UserId={}&SortBy=SortName&SortOrder=Ascending&Recursive=true&Fields=SortName",
            self.base_url, lib_id, self.user_id
        );
        let data: ItemsResponse = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?
            .json()
            .await?;
        Ok(data.items)
    }

    pub async fn get_artist_albums(&self, artist_id: &str) -> Result<Vec<Item>> {
        let url = format!(
            "{}/Users/{}/Items?ArtistIds={}&IncludeItemTypes=MusicAlbum&Recursive=true&SortBy=ProductionYear,SortName&SortOrder=Ascending&Fields=ProductionYear,AlbumArtist",
            self.base_url, self.user_id, artist_id
        );
        let data: ItemsResponse = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?
            .json()
            .await?;
        Ok(data.items)
    }

    pub async fn get_artist_tracks(&self, artist_id: &str) -> Result<Vec<Item>> {
        let url = format!(
            "{}/Users/{}/Items?ArtistIds={}&IncludeItemTypes=Audio&Recursive=true&SortBy=Album,ParentIndexNumber,IndexNumber&SortOrder=Ascending&Fields=Artists,Album,RunTimeTicks,IndexNumber,ParentIndexNumber,AlbumArtist,ProductionYear,AlbumId,MediaSources",
            self.base_url, self.user_id, artist_id
        );
        let data: ItemsResponse = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?
            .json()
            .await?;
        Ok(data.items)
    }

    pub async fn get_album_tracks(&self, album_id: &str) -> Result<Vec<Item>> {
        let url = format!(
            "{}/Users/{}/Items?ParentId={}&IncludeItemTypes=Audio&Recursive=true&SortBy=ParentIndexNumber,IndexNumber&SortOrder=Ascending&Fields=Artists,Album,RunTimeTicks,IndexNumber,ParentIndexNumber,AlbumArtist,MediaSources",
            self.base_url, self.user_id, album_id
        );
        let data: ItemsResponse = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?
            .json()
            .await?;
        Ok(data.items)
    }

    pub async fn search(&self, query: &str) -> Result<Vec<Item>> {
        let url = format!(
            "{}/Users/{}/Items?SearchTerm={}&IncludeItemTypes=MusicArtist,MusicAlbum,Audio&Recursive=true&Limit=50&Fields=Artists,Album,RunTimeTicks,IndexNumber,AlbumArtist,ProductionYear,MediaSources",
            self.base_url, self.user_id,
            urlencoding(query)
        );
        let data: ItemsResponse = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?
            .json()
            .await?;
        Ok(data.items)
    }

    pub async fn get_recent_tracks(&self, limit: u32) -> Result<Vec<Item>> {
        let url = format!(
            "{}/Users/{}/Items?IncludeItemTypes=Audio&SortBy=DateCreated&SortOrder=Descending&Recursive=true&Limit={}&Fields=Artists,Album,RunTimeTicks,IndexNumber,AlbumArtist,ProductionYear,Genres,DateCreated,MediaSources",
            self.base_url, self.user_id, limit
        );
        let data: ItemsResponse = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?
            .json()
            .await?;
        Ok(data.items)
    }

    pub async fn get_playlists(&self) -> Result<Vec<Item>> {
        let url = format!(
            "{}/Users/{}/Items?IncludeItemTypes=Playlist&Recursive=true&SortBy=SortName&SortOrder=Ascending",
            self.base_url, self.user_id
        );
        let data: ItemsResponse = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?
            .json()
            .await?;
        Ok(data.items)
    }

    pub async fn get_playlist_tracks(&self, playlist_id: &str) -> Result<Vec<Item>> {
        let url = format!(
            "{}/Playlists/{}/Items?userId={}&Fields=Artists,Album,RunTimeTicks,IndexNumber,AlbumArtist,ProductionYear,Genres,MediaSources",
            self.base_url, playlist_id, self.user_id
        );
        let data: ItemsResponse = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?
            .json()
            .await?;
        Ok(data.items)
    }

    pub async fn create_playlist(&self, name: &str) -> Result<PlaylistCreationResult> {
        let url = format!("{}/Playlists", self.base_url);
        let body = CreatePlaylistRequest {
            name: name.to_string(),
            user_id: self.user_id.clone(),
            media_type: "Audio".to_string(),
        };
        let result: PlaylistCreationResult = self
            .client
            .post(&url)
            .header("Authorization", self.auth_header())
            .json(&body)
            .send()
            .await?
            .json()
            .await?;
        Ok(result)
    }

    pub async fn add_to_playlist(&self, playlist_id: &str, item_ids: &[String]) -> Result<()> {
        let ids = item_ids.join(",");
        let url = format!(
            "{}/Playlists/{}/Items?ids={}&userId={}",
            self.base_url, playlist_id, ids, self.user_id
        );
        self.client
            .post(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?;
        Ok(())
    }

    pub async fn remove_from_playlist(&self, playlist_id: &str, entry_ids: &[String]) -> Result<()> {
        let ids = entry_ids.join(",");
        let url = format!(
            "{}/Playlists/{}/Items?entryIds={}",
            self.base_url, playlist_id, ids
        );
        self.client
            .delete(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?;
        Ok(())
    }

    pub async fn delete_playlist(&self, playlist_id: &str) -> Result<()> {
        let url = format!("{}/Items/{}", self.base_url, playlist_id);
        self.client
            .delete(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?;
        Ok(())
    }

    /// Lightweight authenticated ping to keep the session alive.
    pub async fn ping(&self) -> Result<()> {
        let url = format!("{}/System/Info", self.base_url);
        self.client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?;
        Ok(())
    }

    pub fn stream_url(&self, item_id: &str) -> String {
        format!(
            "{}/Audio/{}/stream?static=true&api_key={}",
            self.base_url, item_id, self.token
        )
    }
}

fn urlencoding(s: &str) -> String {
    let mut result = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(b as char);
            }
            _ => {
                result.push_str(&format!("%{:02X}", b));
            }
        }
    }
    result
}
