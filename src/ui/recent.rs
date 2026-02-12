use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};
use ratatui::Frame;

use crate::app::App;

pub fn render(frame: &mut Frame, app: &mut App, area: Rect) {
    let visual_range = app.visual_selection_range();
    let items: Vec<ListItem> = app
        .recent_tracks
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let is_playing = app
                .queue
                .current_item()
                .map(|c| c.id == item.id)
                .unwrap_or(false);
            let is_visual = visual_range
                .map(|(s, e)| i >= s && i <= e)
                .unwrap_or(false);

            let prefix = if is_playing { "♪ " } else { "  " };
            let artists = item
                .artists
                .as_ref()
                .map(|a| a.join(", "))
                .or(item.album_artist.clone())
                .unwrap_or_default();
            let album = item.album.as_deref().unwrap_or("");
            let genre = item
                .genres
                .as_ref()
                .map(|g| g.join(", "))
                .unwrap_or_default();
            let dur = item.duration_display();
            let date_added = item
                .date_created
                .as_deref()
                .and_then(|d| d.get(..10))
                .unwrap_or("");

            let (style, dim) = if is_playing {
                let s = Style::default().fg(Color::Green).add_modifier(Modifier::BOLD);
                (s, s)
            } else if is_visual {
                let s = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
                (s, s)
            } else {
                (Style::default().add_modifier(Modifier::BOLD), Style::default().fg(Color::DarkGray))
            };

            let prefix_style = if is_playing {
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
            } else if is_visual {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let mut spans = vec![
                Span::styled(prefix.to_string(), prefix_style),
                Span::styled(item.name.clone(), style),
                Span::styled(format!(" - {artists}"), dim),
                Span::styled(format!("  {album}"), dim),
            ];
            if !genre.is_empty() {
                spans.push(Span::styled(format!("  [{genre}]"), dim));
            }
            spans.push(Span::styled(format!("  {dur}"), dim));
            if !date_added.is_empty() {
                spans.push(Span::styled(format!("  {date_added}"), dim));
            }

            ListItem::new(Line::from(spans))
        })
        .collect();

    let count = app.recent_tracks.len();
    let title = if let Some((s, e)) = visual_range {
        format!(" Recent ({count}) -- VISUAL {} selected ", e - s + 1)
    } else {
        format!(" Recent ({count}) ")
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(title),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    frame.render_stateful_widget(list, area, &mut app.recent_state);
}
