use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

const HELP_TEXT: &[(&str, &str)] = &[
    ("Navigation", ""),
    ("j / k", "Move down / up"),
    ("g / G", "Jump to top / bottom"),
    ("Ctrl+d / Ctrl+u", "Half-page down / up"),
    ("h / l", "Back / drill-in (Artist→Album→Track)"),
    ("Tab / Shift+Tab", "Cycle focus"),
    ("1 / 2 / 3 / 4 / 5", "Library / Search / Queue / Recent / Playlists"),
    ("", ""),
    ("Playback", ""),
    ("Enter", "Play selected track"),
    ("Space", "Toggle pause"),
    ("n / N", "Next / previous track"),
    ("> / <", "Seek forward / backward 10s"),
    ("+ / -", "Volume up / down"),
    ("", ""),
    ("Actions", ""),
    ("e", "Enqueue selected track(s)"),
    ("v", "Visual select mode"),
    ("a", "Add to playlist (visual: add all)"),
    ("/", "Filter list / search playlist"),
    ("Esc", "Clear filter / close popup"),
    ("r", "Cycle repeat mode"),
    ("s", "Toggle shuffle"),
    ("d", "Remove from queue / playlist"),
    ("c", "Clear queue"),
    ("C", "Create playlist"),
    ("D", "Delete playlist"),
    ("", ""),
    ("Favorites & Ratings", ""),
    ("f", "Toggle favorite (selected or now-playing)"),
    ("Ctrl+1..5", "Set rating 1–5"),
    ("`", "Clear rating"),
    ("", ""),
    ("q", "Quit"),
    ("?", "Toggle this help"),
];

pub fn render(frame: &mut Frame, area: Rect) {
    let popup_width = 56u16.min(area.width.saturating_sub(4));
    let popup_height = (HELP_TEXT.len() as u16 + 2).min(area.height.saturating_sub(4));

    let [popup_area] = Layout::horizontal([Constraint::Length(popup_width)])
        .flex(Flex::Center)
        .areas(area);
    let [popup_area] = Layout::vertical([Constraint::Length(popup_height)])
        .flex(Flex::Center)
        .areas(popup_area);

    frame.render_widget(Clear, popup_area);

    let lines: Vec<Line> = HELP_TEXT
        .iter()
        .map(|(key, desc)| {
            if desc.is_empty() && !key.is_empty() {
                // Section header
                Line::from(Span::styled(
                    format!(" {key}"),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ))
            } else if key.is_empty() {
                Line::raw("")
            } else {
                Line::from(vec![
                    Span::styled(
                        format!("  {key:<20}"),
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(desc.to_string()),
                ])
            }
        })
        .collect();

    let help = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" Help (?) "),
    );

    frame.render_widget(help, popup_area);
}
