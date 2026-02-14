use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::app::App;
use crate::ui::scroll;

pub fn render(frame: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(5)])
        .split(area);

    // Search input
    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Search ");

    let cursor = if app.search_focused { "_" } else { "" };
    let input_text = Paragraph::new(format!(" {}{}", app.search_query, cursor))
        .block(input_block);
    frame.render_widget(input_text, chunks[0]);

    // Search results
    let selected_idx = app.search_state.selected();
    let focused = !app.search_focused;
    let available_width = chunks[1].width.saturating_sub(4);
    let items: Vec<ListItem> = app
        .search_results
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let type_label = match item.item_type.as_deref() {
                Some("MusicArtist") => "[Artist]",
                Some("MusicAlbum") => "[Album] ",
                Some("Audio") => "[Track] ",
                _ => "[?]     ",
            };

            let extra = match item.item_type.as_deref() {
                Some("Audio") => {
                    let artist = item.artist_display();
                    let dur = item.duration_display();
                    format!(" - {artist}  {dur}")
                }
                Some("MusicAlbum") => {
                    let artist = item.album_artist.as_deref().unwrap_or("");
                    let year = item
                        .production_year
                        .map(|y| format!(" ({y})"))
                        .unwrap_or_default();
                    format!(" - {artist}{year}")
                }
                _ => String::new(),
            };

            let line = Line::from(vec![
                Span::styled(
                    type_label.to_string(),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw(" "),
                Span::styled(
                    item.name.clone(),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::styled(extra, Style::default().fg(Color::DarkGray)),
            ]);
            if focused && selected_idx == Some(i) {
                ListItem::new(scroll::scroll_line(line, available_width, app.scroll_tick))
            } else {
                ListItem::new(line)
            }
        })
        .collect();

    let result_count = app.search_results.len();
    let title = if app.search_query.is_empty() {
        " Results ".to_string()
    } else {
        format!(" Results ({result_count}) ")
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

    frame.render_stateful_widget(list, chunks[1], &mut app.search_state);
}
