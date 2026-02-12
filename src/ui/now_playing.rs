use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, Paragraph};
use ratatui::Frame;

use crate::app::App;
use crate::client::models::format_duration;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(inner);

    // Line 1: Track info + time + volume
    let (track_info, time_info, vol_info) = if let Some(track) = app.queue.current_item() {
        let artist = track.artist_display();
        let info = if artist.is_empty() {
            format!("  Now Playing: {}", track.name)
        } else {
            format!("  Now Playing: {} - {}", track.name, artist)
        };

        let pos = format_duration(app.player_state.position);
        let dur = format_duration(app.player_state.duration);
        let pause_indicator = if app.player_state.paused { " [PAUSED]" } else { "" };
        let time = format!("[{pos}/{dur}]{pause_indicator}");

        let vol = format!("Vol: {}%", app.player_state.volume);
        (info, time, vol)
    } else {
        (
            "  No track playing".to_string(),
            String::new(),
            format!("Vol: {}%", app.player_state.volume),
        )
    };

    let info_line = Line::from(vec![
        Span::styled(
            track_info,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(time_info, Style::default().fg(Color::Yellow)),
        Span::raw("  "),
        Span::styled(vol_info, Style::default().fg(Color::Magenta)),
    ]);
    frame.render_widget(Paragraph::new(info_line), chunks[0]);

    // Line 2: Progress bar + repeat/shuffle
    let ratio = if app.player_state.duration > 0.0 {
        (app.player_state.position / app.player_state.duration).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let repeat_label = app.queue.repeat.label();
    let shuffle_label = if app.queue.shuffle { "Shuf" } else { "" };
    let mode_str = format!(" {repeat_label} {shuffle_label}");

    let progress_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(10), Constraint::Length(mode_str.len() as u16 + 1)])
        .split(chunks[1]);

    let gauge = Gauge::default()
        .ratio(ratio)
        .gauge_style(Style::default().fg(Color::Cyan).bg(Color::DarkGray));

    frame.render_widget(gauge, progress_area[0]);
    frame.render_widget(
        Paragraph::new(Span::styled(mode_str, Style::default().fg(Color::DarkGray))),
        progress_area[1],
    );
}
