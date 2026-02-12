use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, Paragraph, Widget};
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

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Track info + volume
            Constraint::Length(1), // Progress bar
            Constraint::Min(0),   // Visualizer (remaining space)
        ])
        .split(inner);

    // Line 1: Track info + time + visual volume bars
    let (track_info, time_info, vol_spans) = if let Some(track) = app.queue.current_item() {
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

        let vol_bar = volume_bar(app.player_state.volume);
        let mut spans = vec![
            Span::styled("Vol ", Style::default().fg(Color::Magenta)),
            Span::styled(vol_bar, Style::default().fg(Color::Magenta)),
            Span::styled(
                format!(" {}%", app.player_state.volume),
                Style::default().fg(Color::Magenta),
            ),
        ];

        if let Some(sys_vol) = app.player_state.system_volume {
            let sys_bar = volume_bar(sys_vol);
            spans.push(Span::styled("  Sys ", Style::default().fg(Color::Magenta)));
            spans.push(Span::styled(sys_bar, Style::default().fg(Color::Magenta)));
            spans.push(Span::styled(
                format!(" {}%", sys_vol),
                Style::default().fg(Color::Magenta),
            ));
        }

        (info, time, spans)
    } else {
        let vol_bar = volume_bar(app.player_state.volume);
        let mut spans = vec![
            Span::styled("Vol ", Style::default().fg(Color::Magenta)),
            Span::styled(vol_bar, Style::default().fg(Color::Magenta)),
            Span::styled(
                format!(" {}%", app.player_state.volume),
                Style::default().fg(Color::Magenta),
            ),
        ];

        if let Some(sys_vol) = app.player_state.system_volume {
            let sys_bar = volume_bar(sys_vol);
            spans.push(Span::styled("  Sys ", Style::default().fg(Color::Magenta)));
            spans.push(Span::styled(sys_bar, Style::default().fg(Color::Magenta)));
            spans.push(Span::styled(
                format!(" {}%", sys_vol),
                Style::default().fg(Color::Magenta),
            ));
        }

        ("  No track playing".to_string(), String::new(), spans)
    };

    let mut info_spans = vec![
        Span::styled(
            track_info,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(time_info, Style::default().fg(Color::Yellow)),
        Span::raw("  "),
    ];
    info_spans.extend(vol_spans);

    let info_line = Line::from(info_spans);
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

    // Visualizer rows
    let vis_area = chunks[2];
    if vis_area.height > 0 && vis_area.width > 0 {
        frame.render_widget(Visualizer { bars: &app.visualizer_bars }, vis_area);
    }
}

/// Block characters for 8 levels within a single row (index 0 = empty space)
const BLOCK_CHARS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

struct Visualizer<'a> {
    bars: &'a [f64],
}

impl Widget for Visualizer<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let width = area.width as usize;
        let rows = area.height as usize;
        if rows == 0 || width == 0 {
            return;
        }

        // Total levels = rows * 8 (8 block chars per row)
        let total_levels = rows * 8;

        for col in 0..width {
            // Map bar index to available bars (wrap/repeat if fewer bars than columns)
            let bar_val = if !self.bars.is_empty() {
                self.bars[col % self.bars.len()]
            } else {
                0.0
            };

            let level = (bar_val * total_levels as f64).round() as usize;

            // Render from bottom row upward
            for row in 0..rows {
                let y = area.y + (rows - 1 - row) as u16;
                let x = area.x + col as u16;

                // How many levels have been filled by rows below this one
                let filled_below = row * 8;
                let remaining = level.saturating_sub(filled_below);

                if remaining == 0 {
                    // Empty — just space
                    buf[(x, y)].set_char(' ');
                } else if remaining >= 8 {
                    // Full block
                    buf[(x, y)]
                        .set_char('█')
                        .set_fg(Color::Cyan);
                } else {
                    // Partial block (1-7)
                    buf[(x, y)]
                        .set_char(BLOCK_CHARS[remaining - 1])
                        .set_fg(Color::Cyan);
                }
            }
        }
    }
}
