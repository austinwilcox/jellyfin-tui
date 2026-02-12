use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::ListState;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crate::client::api::JellyfinClient;
use crate::client::models::Item;
use crate::player::media_controls::{self, MediaEvent};
use crate::player::mpv::{spawn_player_thread, PlayerCommand, PlayerState};
use crate::player::queue::Queue;
use crate::tui::Tui;
use crate::ui;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tab {
    Library,
    Search,
    Queue,
    Recent,
    Playlists,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Focus {
    Artists,
    Albums,
    Tracks,
    QueuePanel,
    Playlists,
    PlaylistTracks,
}

pub struct App {
    pub running: bool,
    pub active_tab: Tab,
    pub focus: Focus,

    // Data
    pub artists: Vec<Item>,
    pub tracks: Vec<Item>,
    pub search_results: Vec<Item>,

    // List states
    pub artist_state: ListState,
    pub track_state: ListState,
    pub queue_state: ListState,
    pub search_state: ListState,

    // Albums
    pub albums: Vec<Item>,
    pub album_state: ListState,
    pub selected_album_name: Option<String>,

    // Selection tracking
    pub selected_artist_name: Option<String>,

    // Filter
    pub filter_active: bool,
    pub filter_text: String,
    filtered_artist_indices: Vec<usize>,

    // Search
    pub search_query: String,
    pub search_focused: bool,

    // Recent
    pub recent_tracks: Vec<Item>,
    pub recent_state: ListState,
    pub recent_loaded: bool,

    // Playlists
    pub playlists: Vec<Item>,
    pub playlist_state: ListState,
    pub playlists_loaded: bool,
    pub playlist_tracks: Vec<Item>,
    pub playlist_track_state: ListState,
    pub selected_playlist_id: Option<String>,
    pub selected_playlist_name: Option<String>,
    pub playlist_create_mode: bool,
    pub playlist_create_name: String,

    // Visual selection mode
    pub visual_mode: bool,
    pub visual_anchor: Option<usize>,

    // Add-to-playlist popup
    pub add_to_playlist_popup: bool,
    pub add_to_playlist_items: Vec<Item>,
    pub add_to_playlist_state: ListState,

    // Player
    pub queue: Queue,
    pub player_state: PlayerState,
    player_cmd_tx: mpsc::Sender<PlayerCommand>,

    // Client
    client: JellyfinClient,

    // Status
    pub status_message: Option<String>,
    pub show_help: bool,

    // Async channel for player state
    player_state_rx: tokio::sync::mpsc::UnboundedReceiver<PlayerState>,

    // Media controls
    media_controls: Option<souvlaki::MediaControls>,
    media_event_rx: tokio::sync::mpsc::UnboundedReceiver<MediaEvent>,

}

impl App {
    pub fn new(client: JellyfinClient) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel();
        let (state_tx, state_rx) = tokio::sync::mpsc::unbounded_channel();

        spawn_player_thread(cmd_rx, state_tx);

        let (media_tx, media_rx) = tokio::sync::mpsc::unbounded_channel();
        let media_controls = media_controls::init_media_controls(media_tx);

        let mut artist_state = ListState::default();
        artist_state.select(Some(0));

        Self {
            running: true,
            active_tab: Tab::Library,
            focus: Focus::Artists,

            artists: Vec::new(),
            tracks: Vec::new(),
            search_results: Vec::new(),

            artist_state,
            track_state: ListState::default(),
            queue_state: ListState::default(),
            search_state: ListState::default(),

            albums: Vec::new(),
            album_state: ListState::default(),
            selected_album_name: None,

            selected_artist_name: None,

            filter_active: false,
            filter_text: String::new(),
            filtered_artist_indices: Vec::new(),

            search_query: String::new(),
            search_focused: false,

            recent_tracks: Vec::new(),
            recent_state: ListState::default(),
            recent_loaded: false,

            playlists: Vec::new(),
            playlist_state: ListState::default(),
            playlists_loaded: false,
            playlist_tracks: Vec::new(),
            playlist_track_state: ListState::default(),
            selected_playlist_id: None,
            selected_playlist_name: None,
            playlist_create_mode: false,
            playlist_create_name: String::new(),

            visual_mode: false,
            visual_anchor: None,

            add_to_playlist_popup: false,
            add_to_playlist_items: Vec::new(),
            add_to_playlist_state: ListState::default(),

            queue: Queue::new(),
            player_state: PlayerState::default(),
            player_cmd_tx: cmd_tx,

            client,

            status_message: None,
            show_help: false,

            player_state_rx: state_rx,

            media_controls,
            media_event_rx: media_rx,

        }
    }

    pub async fn run(&mut self, terminal: &mut Tui) -> Result<()> {
        // Load artists on startup
        self.load_artists().await;

        let tick_rate = Duration::from_millis(100);
        let mut last_tick = Instant::now();

        while self.running {
            // Draw
            terminal.draw(|frame| {
                ui::render(frame, self);
            })?;

            // Poll for player state updates
            while let Ok(state) = self.player_state_rx.try_recv() {
                let was_finished = self.player_state.finished;
                let was_paused = self.player_state.paused;
                let was_playing = self.player_state.playing;
                self.player_state = state;

                // Auto-advance on track finish
                if self.player_state.finished && !was_finished {
                    self.play_next();
                }

                // Update media controls playback state on changes
                if self.player_state.paused != was_paused || self.player_state.playing != was_playing {
                    if let Some(ref mut controls) = self.media_controls {
                        media_controls::update_playback(
                            controls,
                            self.player_state.playing,
                            self.player_state.paused,
                        );
                    }
                }
            }

            // Pump macOS run loop so media key callbacks get delivered
            media_controls::pump_event_loop();

            // Poll for media key events
            while let Ok(event) = self.media_event_rx.try_recv() {
                match event {
                    MediaEvent::Toggle => {
                        let _ = self.player_cmd_tx.send(PlayerCommand::TogglePause);
                    }
                    MediaEvent::Next => {
                        self.play_next();
                    }
                    MediaEvent::Prev => {
                        self.play_prev();
                    }
                }
            }

            // Poll for events with timeout
            let timeout = tick_rate.saturating_sub(last_tick.elapsed());
            if event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    self.handle_key(key).await;
                }
            }

            if last_tick.elapsed() >= tick_rate {
                last_tick = Instant::now();
            }
        }

        let _ = self.player_cmd_tx.send(PlayerCommand::Quit);
        Ok(())
    }

    async fn handle_key(&mut self, key: KeyEvent) {
        // Help overlay intercepts all keys
        if self.show_help {
            match key.code {
                KeyCode::Char('?') | KeyCode::Esc | KeyCode::Char('q') => {
                    self.show_help = false;
                }
                _ => {}
            }
            return;
        }

        // Add-to-playlist popup intercepts all keys
        if self.add_to_playlist_popup {
            self.handle_add_to_playlist_key(key).await;
            return;
        }

        // Playlist create mode intercepts all keys
        if self.playlist_create_mode {
            self.handle_playlist_create_key(key).await;
            return;
        }

        // Visual mode: Esc cancels
        if self.visual_mode && key.code == KeyCode::Esc {
            self.visual_mode = false;
            self.visual_anchor = None;
            return;
        }

        // Filter input mode
        if self.filter_active {
            match key.code {
                KeyCode::Esc => {
                    self.filter_active = false;
                    self.filter_text.clear();
                    self.rebuild_filter();
                }
                KeyCode::Enter => {
                    self.filter_active = false;
                }
                KeyCode::Backspace => {
                    self.filter_text.pop();
                    self.rebuild_filter();
                }
                KeyCode::Char(c) => {
                    self.filter_text.push(c);
                    self.rebuild_filter();
                }
                _ => {}
            }
            return;
        }

        // Search input mode
        if self.search_focused {
            match key.code {
                KeyCode::Esc => {
                    self.search_focused = false;
                }
                KeyCode::Enter => {
                    self.search_focused = false;
                    self.do_search().await;
                }
                KeyCode::Backspace => {
                    self.search_query.pop();
                }
                KeyCode::Char(c) => {
                    self.search_query.push(c);
                }
                _ => {}
            }
            return;
        }

        // Global keys
        match key.code {
            KeyCode::Char('q') => {
                self.running = false;
                return;
            }
            KeyCode::Char('?') => {
                self.show_help = true;
                return;
            }
            KeyCode::Char('1') => {
                self.active_tab = Tab::Library;
                self.visual_mode = false;
                self.visual_anchor = None;
                if !matches!(self.focus, Focus::Artists | Focus::Albums | Focus::Tracks | Focus::QueuePanel) {
                    self.focus = Focus::Artists;
                }
                return;
            }
            KeyCode::Char('2') => {
                self.active_tab = Tab::Search;
                self.visual_mode = false;
                self.visual_anchor = None;
                self.search_focused = true;
                return;
            }
            KeyCode::Char('3') => {
                self.active_tab = Tab::Queue;
                self.visual_mode = false;
                self.visual_anchor = None;
                return;
            }
            KeyCode::Char('4') => {
                self.active_tab = Tab::Recent;
                self.visual_mode = false;
                self.visual_anchor = None;
                if !self.recent_loaded {
                    self.load_recent().await;
                }
                return;
            }
            KeyCode::Char('5') => {
                self.active_tab = Tab::Playlists;
                self.visual_mode = false;
                self.visual_anchor = None;
                if !matches!(self.focus, Focus::Playlists | Focus::PlaylistTracks) {
                    self.focus = Focus::Playlists;
                }
                if !self.playlists_loaded {
                    self.load_playlists().await;
                }
                return;
            }
            KeyCode::Char('a') => {
                let items = if self.visual_mode {
                    self.get_visual_selection_items()
                } else {
                    self.get_selected_audio_item().into_iter().collect()
                };
                if !items.is_empty() {
                    self.add_to_playlist_items = items;
                    self.add_to_playlist_popup = true;
                    self.visual_mode = false;
                    self.visual_anchor = None;
                    if !self.playlists_loaded {
                        self.load_playlists().await;
                    }
                    if self.add_to_playlist_state.selected().is_none() && !self.playlists.is_empty() {
                        self.add_to_playlist_state.select(Some(0));
                    }
                }
                return;
            }
            KeyCode::Char('v') => {
                if self.visual_mode {
                    self.visual_mode = false;
                    self.visual_anchor = None;
                } else {
                    self.visual_mode = true;
                    self.visual_anchor = self.current_cursor_index();
                }
                return;
            }
            // Playback controls (global)
            KeyCode::Char(' ') => {
                let _ = self.player_cmd_tx.send(PlayerCommand::TogglePause);
                return;
            }
            KeyCode::Char('n') => {
                self.play_next();
                return;
            }
            KeyCode::Char('N') => {
                self.play_prev();
                return;
            }
            KeyCode::Char('+') | KeyCode::Char('=') => {
                let vol = (self.player_state.volume + 5).min(100);
                let _ = self.player_cmd_tx.send(PlayerCommand::SetVolume(vol));
                return;
            }
            KeyCode::Char('-') => {
                let vol = (self.player_state.volume - 5).max(0);
                let _ = self.player_cmd_tx.send(PlayerCommand::SetVolume(vol));
                return;
            }
            KeyCode::Char('>') => {
                let _ = self.player_cmd_tx.send(PlayerCommand::SeekForward(10.0));
                return;
            }
            KeyCode::Char('<') => {
                let _ = self.player_cmd_tx.send(PlayerCommand::SeekBackward(10.0));
                return;
            }
            KeyCode::Char('r') => {
                self.queue.repeat = self.queue.repeat.next();
                return;
            }
            KeyCode::Char('s') => {
                self.queue.toggle_shuffle();
                return;
            }
            KeyCode::Tab => {
                self.cycle_focus(true);
                return;
            }
            KeyCode::BackTab => {
                self.cycle_focus(false);
                return;
            }
            _ => {}
        }

        // Tab-specific keys
        match self.active_tab {
            Tab::Library => self.handle_library_key(key).await,
            Tab::Search => self.handle_search_key(key).await,
            Tab::Queue => self.handle_queue_key(key),
            Tab::Recent => self.handle_recent_key(key),
            Tab::Playlists => self.handle_playlists_key(key).await,
        }
    }

    async fn handle_library_key(&mut self, key: KeyEvent) {
        match self.focus {
            Focus::Artists => match key.code {
                KeyCode::Char('j') => self.move_artist_selection(1),
                KeyCode::Char('k') => self.move_artist_selection(-1),
                KeyCode::Char('g') => self.jump_artist_selection(true),
                KeyCode::Char('G') => self.jump_artist_selection(false),
                KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.half_page_artist(true);
                }
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.half_page_artist(false);
                }
                KeyCode::Char('l') | KeyCode::Enter => {
                    self.select_artist().await;
                    self.focus = Focus::Albums;
                }
                KeyCode::Char('/') => {
                    self.filter_active = true;
                    self.filter_text.clear();
                }
                KeyCode::Esc => {
                    if !self.filter_text.is_empty() {
                        self.filter_text.clear();
                        self.rebuild_filter();
                    }
                }
                _ => {}
            },
            Focus::Albums => match key.code {
                KeyCode::Char('j') => self.move_album_selection(1),
                KeyCode::Char('k') => self.move_album_selection(-1),
                KeyCode::Char('g') => {
                    // +1 for "All Tracks" entry
                    if !self.albums.is_empty() || self.selected_artist_name.is_some() {
                        self.album_state.select(Some(0));
                    }
                }
                KeyCode::Char('G') => {
                    // albums.len() items + 1 "All Tracks" entry, so last index = albums.len()
                    if !self.albums.is_empty() {
                        self.album_state.select(Some(self.albums.len()));
                    }
                }
                KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    let len = self.albums.len() + 1; // +1 for "All Tracks"
                    let half = (len / 2).max(1) as i32;
                    self.move_album_selection(half);
                }
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    let len = self.albums.len() + 1;
                    let half = (len / 2).max(1) as i32;
                    self.move_album_selection(-half);
                }
                KeyCode::Char('h') => {
                    self.focus = Focus::Artists;
                }
                KeyCode::Char('l') | KeyCode::Enter => {
                    self.select_album().await;
                    self.focus = Focus::Tracks;
                }
                _ => {}
            },
            Focus::Tracks => match key.code {
                KeyCode::Char('j') => self.move_track_selection(1),
                KeyCode::Char('k') => self.move_track_selection(-1),
                KeyCode::Char('g') => {
                    if !self.tracks.is_empty() {
                        self.track_state.select(Some(0));
                    }
                }
                KeyCode::Char('G') => {
                    if !self.tracks.is_empty() {
                        self.track_state.select(Some(self.tracks.len() - 1));
                    }
                }
                KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    let half = (self.tracks.len() / 2).max(1) as i32;
                    self.move_track_selection(half);
                }
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    let half = (self.tracks.len() / 2).max(1) as i32;
                    self.move_track_selection(-half);
                }
                KeyCode::Char('h') => {
                    self.focus = Focus::Albums;
                }
                KeyCode::Enter => {
                    self.play_selected_track();
                }
                KeyCode::Char('e') => {
                    self.enqueue_selected_track();
                }
                _ => {}
            },
            Focus::QueuePanel => {
                self.handle_queue_key(key);
            }
            _ => {}
        }
    }

    async fn handle_search_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('/') | KeyCode::Char('i') => {
                self.search_focused = true;
            }
            KeyCode::Char('j') => {
                let len = self.search_results.len();
                if len > 0 {
                    let i = self.search_state.selected().unwrap_or(0);
                    self.search_state.select(Some((i + 1).min(len - 1)));
                }
            }
            KeyCode::Char('k') => {
                let i = self.search_state.selected().unwrap_or(0);
                self.search_state.select(Some(i.saturating_sub(1)));
            }
            KeyCode::Char('g') => {
                if !self.search_results.is_empty() {
                    self.search_state.select(Some(0));
                }
            }
            KeyCode::Char('G') => {
                if !self.search_results.is_empty() {
                    self.search_state.select(Some(self.search_results.len() - 1));
                }
            }
            KeyCode::Enter => {
                self.activate_search_result().await;
            }
            KeyCode::Char('e') => {
                self.enqueue_search_result();
            }
            _ => {}
        }
    }

    fn handle_queue_key(&mut self, key: KeyEvent) {
        let len = self.queue.items.len();
        match key.code {
            KeyCode::Char('j') => {
                if len > 0 {
                    let i = self.queue_state.selected().unwrap_or(0);
                    self.queue_state.select(Some((i + 1).min(len - 1)));
                }
            }
            KeyCode::Char('k') => {
                let i = self.queue_state.selected().unwrap_or(0);
                self.queue_state.select(Some(i.saturating_sub(1)));
            }
            KeyCode::Char('g') => {
                if len > 0 {
                    self.queue_state.select(Some(0));
                }
            }
            KeyCode::Char('G') => {
                if len > 0 {
                    self.queue_state.select(Some(len - 1));
                }
            }
            KeyCode::Enter => {
                // Jump to selected queue item and play it
                if let Some(i) = self.queue_state.selected() {
                    if i < self.queue.items.len() {
                        self.queue.current = Some(i);
                        if let Some(item) = self.queue.current_item().cloned() {
                            let url = self.client.stream_url(&item.id);
                            let _ = self.player_cmd_tx.send(PlayerCommand::Play(url));
                            self.update_media_metadata(&item);
                        }
                    }
                }
            }
            KeyCode::Char('d') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Remove selected from queue
                if let Some(i) = self.queue_state.selected() {
                    self.queue.remove(i);
                    if i >= self.queue.items.len() && !self.queue.items.is_empty() {
                        self.queue_state.select(Some(self.queue.items.len() - 1));
                    }
                }
            }
            KeyCode::Char('c') => {
                self.queue.clear();
                let _ = self.player_cmd_tx.send(PlayerCommand::Stop);
                self.queue_state.select(None);
            }
            _ => {}
        }
    }

    // --- Helpers ---

    fn cycle_focus(&mut self, forward: bool) {
        let panels = match self.active_tab {
            Tab::Library => vec![Focus::Artists, Focus::Albums, Focus::Tracks, Focus::QueuePanel],
            Tab::Playlists => vec![Focus::Playlists, Focus::PlaylistTracks],
            Tab::Search => return,
            Tab::Queue => return,
            Tab::Recent => return,
        };

        if let Some(pos) = panels.iter().position(|f| *f == self.focus) {
            let next = if forward {
                (pos + 1) % panels.len()
            } else {
                (pos + panels.len() - 1) % panels.len()
            };
            self.focus = panels[next];
        }
    }

    async fn load_artists(&mut self) {
        match self.client.get_artists().await {
            Ok(artists) => {
                self.artists = artists;
                if !self.artists.is_empty() {
                    self.artist_state.select(Some(0));
                }
                self.rebuild_filter();
            }
            Err(e) => {
                self.status_message = Some(format!("Failed to load artists: {e}"));
            }
        }
    }

    pub fn selected_artist_id(&self) -> Option<String> {
        let idx = self.artist_state.selected()?;
        let artists = self.filtered_artists();
        artists.get(idx).map(|a| a.id.clone())
    }

    pub fn filtered_artists(&self) -> Vec<&Item> {
        if self.filter_text.is_empty() {
            self.artists.iter().collect()
        } else {
            self.filtered_artist_indices
                .iter()
                .filter_map(|&i| self.artists.get(i))
                .collect()
        }
    }

    fn rebuild_filter(&mut self) {
        if self.filter_text.is_empty() {
            self.filtered_artist_indices = (0..self.artists.len()).collect();
        } else {
            let query = self.filter_text.to_lowercase();
            self.filtered_artist_indices = self
                .artists
                .iter()
                .enumerate()
                .filter(|(_, a)| a.name.to_lowercase().contains(&query))
                .map(|(i, _)| i)
                .collect();
        }

        if !self.filtered_artist_indices.is_empty() {
            let sel = self.artist_state.selected().unwrap_or(0);
            if sel >= self.filtered_artist_indices.len() {
                self.artist_state.select(Some(0));
            }
        } else {
            self.artist_state.select(None);
        }
    }

    async fn select_artist(&mut self) {
        if let Some(artist_id) = self.selected_artist_id() {
            let filtered = self.filtered_artists();
            self.selected_artist_name = filtered
                .iter()
                .find(|a| a.id == artist_id)
                .map(|a| a.name.clone());

            match self.client.get_artist_albums(&artist_id).await {
                Ok(albums) => {
                    self.albums = albums;
                    self.album_state.select(Some(0)); // "All Tracks" at index 0
                    self.selected_album_name = None;
                    self.tracks.clear();
                    self.track_state.select(None);
                }
                Err(e) => {
                    self.status_message = Some(format!("Failed to load albums: {e}"));
                }
            }
        }
    }

    async fn select_album(&mut self) {
        let idx = match self.album_state.selected() {
            Some(i) => i,
            None => return,
        };

        if idx == 0 {
            // "All Tracks" — load all tracks by this artist
            if let Some(artist_id) = self.selected_artist_id() {
                self.selected_album_name = Some("All Tracks".to_string());
                match self.client.get_artist_tracks(&artist_id).await {
                    Ok(tracks) => {
                        self.tracks = tracks;
                        self.track_state.select(if self.tracks.is_empty() {
                            None
                        } else {
                            Some(0)
                        });
                    }
                    Err(e) => {
                        self.status_message = Some(format!("Failed to load tracks: {e}"));
                    }
                }
            }
        } else {
            // Specific album — index into albums (offset by 1 for "All Tracks")
            let album_idx = idx - 1;
            if let Some(album) = self.albums.get(album_idx).cloned() {
                self.selected_album_name = Some(album.name.clone());
                match self.client.get_album_tracks(&album.id).await {
                    Ok(tracks) => {
                        self.tracks = tracks;
                        self.track_state.select(if self.tracks.is_empty() {
                            None
                        } else {
                            Some(0)
                        });
                    }
                    Err(e) => {
                        self.status_message = Some(format!("Failed to load tracks: {e}"));
                    }
                }
            }
        }
    }

    fn move_album_selection(&mut self, delta: i32) {
        let len = self.albums.len() + 1; // +1 for "All Tracks"
        if len == 0 {
            return;
        }
        let current = self.album_state.selected().unwrap_or(0) as i32;
        let new = (current + delta).clamp(0, len as i32 - 1) as usize;
        self.album_state.select(Some(new));
    }

    fn play_selected_track(&mut self) {
        if let Some(idx) = self.track_state.selected() {
            if idx < self.tracks.len() {
                // Replace queue with all tracks from this artist, starting at selected
                self.queue.replace(self.tracks.clone(), idx);
                self.queue_state.select(Some(idx));

                if let Some(item) = self.queue.current_item().cloned() {
                    let url = self.client.stream_url(&item.id);
                    let _ = self.player_cmd_tx.send(PlayerCommand::Play(url));
                    self.update_media_metadata(&item);
                }
            }
        }
    }

    fn enqueue_selected_track(&mut self) {
        if let Some(idx) = self.track_state.selected() {
            if let Some(track) = self.tracks.get(idx) {
                self.queue.enqueue(track.clone());
                self.status_message = Some(format!("Enqueued: {}", track.name));
            }
        }
    }

    fn play_next(&mut self) {
        if let Some(item) = self.queue.next().cloned() {
            let url = self.client.stream_url(&item.id);
            let _ = self.player_cmd_tx.send(PlayerCommand::Play(url));
            self.queue_state.select(self.queue.current);
            self.update_media_metadata(&item);
        }
    }

    fn play_prev(&mut self) {
        if let Some(item) = self.queue.prev().cloned() {
            let url = self.client.stream_url(&item.id);
            let _ = self.player_cmd_tx.send(PlayerCommand::Play(url));
            self.queue_state.select(self.queue.current);
            self.update_media_metadata(&item);
        }
    }

    fn update_media_metadata(&mut self, item: &Item) {
        if let Some(ref mut controls) = self.media_controls {
            let artist = item.artist_display();
            let album = item.album.as_deref().unwrap_or("");
            media_controls::update_metadata(controls, &item.name, &artist, album);
            media_controls::update_playback(controls, true, false);
        }
    }

    async fn do_search(&mut self) {
        if self.search_query.is_empty() {
            return;
        }
        match self.client.search(&self.search_query).await {
            Ok(results) => {
                self.search_results = results;
                self.search_state.select(if self.search_results.is_empty() {
                    None
                } else {
                    Some(0)
                });
            }
            Err(e) => {
                self.status_message = Some(format!("Search failed: {e}"));
            }
        }
    }

    async fn activate_search_result(&mut self) {
        if let Some(idx) = self.search_state.selected() {
            if let Some(item) = self.search_results.get(idx).cloned() {
                match item.item_type.as_deref() {
                    Some("Audio") => {
                        // Play this track
                        self.queue.enqueue(item.clone());
                        let last = self.queue.items.len() - 1;
                        self.queue.current = Some(last);
                        let url = self.client.stream_url(&item.id);
                        let _ = self.player_cmd_tx.send(PlayerCommand::Play(url));
                        self.update_media_metadata(&item);
                    }
                    Some("MusicArtist") => {
                        // Load artist albums
                        match self.client.get_artist_albums(&item.id).await {
                            Ok(albums) => {
                                self.albums = albums;
                                self.album_state.select(Some(0));
                                self.selected_artist_name = Some(item.name.clone());
                                self.selected_album_name = None;
                                self.tracks.clear();
                                self.track_state.select(None);
                                self.active_tab = Tab::Library;
                                self.focus = Focus::Albums;
                            }
                            Err(e) => {
                                self.status_message =
                                    Some(format!("Failed to load artist: {e}"));
                            }
                        }
                    }
                    Some("MusicAlbum") => {
                        // Load album tracks
                        match self.client.get_album_tracks(&item.id).await {
                            Ok(tracks) => {
                                self.tracks = tracks;
                                self.track_state.select(if self.tracks.is_empty() {
                                    None
                                } else {
                                    Some(0)
                                });
                                self.selected_artist_name = Some(
                                    item.album_artist
                                        .as_deref()
                                        .unwrap_or(&item.name)
                                        .to_string(),
                                );
                                self.selected_album_name = Some(item.name.clone());
                                self.active_tab = Tab::Library;
                                self.focus = Focus::Tracks;
                            }
                            Err(e) => {
                                self.status_message =
                                    Some(format!("Failed to load album: {e}"));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn enqueue_search_result(&mut self) {
        if let Some(idx) = self.search_state.selected() {
            if let Some(item) = self.search_results.get(idx) {
                if item.item_type.as_deref() == Some("Audio") {
                    self.queue.enqueue(item.clone());
                    self.status_message = Some(format!("Enqueued: {}", item.name));
                }
            }
        }
    }

    fn move_artist_selection(&mut self, delta: i32) {
        let len = self.filtered_artists().len();
        if len == 0 {
            return;
        }
        let current = self.artist_state.selected().unwrap_or(0) as i32;
        let new = (current + delta).clamp(0, len as i32 - 1) as usize;
        self.artist_state.select(Some(new));
    }

    fn jump_artist_selection(&mut self, top: bool) {
        let len = self.filtered_artists().len();
        if len == 0 {
            return;
        }
        if top {
            self.artist_state.select(Some(0));
        } else {
            self.artist_state.select(Some(len - 1));
        }
    }

    fn half_page_artist(&mut self, down: bool) {
        let len = self.filtered_artists().len();
        let half = (len / 2).max(1) as i32;
        self.move_artist_selection(if down { half } else { -half });
    }

    fn move_track_selection(&mut self, delta: i32) {
        let len = self.tracks.len();
        if len == 0 {
            return;
        }
        let current = self.track_state.selected().unwrap_or(0) as i32;
        let new = (current + delta).clamp(0, len as i32 - 1) as usize;
        self.track_state.select(Some(new));
    }

    fn handle_recent_key(&mut self, key: KeyEvent) {
        let len = self.recent_tracks.len();
        match key.code {
            KeyCode::Char('j') => {
                if len > 0 {
                    let i = self.recent_state.selected().unwrap_or(0);
                    self.recent_state.select(Some((i + 1).min(len - 1)));
                }
            }
            KeyCode::Char('k') => {
                let i = self.recent_state.selected().unwrap_or(0);
                self.recent_state.select(Some(i.saturating_sub(1)));
            }
            KeyCode::Char('g') => {
                if len > 0 {
                    self.recent_state.select(Some(0));
                }
            }
            KeyCode::Char('G') => {
                if len > 0 {
                    self.recent_state.select(Some(len - 1));
                }
            }
            KeyCode::Enter => {
                if let Some(idx) = self.recent_state.selected() {
                    if idx < self.recent_tracks.len() {
                        self.queue.replace(self.recent_tracks.clone(), idx);
                        self.queue_state.select(Some(idx));
                        if let Some(item) = self.queue.current_item().cloned() {
                            let url = self.client.stream_url(&item.id);
                            let _ = self.player_cmd_tx.send(PlayerCommand::Play(url));
                            self.update_media_metadata(&item);
                        }
                    }
                }
            }
            KeyCode::Char('e') => {
                if let Some(idx) = self.recent_state.selected() {
                    if let Some(track) = self.recent_tracks.get(idx) {
                        self.queue.enqueue(track.clone());
                        self.status_message = Some(format!("Enqueued: {}", track.name));
                    }
                }
            }
            _ => {}
        }
    }

    async fn load_recent(&mut self) {
        match self.client.get_recent_tracks(100).await {
            Ok(tracks) => {
                self.recent_tracks = tracks;
                self.recent_state.select(if self.recent_tracks.is_empty() {
                    None
                } else {
                    Some(0)
                });
                self.recent_loaded = true;
            }
            Err(e) => {
                self.status_message = Some(format!("Failed to load recent: {e}"));
            }
        }
    }

    // --- Playlists ---

    async fn load_playlists(&mut self) {
        match self.client.get_playlists().await {
            Ok(playlists) => {
                self.playlists = playlists;
                self.playlist_state.select(if self.playlists.is_empty() {
                    None
                } else {
                    Some(0)
                });
                self.playlists_loaded = true;
            }
            Err(e) => {
                self.status_message = Some(format!("Failed to load playlists: {e}"));
            }
        }
    }

    async fn load_playlist_tracks(&mut self) {
        if let Some(ref id) = self.selected_playlist_id.clone() {
            match self.client.get_playlist_tracks(id).await {
                Ok(tracks) => {
                    self.playlist_tracks = tracks;
                    self.playlist_track_state.select(if self.playlist_tracks.is_empty() {
                        None
                    } else {
                        Some(0)
                    });
                }
                Err(e) => {
                    self.status_message = Some(format!("Failed to load playlist tracks: {e}"));
                }
            }
        }
    }

    async fn handle_playlists_key(&mut self, key: KeyEvent) {
        match self.focus {
            Focus::Playlists => match key.code {
                KeyCode::Char('j') => {
                    let len = self.playlists.len();
                    if len > 0 {
                        let i = self.playlist_state.selected().unwrap_or(0);
                        self.playlist_state.select(Some((i + 1).min(len - 1)));
                    }
                }
                KeyCode::Char('k') => {
                    let i = self.playlist_state.selected().unwrap_or(0);
                    self.playlist_state.select(Some(i.saturating_sub(1)));
                }
                KeyCode::Char('g') => {
                    if !self.playlists.is_empty() {
                        self.playlist_state.select(Some(0));
                    }
                }
                KeyCode::Char('G') => {
                    if !self.playlists.is_empty() {
                        self.playlist_state.select(Some(self.playlists.len() - 1));
                    }
                }
                KeyCode::Enter | KeyCode::Char('l') => {
                    if let Some(idx) = self.playlist_state.selected() {
                        if let Some(pl) = self.playlists.get(idx) {
                            self.selected_playlist_id = Some(pl.id.clone());
                            self.selected_playlist_name = Some(pl.name.clone());
                            self.load_playlist_tracks().await;
                            self.focus = Focus::PlaylistTracks;
                        }
                    }
                }
                KeyCode::Char('C') => {
                    self.playlist_create_mode = true;
                    self.playlist_create_name.clear();
                }
                KeyCode::Char('D') => {
                    if let Some(idx) = self.playlist_state.selected() {
                        if let Some(pl) = self.playlists.get(idx).cloned() {
                            match self.client.delete_playlist(&pl.id).await {
                                Ok(()) => {
                                    self.status_message = Some(format!("Deleted playlist: {}", pl.name));
                                    self.playlists.remove(idx);
                                    if idx >= self.playlists.len() && !self.playlists.is_empty() {
                                        self.playlist_state.select(Some(self.playlists.len() - 1));
                                    } else if self.playlists.is_empty() {
                                        self.playlist_state.select(None);
                                    }
                                    if self.selected_playlist_id.as_deref() == Some(&pl.id) {
                                        self.selected_playlist_id = None;
                                        self.selected_playlist_name = None;
                                        self.playlist_tracks.clear();
                                    }
                                }
                                Err(e) => {
                                    self.status_message = Some(format!("Failed to delete playlist: {e}"));
                                }
                            }
                        }
                    }
                }
                _ => {}
            },
            Focus::PlaylistTracks => match key.code {
                KeyCode::Char('j') => {
                    let len = self.playlist_tracks.len();
                    if len > 0 {
                        let i = self.playlist_track_state.selected().unwrap_or(0);
                        self.playlist_track_state.select(Some((i + 1).min(len - 1)));
                    }
                }
                KeyCode::Char('k') => {
                    let i = self.playlist_track_state.selected().unwrap_or(0);
                    self.playlist_track_state.select(Some(i.saturating_sub(1)));
                }
                KeyCode::Char('g') => {
                    if !self.playlist_tracks.is_empty() {
                        self.playlist_track_state.select(Some(0));
                    }
                }
                KeyCode::Char('G') => {
                    if !self.playlist_tracks.is_empty() {
                        self.playlist_track_state.select(Some(self.playlist_tracks.len() - 1));
                    }
                }
                KeyCode::Char('h') => {
                    self.focus = Focus::Playlists;
                }
                KeyCode::Enter => {
                    if let Some(idx) = self.playlist_track_state.selected() {
                        if idx < self.playlist_tracks.len() {
                            self.queue.replace(self.playlist_tracks.clone(), idx);
                            self.queue_state.select(Some(idx));
                            if let Some(item) = self.queue.current_item().cloned() {
                                let url = self.client.stream_url(&item.id);
                                let _ = self.player_cmd_tx.send(PlayerCommand::Play(url));
                                self.update_media_metadata(&item);
                            }
                        }
                    }
                }
                KeyCode::Char('e') => {
                    if let Some(idx) = self.playlist_track_state.selected() {
                        if let Some(track) = self.playlist_tracks.get(idx) {
                            self.queue.enqueue(track.clone());
                            self.status_message = Some(format!("Enqueued: {}", track.name));
                        }
                    }
                }
                KeyCode::Char('d') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    if let Some(idx) = self.playlist_track_state.selected() {
                        if let Some(ref playlist_id) = self.selected_playlist_id.clone() {
                            if let Some(track) = self.playlist_tracks.get(idx) {
                                if let Some(ref entry_id) = track.playlist_item_id {
                                    match self.client.remove_from_playlist(playlist_id, &[entry_id.clone()]).await {
                                        Ok(()) => {
                                            self.status_message = Some(format!("Removed: {}", track.name));
                                            self.playlist_tracks.remove(idx);
                                            if idx >= self.playlist_tracks.len() && !self.playlist_tracks.is_empty() {
                                                self.playlist_track_state.select(Some(self.playlist_tracks.len() - 1));
                                            } else if self.playlist_tracks.is_empty() {
                                                self.playlist_track_state.select(None);
                                            }
                                        }
                                        Err(e) => {
                                            self.status_message = Some(format!("Failed to remove track: {e}"));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }

    async fn handle_playlist_create_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.playlist_create_mode = false;
                self.playlist_create_name.clear();
            }
            KeyCode::Enter => {
                if !self.playlist_create_name.is_empty() {
                    let name = self.playlist_create_name.clone();
                    match self.client.create_playlist(&name).await {
                        Ok(_result) => {
                            self.status_message = Some(format!("Created playlist: {name}"));
                            self.load_playlists().await;
                        }
                        Err(e) => {
                            self.status_message = Some(format!("Failed to create playlist: {e}"));
                        }
                    }
                }
                self.playlist_create_mode = false;
                self.playlist_create_name.clear();
            }
            KeyCode::Backspace => {
                self.playlist_create_name.pop();
            }
            KeyCode::Char(c) => {
                self.playlist_create_name.push(c);
            }
            _ => {}
        }
    }

    async fn handle_add_to_playlist_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.add_to_playlist_popup = false;
                self.add_to_playlist_items.clear();
            }
            KeyCode::Char('j') => {
                let len = self.playlists.len();
                if len > 0 {
                    let i = self.add_to_playlist_state.selected().unwrap_or(0);
                    self.add_to_playlist_state.select(Some((i + 1).min(len - 1)));
                }
            }
            KeyCode::Char('k') => {
                let i = self.add_to_playlist_state.selected().unwrap_or(0);
                self.add_to_playlist_state.select(Some(i.saturating_sub(1)));
            }
            KeyCode::Enter => {
                if let Some(idx) = self.add_to_playlist_state.selected() {
                    if let Some(playlist) = self.playlists.get(idx).cloned() {
                        if !self.add_to_playlist_items.is_empty() {
                            let ids: Vec<String> = self.add_to_playlist_items.iter().map(|i| i.id.clone()).collect();
                            let count = ids.len();
                            match self.client.add_to_playlist(&playlist.id, &ids).await {
                                Ok(()) => {
                                    if count == 1 {
                                        self.status_message = Some(format!("Added \"{}\" to {}", self.add_to_playlist_items[0].name, playlist.name));
                                    } else {
                                        self.status_message = Some(format!("Added {count} tracks to {}", playlist.name));
                                    }
                                    if self.selected_playlist_id.as_deref() == Some(&playlist.id) {
                                        self.load_playlist_tracks().await;
                                    }
                                }
                                Err(e) => {
                                    self.status_message = Some(format!("Failed to add to playlist: {e}"));
                                }
                            }
                        }
                    }
                }
                self.add_to_playlist_popup = false;
                self.add_to_playlist_items.clear();
            }
            _ => {}
        }
    }

    fn get_selected_audio_item(&self) -> Option<Item> {
        match self.active_tab {
            Tab::Library => {
                if self.focus == Focus::Tracks {
                    self.track_state.selected().and_then(|i| self.tracks.get(i).cloned())
                } else {
                    None
                }
            }
            Tab::Search => {
                self.search_state.selected().and_then(|i| {
                    self.search_results.get(i).and_then(|item| {
                        if item.item_type.as_deref() == Some("Audio") {
                            Some(item.clone())
                        } else {
                            None
                        }
                    })
                })
            }
            Tab::Queue => {
                self.queue_state.selected().and_then(|i| self.queue.items.get(i).cloned())
            }
            Tab::Recent => {
                self.recent_state.selected().and_then(|i| self.recent_tracks.get(i).cloned())
            }
            Tab::Playlists => {
                if self.focus == Focus::PlaylistTracks {
                    self.playlist_track_state.selected().and_then(|i| self.playlist_tracks.get(i).cloned())
                } else {
                    None
                }
            }
        }
    }

    /// Returns the current cursor index for the active list.
    fn current_cursor_index(&self) -> Option<usize> {
        match self.active_tab {
            Tab::Library => {
                if self.focus == Focus::Tracks {
                    self.track_state.selected()
                } else {
                    None
                }
            }
            Tab::Search => self.search_state.selected(),
            Tab::Queue => self.queue_state.selected(),
            Tab::Recent => self.recent_state.selected(),
            Tab::Playlists => {
                if self.focus == Focus::PlaylistTracks {
                    self.playlist_track_state.selected()
                } else {
                    None
                }
            }
        }
    }

    /// Returns (start, end) inclusive range of the visual selection for the active list.
    /// Returns None if visual mode is off or anchor is missing.
    pub fn visual_selection_range(&self) -> Option<(usize, usize)> {
        if !self.visual_mode {
            return None;
        }
        let anchor = self.visual_anchor?;
        let cursor = self.current_cursor_index()?;
        Some((anchor.min(cursor), anchor.max(cursor)))
    }

    fn get_visual_selection_items(&self) -> Vec<Item> {
        let (start, end) = match self.visual_selection_range() {
            Some(r) => r,
            None => return Vec::new(),
        };
        let items: &[Item] = match self.active_tab {
            Tab::Library if self.focus == Focus::Tracks => &self.tracks,
            Tab::Search => &self.search_results,
            Tab::Queue => &self.queue.items,
            Tab::Recent => &self.recent_tracks,
            Tab::Playlists if self.focus == Focus::PlaylistTracks => &self.playlist_tracks,
            _ => return Vec::new(),
        };
        items.get(start..=end)
            .unwrap_or(&[])
            .iter()
            .filter(|item| {
                // For search results, only include audio items
                if self.active_tab == Tab::Search {
                    item.item_type.as_deref() == Some("Audio")
                } else {
                    true
                }
            })
            .cloned()
            .collect()
    }
}
