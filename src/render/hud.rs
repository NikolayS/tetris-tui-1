//! HUD: score / level / lines counter + next-piece preview + overlays.

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::game::state::{GameState, Phase};
use crate::persistence::HighScoreStore;
use crate::render::helpers::{format_level, format_lines, format_score, next_preview_glyphs};
use crate::render::theme::Theme;

/// Draw the HUD panel (score, level, lines, next-piece preview) into `area`.
///
/// `high_scores` is optional: when `Some`, the game-over overlay compares
/// the final score against the current best and shows a "NEW BEST!" banner
/// when applicable. Pass `None` to suppress the comparison (the overlay is
/// still rendered, but without the new-best highlight).
///
/// Also draws pause or game-over overlays when appropriate.
pub fn draw(frame: &mut Frame, area: Rect, state: &GameState, theme: &Theme) {
    draw_with_scores(frame, area, state, theme, None);
}

/// Like `draw`, but accepts an optional `HighScoreStore` for the new-best
/// overlay. Prefer this entry point from the main loop once persistence is
/// wired (issue #31).
pub fn draw_with_scores(
    frame: &mut Frame,
    area: Rect,
    state: &GameState,
    theme: &Theme,
    high_scores: Option<&HighScoreStore>,
) {
    // Split into sections: stats on top, next preview below.
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7), // stats block
            Constraint::Min(8),    // next preview
        ])
        .split(area);

    draw_stats(frame, chunks[0], state);
    draw_next_preview(frame, chunks[1], state, theme);

    // Overlays (drawn last so they appear on top).
    match &state.phase {
        Phase::Paused => draw_pause_overlay(frame, area),
        Phase::GameOver { .. } => {
            let is_new_best = is_new_best(state.score, high_scores);
            draw_game_over_overlay(frame, area, state.score, is_new_best);
        }
        Phase::Playing => {}
    }
}

/// Returns `true` if `score` beats the current top score in `store`.
///
/// - If `store` is `None` → `false` (no comparison available).
/// - If store is empty (no previous scores) → `true` (any score is a new best).
pub fn is_new_best(score: u32, store: Option<&HighScoreStore>) -> bool {
    match store {
        None => false,
        Some(hs) => {
            let top = hs.top(1);
            if top.is_empty() {
                // No previous scores — every score is a personal best.
                true
            } else {
                score > top[0].score
            }
        }
    }
}

/// Draw score / level / lines stats.
fn draw_stats(frame: &mut Frame, area: Rect, state: &GameState) {
    let text = Text::from(vec![
        Line::from(vec![Span::styled(
            "SCORE",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from(Span::raw(format_score(state.score))),
        Line::from(""),
        Line::from(vec![Span::styled(
            "LEVEL",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from(Span::raw(format_level(state.level))),
        Line::from(vec![Span::styled(
            "LINES",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from(Span::raw(format_lines(state.lines_cleared))),
    ]);
    let para = Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("Stats"));
    frame.render_widget(para, area);
}

/// Draw next-piece preview (up to 5 pieces).
fn draw_next_preview(frame: &mut Frame, area: Rect, state: &GameState, theme: &Theme) {
    let mut lines: Vec<Line> = vec![Line::from(Span::styled(
        "NEXT",
        Style::default().add_modifier(Modifier::BOLD),
    ))];

    for (kind, glyph) in next_preview_glyphs(&state.next_queue, theme).take(5) {
        let color = if theme.monochrome {
            Color::Reset
        } else {
            theme.color(kind)
        };
        let s = format!(" {} {}", glyph, glyph);
        lines.push(Line::from(Span::styled(s, Style::default().fg(color))));
    }

    let para = Paragraph::new(Text::from(lines))
        .block(Block::default().borders(Borders::ALL).title("Next"));
    frame.render_widget(para, area);
}

/// Draw a centered "PAUSED" overlay.
pub fn draw_pause_overlay(frame: &mut Frame, area: Rect) {
    let overlay_w = 16u16.min(area.width);
    let overlay_h = 4u16.min(area.height);
    let x = area.x + area.width.saturating_sub(overlay_w) / 2;
    let y = area.y + area.height.saturating_sub(overlay_h) / 2;
    let overlay_area = Rect::new(x, y, overlay_w, overlay_h);

    frame.render_widget(Clear, overlay_area);

    let text = Text::from(vec![
        Line::from(""),
        Line::from(Span::styled(
            "  PAUSED  ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED),
        )),
        Line::from(Span::raw("  p to resume")),
    ]);
    let para = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: false });
    frame.render_widget(para, overlay_area);
}

/// Draw a centered "GAME OVER" overlay.
///
/// `score` is the final score shown in the overlay.
/// `new_best` triggers a "NEW BEST!" banner and highlighted score line.
pub fn draw_game_over_overlay(frame: &mut Frame, area: Rect, score: u32, new_best: bool) {
    // Extra line for NEW BEST banner when applicable.
    let overlay_h = if new_best { 7u16 } else { 5u16 }.min(area.height);
    let overlay_w = 22u16.min(area.width);
    let x = area.x + area.width.saturating_sub(overlay_w) / 2;
    let y = area.y + area.height.saturating_sub(overlay_h) / 2;
    let overlay_area = Rect::new(x, y, overlay_w, overlay_h);

    frame.render_widget(Clear, overlay_area);

    let score_str = format_score(score);
    let score_style = if new_best {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            " GAME OVER ",
            Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED),
        )),
    ];

    if new_best {
        lines.push(Line::from(Span::styled(
            " NEW BEST! ",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED),
        )));
        lines.push(Line::from(Span::styled(score_str, score_style)));
    }

    lines.push(Line::from(Span::raw("  r: restart")));
    lines.push(Line::from(Span::raw("  q: quit")));

    let text = Text::from(lines);
    let para = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: false });
    frame.render_widget(para, overlay_area);
}
