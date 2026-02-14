use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthChar;

/// Computes the scroll offset for a marquee effect.
///
/// - Initial pause: 10 ticks (1s) at offset 0
/// - Scrolling: 3 ticks (300ms) per character
/// - End pause: 10 ticks (1s) at max_offset
/// - Then cycles back
pub fn compute_scroll_offset(tick: u16, max_offset: usize) -> usize {
    if max_offset == 0 {
        return 0;
    }

    let initial_pause = 8u16;
    let scroll_ticks = (max_offset as u16) * 2;
    let end_pause = 8u16;
    let cycle_len = initial_pause + scroll_ticks + end_pause;

    let t = tick % cycle_len;

    if t < initial_pause {
        0
    } else if t < initial_pause + scroll_ticks {
        ((t - initial_pause) / 2) as usize
    } else {
        max_offset
    }
}

/// Scrolls a `Line` horizontally by trimming characters from the left when it overflows `width`.
///
/// If the line fits within `width`, it is returned unchanged.
/// Otherwise, uses `tick` to compute a scroll offset and returns a new `Line`
/// with spans shifted left, preserving per-span styles.
pub fn scroll_line<'a>(line: Line<'a>, width: u16, tick: u16) -> Line<'a> {
    let line_width = line_display_width(&line);
    let available = width as usize;

    if line_width <= available {
        return line;
    }

    let max_offset = line_width - available;
    let offset = compute_scroll_offset(tick, max_offset);

    slice_line(line, offset, available)
}

/// Measures the display width of a Line by summing span character widths.
fn line_display_width(line: &Line<'_>) -> usize {
    line.spans.iter().map(|s| span_width(s)).sum()
}

/// Measures the display width of a Span.
fn span_width(span: &Span<'_>) -> usize {
    span.content.chars().map(|c| c.width().unwrap_or(0)).sum()
}

/// Slices a Line starting at display column `offset`, taking up to `max_width` display columns.
/// Preserves per-span styles.
fn slice_line<'a>(line: Line<'a>, offset: usize, max_width: usize) -> Line<'a> {
    let mut result_spans: Vec<Span<'a>> = Vec::new();
    let mut skipped = 0usize;
    let mut taken = 0usize;

    for span in line.spans {
        if taken >= max_width {
            break;
        }

        let sw = span_width(&span);

        // Entirely before the visible window
        if skipped + sw <= offset {
            skipped += sw;
            continue;
        }

        // How many display columns to skip within this span
        let skip_in_span = if skipped < offset { offset - skipped } else { 0 };
        let remaining_capacity = max_width - taken;

        let sliced = slice_span_content(&span.content, skip_in_span, remaining_capacity);
        let sliced_width: usize = sliced.chars().map(|c| c.width().unwrap_or(0)).sum();

        if sliced_width > 0 {
            result_spans.push(Span::styled(sliced, span.style));
            taken += sliced_width;
        }

        skipped += sw;
    }

    Line::from(result_spans)
}

/// Slices a string by display width: skips `skip` display columns, then takes up to `take` columns.
fn slice_span_content(content: &str, skip: usize, take: usize) -> String {
    let mut result = String::new();
    let mut skipped = 0usize;
    let mut taken = 0usize;

    for ch in content.chars() {
        let w = ch.width().unwrap_or(0);

        if skipped < skip {
            skipped += w;
            continue;
        }

        if taken + w > take {
            break;
        }

        result.push(ch);
        taken += w;
    }

    result
}
