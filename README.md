# navidrome-tui

A terminal-based music player for [Navidrome](https://www.navidrome.org/) (and other Subsonic-compatible) servers, built with Rust.

Forked from [jellyfin-tui](https://github.com/austinwilcox/jellyfin-tui) — same TUI, same keybindings, talks to the Subsonic API instead.

Browse your library, search for music, manage playlists, and control playback entirely from the terminal with vim-style keybindings. Supports media key controls (play/pause/next/previous) from headphones and keyboards on macOS and Linux.

## Features

- **Library browsing** -- Browse artists, albums, and tracks with a filterable artist list
- **Search** -- Search across artists, albums, and tracks
- **Queue management** -- Full queue with repeat (off/all/one) and shuffle modes
- **Recently added** -- View recently added tracks with all artists and genre info
- **Playlists** -- Full CRUD: create, browse, add/remove tracks, delete playlists
- **Add-to-playlist popup** -- Press `a` from any tab to add the selected track to a playlist
- **Media key support** -- Play/pause/next/previous from headphones and keyboard media keys via MPRIS (Linux) and MediaPlayer framework (macOS)
- **Now playing bar** -- Always-visible bar with track info, time, codec/bitrate (e.g. `FLAC @ 1411 kbps`), volume, and repeat/shuffle status
- **Scrolling text** -- Long track/artist names scroll automatically when they don't fit in the available space
- **Multi-server support** -- Configure multiple Navidrome/Subsonic servers and switch between them
- **Session keep-alive** -- Periodically pings the server to surface auth/connection issues early

## Dependencies

### macOS

```bash
brew install mpv rust
```

### Ubuntu / Debian

```bash
sudo apt install libmpv-dev libdbus-1-dev libssl-dev pkg-config build-essential
```

You also need Rust installed. If you don't have it:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Raspberry Pi (Pi 4 / Pi 5)

```bash
sudo apt install libmpv-dev libdbus-1-dev libssl-dev pkg-config build-essential
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Note: the first build will be slow on a Pi. Use `cargo build --release` for a faster binary.

### Arch Linux

```bash
sudo pacman -S mpv dbus openssl pkg-config
```

## Building

```bash
git clone https://github.com/your-username/navidrome-tui.git
cd navidrome-tui
cargo build --release
```

The binary will be at `target/release/navidrome-tui`.

## Configuration

On first run, the app will prompt you for your Navidrome server details:

```
Navidrome server URL (e.g. http://localhost:4533):
Username:
Password:
```

Configuration is stored at `~/.config/navidrome-tui/config.toml`. The password is stored in plaintext and used to compute a per-request salted MD5 token (Subsonic's standard auth scheme); no long-lived session token is kept on disk.

### Multi-server

You can configure multiple servers. On launch, you'll be prompted to pick one:

```toml
active = "home"

[[servers]]
name = "home"
server_url = "http://192.168.1.10:4533"
username = "alice"
password = "password"

[[servers]]
name = "remote"
server_url = "https://navidrome.example.com"
username = "alice"
password = "password"
```

Old single-server configs are automatically migrated to the new format.

## Usage

Run the app:

```bash
navidrome-tui
```

Or if running from the source directory:

```bash
cargo run --release
```

### Editing the Config

To open the config file in your default editor (`$EDITOR`):

```bash
navidrome-tui config
```

### Tabs

Switch between tabs using the number keys:

| Key | Tab | Description |
|-----|-----|-------------|
| `1` | **Library** | Browse artists and their tracks. Three-panel layout: artists, tracks, and queue sidebar. |
| `2` | **Search** | Search for artists, albums, and tracks. Opens with the search input focused. |
| `3` | **Queue** | View and manage the current playback queue. |
| `4` | **Recent** | Recently added tracks with full artist list, album, genre, and duration. |
| `5` | **Playlists** | Browse and manage playlists. Two-panel layout: playlist list and tracks. |

### Keybindings

Press `?` at any time to toggle the in-app help overlay.

#### Navigation

| Key | Action |
|-----|--------|
| `j` / `k` | Move down / up |
| `g` / `G` | Jump to top / bottom |
| `Ctrl+d` / `Ctrl+u` | Half-page down / up |
| `h` / `l` | Back / drill-in (Library and Playlists tabs) |
| `Tab` / `Shift+Tab` | Cycle focus between panels |
| `/` | Filter artist list (Library tab) |
| `Esc` | Clear filter / close popup |

#### Playback

| Key | Action |
|-----|--------|
| `Enter` | Play selected track (replaces queue with current context) |
| `Space` | Toggle pause |
| `n` / `N` | Next / previous track |
| `>` / `<` | Seek forward / backward 10 seconds |
| `+` / `-` | Volume up / down (5% increments) |
| `r` | Cycle repeat mode (Off / All / One) |
| `s` | Toggle shuffle |

#### Queue

| Key | Action |
|-----|--------|
| `e` | Enqueue selected track (adds to end of queue without replacing) |
| `d` | Remove selected track from queue |
| `c` | Clear entire queue and stop playback |

#### Playlists

| Key | Action |
|-----|--------|
| `a` | Add selected track to a playlist (works from any tab) |
| `C` | Create a new playlist (on Playlists tab) |
| `D` | Delete selected playlist (on Playlists tab) |
| `d` | Remove selected track from playlist (in playlist tracks view) |

#### General

| Key | Action |
|-----|--------|
| `?` | Toggle help overlay |
| `q` | Quit |

### Playback Behavior

- **Enter** replaces the queue with all tracks in the current view (artist tracks, recent tracks, playlist tracks) starting at the selected track. This means pressing `n` will advance through the remaining tracks in order.
- **e** enqueues a single track to the end of the queue without disrupting current playback.
- The currently playing track is highlighted in green with a `♪` indicator across all tabs.

### Media Keys

Hardware media keys (from headphones, Bluetooth devices, or keyboard) are supported:

- **macOS** -- Uses the MediaPlayer framework (`MPRemoteCommandCenter`). Play/pause, next, and previous are supported. Track metadata appears in the macOS Now Playing widget.
- **Linux** -- Uses MPRIS over D-Bus. Compatible with desktop environments and media key daemons that support MPRIS.

### i3 Window Manager

i3 does not automatically route media keys to MPRIS. You need `playerctl` and a few bindings in your i3 config.

Install `playerctl`:

```bash
# Arch
sudo pacman -S playerctl

# Debian/Ubuntu
sudo apt install playerctl
```

Add to `~/.config/i3/config` (near your existing volume bindings):

```
# Media playback controls (requires playerctl)
bindsym XF86AudioPlay exec --no-startup-id playerctl play-pause
bindsym XF86AudioNext exec --no-startup-id playerctl next
bindsym XF86AudioPrev exec --no-startup-id playerctl previous
```

Reload i3 with `$mod+Shift+r` to apply.

## Server Requirements

- A [Navidrome](https://www.navidrome.org/) server (or any server implementing the [Subsonic API](http://www.subsonic.org/pages/api.jsp) at v1.16.1)
- A valid user account
- The server must be reachable over HTTP/HTTPS from the machine running the TUI

The app uses these Subsonic endpoints: `ping`, `getArtists`, `getArtist`, `getAlbum`, `search3`, `getAlbumList2`, `getPlaylists`, `getPlaylist`, `createPlaylist`, `updatePlaylist`, `deletePlaylist`, `getSong`, `stream`.

## License

MIT
