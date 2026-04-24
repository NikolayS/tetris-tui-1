//! Board view: draws the 10×20 visible playfield, active piece, and ghost.

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, BorderType, Borders};
use ratatui::Frame;

use crate::game::piece::Piece;
use crate::game::state::{GameState, LineClearPhase, SPAWN_FADE1_MS, SPAWN_FADE_TOTAL_MS};
use crate::render::helpers::ghost_y;
use crate::render::theme::{Theme, BASE, MANTLE, OVERLAY};

/// Compute the spawn-fade intensity multiplier (0.0–1.0) for the active piece.
///
/// - 0–40 ms: 0.60 (60 % intensity)
/// - 40–80 ms: 0.80 (80 % intensity)
/// - ≥ 80 ms / no anim: 1.00 (full intensity)
fn spawn_fade_factor(state: &GameState) -> f32 {
    use std::time::Duration;
    let Some(ref sa) = state.spawn_anim else {
        return 1.0;
    };
    let now = state.now();
    let elapsed = now.saturating_duration_since(sa.started_at);
    if elapsed >= Duration::from_millis(SPAWN_FADE_TOTAL_MS) {
        1.0
    } else if elapsed >= Duration::from_millis(SPAWN_FADE1_MS) {
        0.8
    } else {
        0.6
    }
}

/// Dim an RGB color by `factor` (0.0–1.0) by scaling each channel.
fn dim_color(c: Color, factor: f32) -> Color {
    match c {
        Color::Rgb(r, g, b) => Color::Rgb(
            (r as f32 * factor) as u8,
            (g as f32 * factor) as u8,
            (b as f32 * factor) as u8,
        ),
        other => other, // non-RGB colors pass through unchanged
    }
}

/// Width of one cell in terminal columns (each cell is 2 chars wide).
const CELL_W: u16 = 2;
/// Height of one cell in terminal rows.
const CELL_H: u16 = 1;

/// Number of visible rows (rows 20..40 of the 40-row playfield).
const VISIBLE_ROWS: i32 = 20;
/// Number of columns.
const COLS: i32 = 10;

/// Filled-cell glyph pair (two Unicode full-block chars).
const FILLED: &str = "██";
/// Ghost-cell glyph pair (two light-shade block chars).
const GHOST: &str = "░░";
/// Empty-cell glyph pair (two spaces).
const EMPTY: &str = "  ";

