pub mod help;
pub mod library;
pub mod now_playing;
pub mod playlists;
pub mod recent;
pub mod scroll;
pub mod search;

use ratatui::Frame;

use crate::app::App;

pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    let chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Length(1),  // Tab bar
            ratatui::layout::Constraint::Min(5),     // Main content
            ratatui::layout::Constraint::Length(3),  // Now playing
        ])
        .split(area);

    render_tab_bar(frame, app, chunks[0]);

    match app.active_tab {
        crate::app::Tab::Library => library::render(frame, app, chunks[1]),
        crate::app::Tab::Search => search::render(frame, app, chunks[1]),
        crate::app::Tab::Queue => library::render_queue_tab(frame, app, chunks[1]),
        crate::app::Tab::Recent => recent::render(frame, app, chunks[1]),
        crate::app::Tab::Playlists => playlists::render(frame, app, chunks[1]),
    }

    now_playing::render(frame, app, chunks[2]);

    if app.add_to_playlist_popup {
        playlists::render_add_to_playlist_popup(frame, app, area);
    }

    if app.show_help {
        help::render(frame, area);
    }
}

fn render_tab_bar(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    use ratatui::style::{Color, Modifier, Style};
    use ratatui::text::{Line, Span};
    use ratatui::widgets::Paragraph;

    let tabs = [
        ("1:Library", crate::app::Tab::Library),
        ("2:Search", crate::app::Tab::Search),
        ("3:Queue", crate::app::Tab::Queue),
        ("4:Recent", crate::app::Tab::Recent),
        ("5:Playlists", crate::app::Tab::Playlists),
    ];

    let mut spans = vec![
        Span::styled(
            " jellyfin-tui ",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
    ];

    for (label, tab) in &tabs {
        let style = if *tab == app.active_tab {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        spans.push(Span::styled(format!("[{label}]"), style));
        spans.push(Span::raw(" "));
    }

    if let Some(ref msg) = app.status_message {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(msg.clone(), Style::default().fg(Color::Red)));
    }

    let line = Line::from(spans);
    frame.render_widget(Paragraph::new(line), area);
}
