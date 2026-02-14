use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::app::{App, Focus};
use crate::ui::scroll;

pub fn render(frame: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(area);

    render_playlist_list(frame, app, chunks[0]);
    render_playlist_tracks(frame, app, chunks[1]);
}

fn render_playlist_list(frame: &mut Frame, app: &mut App, area: Rect) {
    let focused = app.focus == Focus::Playlists;
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let selected_idx = app.playlist_state.selected();
    let available_width = area.width.saturating_sub(4);
    let items: Vec<ListItem> = app
        .playlists
        .iter()
        .enumerate()
        .map(|(i, pl)| {
            let style = if app.selected_playlist_id.as_deref() == Some(&pl.id) {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let line = Line::from(Span::styled(pl.name.clone(), style));
            if focused && selected_idx == Some(i) {
                ListItem::new(scroll::scroll_line(line, available_width, app.scroll_tick))
            } else {
                ListItem::new(line)
            }
        })
        .collect();

    let title = if app.playlist_create_mode {
        format!(" New Playlist: {}_ ", app.playlist_create_name)
    } else {
        format!(" Playlists ({}) ", app.playlists.len())
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(title),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    frame.render_stateful_widget(list, area, &mut app.playlist_state);
}

fn render_playlist_tracks(frame: &mut Frame, app: &mut App, area: Rect) {
    let focused = app.focus == Focus::PlaylistTracks;
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let selected_idx = app.playlist_track_state.selected();
    let available_width = area.width.saturating_sub(4);
    let visual_range = if focused { app.visual_selection_range() } else { None };
    let items: Vec<ListItem> = app
        .playlist_tracks
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let is_playing = app
                .queue
                .current_item()
                .map(|c| c.id == t.id)
                .unwrap_or(false);
            let is_visual = visual_range
                .map(|(s, e)| i >= s && i <= e)
                .unwrap_or(false);

            let prefix = if is_playing { "♪ " } else { "  " };
            let num = i + 1;
            let dur = t.duration_display();
            let artist = t.artist_display();
            let album = t.album.as_deref().unwrap_or("");

            let (style, dim) = if is_playing {
                let s = Style::default().fg(Color::Green).add_modifier(Modifier::BOLD);
                (s, s)
            } else if is_visual {
                let s = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
                (s, s)
            } else {
                (Style::default(), Style::default().fg(Color::DarkGray))
            };

            let line = Line::from(vec![
                Span::styled(format!("{prefix}{num:>2}. "), style),
                Span::styled(t.name.clone(), style),
                Span::styled(format!(" - {artist}"), dim),
                Span::styled(format!("  {album}"), dim),
                Span::styled(format!("  {dur}"), dim),
            ]);
            if focused && selected_idx == Some(i) {
                ListItem::new(scroll::scroll_line(line, available_width, app.scroll_tick))
            } else {
                ListItem::new(line)
            }
        })
        .collect();

    let title = if let Some((s, e)) = visual_range {
        format!(" Tracks -- VISUAL {} selected ", e - s + 1)
    } else if let Some(ref name) = app.selected_playlist_name {
        format!(" {name} ({}) ", app.playlist_tracks.len())
    } else {
        " Tracks ".to_string()
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(title),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    frame.render_stateful_widget(list, area, &mut app.playlist_track_state);
}

pub fn render_add_to_playlist_popup(frame: &mut Frame, app: &mut App, area: Rect) {
    use ratatui::layout::{Constraint, Flex, Layout};
    use ratatui::widgets::Clear;

    let count = app.add_to_playlist_items.len();
    let title = if count > 1 {
        format!(" Add {count} tracks to Playlist ")
    } else {
        " Add to Playlist ".to_string()
    };

    let popup_width = 40u16.min(area.width.saturating_sub(4));
    let popup_height = (app.playlists.len() as u16 + 2).min(area.height.saturating_sub(4)).max(4);

    let [popup_area] = Layout::horizontal([Constraint::Length(popup_width)])
        .flex(Flex::Center)
        .areas(area);
    let [popup_area] = Layout::vertical([Constraint::Length(popup_height)])
        .flex(Flex::Center)
        .areas(popup_area);

    frame.render_widget(Clear, popup_area);

    if app.playlists.is_empty() {
        let msg = Paragraph::new(" No playlists. Press C on Playlists tab to create one.")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow))
                    .title(title),
            );
        frame.render_widget(msg, popup_area);
        return;
    }

    let items: Vec<ListItem> = app
        .playlists
        .iter()
        .map(|pl| ListItem::new(Span::raw(format!("  {}", pl.name))))
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title(title),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    frame.render_stateful_widget(list, popup_area, &mut app.add_to_playlist_state);
}