/// Draw the visible 10×20 playfield into `area`.
///
/// Renders the board inside a rounded border. Inside:
///   1. Locked board cells (rows 20..40).
///      During a line-clear animation, full rows are highlighted:
///      - Flash phase: bright white Rgb(255,255,255) + BOLD (eye-catching pop).
///      - Dim phase: DIM modifier.
///   2. Ghost piece (░░, dimmed) at the drop position.
///   3. Active piece (██) at its current position.
///
/// Takes `&GameState` and `&Theme` — no mutation.
pub fn draw(frame: &mut Frame, area: Rect, state: &GameState, theme: &Theme) {
    // Draw the bordered playfield container.
    let board_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(OVERLAY))
        .style(Style::default().bg(BASE));
    let inner = board_block.inner(area);
    frame.render_widget(board_block, area);

    // Build the set of animated row indices (absolute board rows), if any.
    let anim_rows: Option<(&[usize], &LineClearPhase)> = state
        .line_clear_anim
        .as_ref()
        .map(|a| (a.rows.as_slice(), &a.phase));

    // 1. Draw background dots / empty cells.
    for vis_row in 0..VISIBLE_ROWS {
        for col in 0..COLS {
            let x = inner.x + (col as u16) * CELL_W;
            let y = inner.y + vis_row as u16 * CELL_H;
            if x + CELL_W <= inner.x + inner.width && y < inner.y + inner.height {
                frame.render_widget(
                    ratatui::widgets::Paragraph::new(Span::styled(
                        EMPTY,
                        Style::default().bg(MANTLE),
                    )),
                    Rect::new(x, y, CELL_W, CELL_H),
                );
            }
        }
    }

    // 2. Locked board cells (rows 20..40).
    for vis_row in 0..VISIBLE_ROWS {
        let board_row = (vis_row + 20) as usize;

        // Determine if this row is being animated.
        // Flash pop: full-intensity bright white with BOLD (eye-catching pop).
        // Dim: subdued overlay so the eye is drawn back to the board.
        let anim_style: Option<Style> = anim_rows.and_then(|(rows, phase)| {
            if rows.contains(&board_row) {
                Some(match phase {
                    LineClearPhase::Flash => Style::default()
                        .fg(Color::Rgb(255, 255, 255))
                        .bg(Color::Rgb(255, 255, 255))
                        .add_modifier(Modifier::BOLD),
                    LineClearPhase::Dim => Style::default()
                        .fg(OVERLAY)
                        .bg(MANTLE)
                        .add_modifier(Modifier::DIM),
                })
            } else {
                None
            }
        });

        for col in 0..COLS {
            if let Some(kind) = state.board.cell_kind(col as usize, board_row) {
                let base_color = if theme.monochrome {
                    Color::Reset
                } else {
                    theme.color(kind)
                };
                let x = inner.x + (col as u16) * CELL_W;
                let y = inner.y + vis_row as u16 * CELL_H;
                if x + CELL_W <= inner.x + inner.width && y < inner.y + inner.height {
                    let (text, style) = if let Some(anim) = anim_style {
                        // Filled block during animation regardless of glyph.
                        (FILLED, anim)
                    } else {
                        let cell_style = if theme.monochrome {
                            Style::default().fg(base_color)
                        } else {
                            Style::default().fg(base_color).bg(BASE)
                        };
                        (FILLED, cell_style)
                    };
                    frame.render_widget(
                        ratatui::widgets::Paragraph::new(Span::styled(text, style)),
                        Rect::new(x, y, CELL_W, CELL_H),
                    );
                }
            }
        }
    }

    // 3. Ghost piece then active piece (skip during animation — no active piece).
    if let Some(active) = &state.active {
        let ghost_row = ghost_y(&state.board, active);
        // Only draw ghost if it differs from the active position.
        if ghost_row != active.origin.1 {
            let ghost = Piece {
                origin: (active.origin.0, ghost_row),
                ..*active
            };
            render_piece(frame, inner, &ghost, theme, true, 1.0);
        }
        let fade = spawn_fade_factor(state);
        render_piece(frame, inner, active, theme, false, fade);
    }
}

/// Draw one piece onto the frame area.
///
/// `is_ghost` renders ░░ in the ghost-surface color.
/// `fade` is a [0.0, 1.0] intensity multiplier for the spawn-fade animation;
/// use 1.0 for full intensity (non-fading pieces and ghosts).
fn render_piece(
    frame: &mut Frame,
    area: Rect,
    piece: &Piece,
    theme: &Theme,
    is_ghost: bool,
    fade: f32,
) {
    let color = if theme.monochrome {
        Color::Reset
    } else {
        dim_color(theme.color(piece.kind), fade)
    };

    for (col, row) in piece.cells() {
        let vis_row = row - 20;
        if !(0..VISIBLE_ROWS).contains(&vis_row) || !(0..COLS).contains(&col) {
            continue;
        }
        let x = area.x + (col as u16) * CELL_W;
        let y = area.y + vis_row as u16 * CELL_H;
        if x + CELL_W > area.x + area.width || y >= area.y + area.height {
            continue;
        }

        if is_ghost {
            use crate::render::theme::GHOST_MOD;
            let style = if theme.monochrome {
                Style::default()
                    .fg(Color::Reset)
                    .add_modifier(Modifier::DIM)
            } else {
                Style::default().fg(GHOST_MOD).bg(BASE)
            };
            frame.render_widget(
                ratatui::widgets::Paragraph::new(Span::styled(GHOST, style)),
                Rect::new(x, y, CELL_W, CELL_H),
            );
        } else {
            let glyph = if theme.monochrome {
                theme.glyph(piece.kind)
            } else {
                '█'
            };
            let s_owned: String = glyph.to_string().repeat(CELL_W as usize);
            let style = if theme.monochrome {
                Style::default().fg(Color::Reset)
            } else {
                Style::default().fg(color).bg(BASE)
            };
            frame.render_widget(
                ratatui::widgets::Paragraph::new(Span::styled(s_owned, style)),
                Rect::new(x, y, CELL_W, CELL_H),
            );
        }
    }
}
