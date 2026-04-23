//! Board view: draws the 10×20 visible playfield, active piece, and ghost.

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use ratatui::Frame;

use crate::game::piece::Piece;
use crate::game::state::GameState;
use crate::render::helpers::ghost_y;
use crate::render::theme::Theme;

/// Width of one cell in terminal columns (each cell is 2 chars wide).
const CELL_W: u16 = 2;
/// Height of one cell in terminal rows.
const CELL_H: u16 = 1;

/// Number of visible rows (rows 20..40 of the 40-row playfield).
const VISIBLE_ROWS: i32 = 20;
/// Number of columns.
const COLS: i32 = 10;

/// Draw the visible 10×20 playfield into `area`.
///
/// Renders:
///   1. Locked board cells (rows 20..40).
///   2. Ghost piece (hollow, dimmed) at the drop position.
///   3. Active piece at its current position.
///
/// Takes `&GameState` and `&Theme` — no mutation.
pub fn draw(frame: &mut Frame, area: Rect, state: &GameState, theme: &Theme) {
    // 1. Draw border / background dots for empty cells.
    for vis_row in 0..VISIBLE_ROWS {
        for col in 0..COLS {
            let x = area.x + (col as u16) * CELL_W;
            let y = area.y + vis_row as u16 * CELL_H;
            if x + CELL_W <= area.x + area.width && y < area.y + area.height {
                frame.render_widget(
                    ratatui::widgets::Paragraph::new(Span::raw("  ")),
                    Rect::new(x, y, CELL_W, CELL_H),
                );
            }
        }
    }

    // 2. Locked board cells (rows 20..40).
    for vis_row in 0..VISIBLE_ROWS {
        let board_row = vis_row + 20;
        for col in 0..COLS {
            if let Some(kind) = state.board.cell_kind(col as usize, board_row as usize) {
                let color = if theme.monochrome {
                    Color::Reset
                } else {
                    theme.color(kind)
                };
                let glyph = theme.glyph(kind);
                let x = area.x + (col as u16) * CELL_W;
                let y = area.y + vis_row as u16 * CELL_H;
                if x + CELL_W <= area.x + area.width && y < area.y + area.height {
                    let style = Style::default().fg(color);
                    let s: String = std::iter::repeat(glyph).take(CELL_W as usize).collect();
                    frame.render_widget(
                        ratatui::widgets::Paragraph::new(Span::styled(s, style)),
                        Rect::new(x, y, CELL_W, CELL_H),
                    );
                }
            }
        }
    }

    // 3. Ghost piece then active piece.
    if let Some(active) = &state.active {
        let ghost_row = ghost_y(&state.board, active);
        // Only draw ghost if it differs from the active position.
        if ghost_row != active.origin.1 {
            let ghost = Piece {
                origin: (active.origin.0, ghost_row),
                ..*active
            };
            render_piece(frame, area, &ghost, theme, true);
        }
        render_piece(frame, area, active, theme, false);
    }
}

/// Draw one piece onto the frame area.
///
/// `is_ghost` renders a dimmer hollow version.
fn render_piece(frame: &mut Frame, area: Rect, piece: &Piece, theme: &Theme, is_ghost: bool) {
    let color = if theme.monochrome {
        Color::Reset
    } else {
        theme.color(piece.kind)
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

        let (text, style) = if is_ghost {
            (
                "[]".to_string(),
                Style::default().fg(color).add_modifier(Modifier::DIM),
            )
        } else {
            let glyph = theme.glyph(piece.kind);
            let s: String = std::iter::repeat(glyph).take(CELL_W as usize).collect();
            (s, Style::default().fg(color))
        };

        frame.render_widget(
            ratatui::widgets::Paragraph::new(Span::styled(text, style)),
            Rect::new(x, y, CELL_W, CELL_H),
        );
    }
}
