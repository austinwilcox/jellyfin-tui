use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};
use ratatui::Frame;

use crate::app::{App, Focus};

pub fn render(frame: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(35),
            Constraint::Percentage(25),
        ])
        .split(area);

    render_artists(frame, app, chunks[0]);
    render_albums(frame, app, chunks[1]);
    render_tracks(frame, app, chunks[2]);
    render_queue_panel(frame, app, chunks[3]);
}

fn render_artists(frame: &mut Frame, app: &mut App, area: Rect) {
    let focused = app.focus == Focus::Artists;
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let items: Vec<ListItem> = app
        .filtered_artists()
        .iter()
        .map(|a| {
            let style = if Some(&a.id) == app.selected_artist_id().as_ref() {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(Span::styled(a.name.clone(), style))
        })
        .collect();

    let title = if app.filter_active && app.focus == Focus::Artists {
        format!(" Artists [/{}] ", app.filter_text)
    } else {
        format!(" Artists ({}) ", app.artists.len())
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

    frame.render_stateful_widget(list, area, &mut app.artist_state);
}

fn render_albums(frame: &mut Frame, app: &mut App, area: Rect) {
    let focused = app.focus == Focus::Albums;
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let mut items: Vec<ListItem> = Vec::new();

    // "All Tracks" entry at index 0
    if app.selected_artist_name.is_some() {
        items.push(ListItem::new(Span::styled(
            "♪ All Tracks",
            Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
        )));
    }

    // Album entries
    for album in &app.albums {
        let year = album
            .production_year
            .map(|y| format!(" ({y})"))
            .unwrap_or_default();
        let style = if app.selected_album_name.as_deref() == Some(&album.name) {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        items.push(ListItem::new(Line::from(vec![
            Span::styled(album.name.clone(), style),
            Span::styled(year, Style::default().fg(Color::DarkGray)),
        ])));
    }

    let title = if let Some(ref name) = app.selected_artist_name {
        format!(" Albums - {name} ({}) ", app.albums.len())
    } else {
        " Albums ".to_string()
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

    frame.render_stateful_widget(list, area, &mut app.album_state);
}

fn render_tracks(frame: &mut Frame, app: &mut App, area: Rect) {
    let focused = app.focus == Focus::Tracks;
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let visual_range = if focused { app.visual_selection_range() } else { None };
    let items: Vec<ListItem> = app
        .tracks
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
            let num = t.index_number.unwrap_or((i + 1) as u32);
            let dur = t.duration_display();
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
                Span::raw("  "),
                Span::styled(album.to_string(), dim),
                Span::raw("  "),
                Span::styled(dur, dim),
            ]);
            ListItem::new(line)
        })
        .collect();

    let title = if let Some((s, e)) = visual_range {
        format!(" Tracks -- VISUAL {} selected ", e - s + 1)
    } else if let Some(ref album_name) = app.selected_album_name {
        if album_name == "All Tracks" {
            if let Some(ref artist) = app.selected_artist_name {
                format!(" {artist} - All Tracks ({}) ", app.tracks.len())
            } else {
                format!(" All Tracks ({}) ", app.tracks.len())
            }
        } else {
            format!(" {album_name} ({}) ", app.tracks.len())
        }
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

    frame.render_stateful_widget(list, area, &mut app.track_state);
}

fn render_queue_panel(frame: &mut Frame, app: &mut App, area: Rect) {
    render_queue_list(frame, app, area, app.focus == Focus::QueuePanel);
}

pub fn render_queue_tab(frame: &mut Frame, app: &mut App, area: Rect) {
    render_queue_list(frame, app, area, true);
}

fn render_queue_list(frame: &mut Frame, app: &mut App, area: Rect, focused: bool) {
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let items: Vec<ListItem> = app
        .queue
        .items
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let is_current = app.queue.current == Some(i);
            let prefix = if is_current { "▶ " } else { "  " };

            let style = if is_current {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let dur = t.duration_display();
            let line = Line::from(vec![
                Span::styled(format!("{prefix}{}. ", i + 1), style),
                Span::styled(t.name.clone(), style),
                Span::raw("  "),
                Span::styled(dur, Style::default().fg(Color::DarkGray)),
            ]);
            ListItem::new(line)
        })
        .collect();

    let repeat_label = app.queue.repeat.label();
    let shuffle_label = if app.queue.shuffle { "On" } else { "Off" };
    let title = format!(
        " Queue ({}) | Repeat: {} | Shuffle: {} ",
        app.queue.items.len(),
        repeat_label,
        shuffle_label,
    );

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

    frame.render_stateful_widget(list, area, &mut app.queue_state);
}
