//! HUD: score / level / lines counter + next-piece preview + overlays.

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::game::piece::{PieceKind, Rotation};
use crate::game::state::{GameState, Phase};
use crate::persistence::HighScoreStore;
use crate::render::helpers::{format_level, format_lines, format_score};
use crate::render::theme::{Theme, BASE, NEW_BEST, OVERLAY, SUBTEXT, TEXT};

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
            draw_game_over_overlay(frame, area, state, is_new_best);
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

/// Draw score / level / lines stats with clean typography.
fn draw_stats(frame: &mut Frame, area: Rect, state: &GameState) {
    let label_style = Style::default().fg(SUBTEXT);
    let value_style = Style::default().fg(TEXT).add_modifier(Modifier::BOLD);

    // Use the animated rollup display value rather than the raw score.
    let displayed_score = state.score_display.current;

    let text = Text::from(vec![
        Line::from(vec![
            Span::styled("score  ", label_style),
            Span::styled(
                format_score(displayed_score).trim_start().to_string(),
                value_style,
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("level  ", label_style),
            Span::styled(
                format_level(state.level).trim_start().to_string(),
                value_style,
            ),
        ]),
        Line::from(vec![
            Span::styled("lines  ", label_style),
            Span::styled(
                format_lines(state.lines_cleared).trim_start().to_string(),
                value_style,
            ),
        ]),
    ]);
    let para = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(OVERLAY))
                .title(Span::styled(" stats ", Style::default().fg(SUBTEXT)))
                .style(Style::default().bg(BASE)),
        )
        .alignment(Alignment::Left);
    frame.render_widget(para, area);
}

/// Draw next-piece preview showing actual piece shapes for the next 3 pieces.
fn draw_next_preview(frame: &mut Frame, area: Rect, state: &GameState, theme: &Theme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(OVERLAY))
        .title(Span::styled(" next ", Style::default().fg(SUBTEXT)))
        .style(Style::default().bg(BASE));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Render up to 3 pieces as actual shapes (double-wide cells).
    let preview_count = 3usize.min(state.next_queue.len());
    // Each piece preview occupies 3 rows (2 cell rows + 1 gap), except the last.
    for (idx, &kind) in state.next_queue.iter().take(preview_count).enumerate() {
        let start_y = inner.y + (idx as u16) * 3;
        if start_y >= inner.y + inner.height {
            break;
        }
        render_piece_preview(frame, inner, start_y, kind, theme);
    }
}

/// Render a single piece's shape preview starting at `start_y` within `area`.
///
/// Uses Zero rotation. Each cell = 2 terminal cols wide, 1 row tall.
/// Pieces are rendered left-aligned within the inner area.
fn render_piece_preview(
    frame: &mut Frame,
    area: Rect,
    start_y: u16,
    kind: PieceKind,
    theme: &Theme,
) {
    use crate::game::piece::Piece;

    let piece = Piece {
        kind,
        rotation: Rotation::Zero,
        origin: (0, 0),
    };

    let color = if theme.monochrome {
        Color::Reset
    } else {
        theme.color(kind)
    };
    let glyph = theme.glyph(kind);
    let cell_text: String = if theme.monochrome {
        format!("{}{}", glyph, glyph)
    } else {
        "██".to_string()
    };
    let cell_style = if theme.monochrome {
        Style::default().fg(Color::Reset)
    } else {
        Style::default().fg(color).bg(BASE)
    };

    for (col, row) in piece.cells() {
        // col and row are offsets from origin (0,0), so bounding box coords.
        let x = area.x + (col as u16) * 2;
        let y = start_y + row as u16;
        if x + 2 > area.x + area.width || y >= area.y + area.height {
            continue;
        }
        frame.render_widget(
            Paragraph::new(Span::styled(cell_text.clone(), cell_style)),
            Rect::new(x, y, 2, 1),
        );
    }
}

/// Draw a centered "PAUSED" overlay modal.
pub fn draw_pause_overlay(frame: &mut Frame, area: Rect) {
    let overlay_w = 18u16.min(area.width);
    let overlay_h = 7u16.min(area.height);
    let x = area.x + area.width.saturating_sub(overlay_w) / 2;
    let y = area.y + area.height.saturating_sub(overlay_h) / 2;
    let overlay_area = Rect::new(x, y, overlay_w, overlay_h);

    frame.render_widget(Clear, overlay_area);

    let text = Text::from(vec![
        Line::from(""),
        Line::from(Span::styled(
            "PAUSED",
            Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("p ", Style::default().fg(OVERLAY)),
            Span::styled("resume", Style::default().fg(SUBTEXT)),
        ]),
        Line::from(vec![
            Span::styled("q ", Style::default().fg(OVERLAY)),
            Span::styled("quit", Style::default().fg(SUBTEXT)),
        ]),
    ]);
    let para = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(OVERLAY))
                .style(Style::default().bg(BASE)),
        )
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: false });
    frame.render_widget(para, overlay_area);
}

/// Draw a centered "GAME OVER" overlay modal.
///
/// `state` provides the final score, level, and lines for the summary.
/// `new_best` triggers a "NEW BEST!" banner in the highlight color.
///
/// If `state.gameover_zoom` is active the overlay zooms in from 50 % to 100 %
/// over 200 ms using inner-padding shrinkage.
pub fn draw_game_over_overlay(frame: &mut Frame, area: Rect, state: &GameState, new_best: bool) {
    // 11 base lines (9 content + 2 rounded border) + 2 for NEW BEST.
    let full_h = if new_best { 13u16 } else { 11u16 };
    let full_w = 20u16;

    // Compute scale from the zoom animation.
    let scale = state
        .gameover_zoom
        .as_ref()
        .map(|z| z.scale(state.now()))
        .unwrap_or(1.0);

    // Apply scale to width; height uses full so text doesn't clip.
    let overlay_w = ((full_w as f32 * scale) as u16).max(4).min(area.width);
    let overlay_h = full_h.min(area.height);
    let x = area.x + area.width.saturating_sub(overlay_w) / 2;
    let y = area.y + area.height.saturating_sub(overlay_h) / 2;
    let overlay_area = Rect::new(x, y, overlay_w, overlay_h);

    frame.render_widget(Clear, overlay_area);

    let mut lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Game Over",
            Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
        )),
    ];

    if new_best {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "NEW BEST!",
            Style::default().fg(NEW_BEST).add_modifier(Modifier::BOLD),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("score  ", Style::default().fg(SUBTEXT)),
        Span::styled(
            format_score(state.score).trim_start().to_string(),
            Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("level  ", Style::default().fg(SUBTEXT)),
        Span::styled(
            format_level(state.level).trim_start().to_string(),
            Style::default().fg(TEXT),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("lines  ", Style::default().fg(SUBTEXT)),
        Span::styled(
            format_lines(state.lines_cleared).trim_start().to_string(),
            Style::default().fg(TEXT),
        ),
    ]));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("r ", Style::default().fg(OVERLAY)),
        Span::styled("restart", Style::default().fg(SUBTEXT)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("q ", Style::default().fg(OVERLAY)),
        Span::styled("quit", Style::default().fg(SUBTEXT)),
    ]));

    let text = Text::from(lines);
    let para = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(OVERLAY))
                .title(Span::styled(" game over ", Style::default().fg(OVERLAY)))
                .style(Style::default().bg(BASE)),
        )
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: false });
    frame.render_widget(para, overlay_area);
}
