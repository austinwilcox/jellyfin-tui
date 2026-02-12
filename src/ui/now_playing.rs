use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::App;
use crate::client::models::format_duration;

/// Build a volume bar string: `████████░░` (10 chars)
fn volume_bar(pct: i64) -> String {
    let filled = ((pct.clamp(0, 100) as f64 / 100.0) * 10.0).round() as usize;
    let empty = 10 - filled;
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let repeat_label = app.queue.repeat.label();
    let shuffle_label = if app.queue.shuffle { "Shuf" } else { "" };

    let mut spans: Vec<Span> = if let Some(track) = app.queue.current_item() {
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

        vec![
            Span::styled(
                info,
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(time, Style::default().fg(Color::Yellow)),
        ]
    } else {
        vec![Span::styled(
            "  No track playing",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )]
    };

    // Volume
    let vol_bar = volume_bar(app.player_state.volume);
    spans.push(Span::raw("  "));
    spans.push(Span::styled("Vol ", Style::default().fg(Color::Magenta)));
    spans.push(Span::styled(vol_bar, Style::default().fg(Color::Magenta)));
    spans.push(Span::styled(
        format!(" {}%", app.player_state.volume),
        Style::default().fg(Color::Magenta),
    ));

    if let Some(sys_vol) = app.player_state.system_volume {
        let sys_bar = volume_bar(sys_vol);
        spans.push(Span::styled("  Sys ", Style::default().fg(Color::Magenta)));
        spans.push(Span::styled(sys_bar, Style::default().fg(Color::Magenta)));
        spans.push(Span::styled(
            format!(" {}%", sys_vol),
            Style::default().fg(Color::Magenta),
        ));
    }

    // Repeat / Shuffle
    spans.push(Span::raw("  "));
    spans.push(Span::styled(repeat_label, Style::default().fg(Color::DarkGray)));
    if !shuffle_label.is_empty() {
        spans.push(Span::raw(" "));
        spans.push(Span::styled(shuffle_label, Style::default().fg(Color::DarkGray)));
    }

    let line = Line::from(spans);
    frame.render_widget(Paragraph::new(line), inner);
}
