//! Pure helper functions for the renderer — no I/O, no terminal state.
//!
//! All functions here are deterministic and side-effect-free, making them
//! trivially unit-testable without a real terminal.

use std::collections::VecDeque;

use crate::game::board::Board;
use crate::game::piece::{Piece, PieceKind};
use crate::render::theme::Theme;

/// Compute the row at which `piece` would land if dropped straight down.
///
/// Returns the minimum `row_offset` such that shifting `piece` down by
/// `row_offset` causes at least one cell to collide with the board.
/// The ghost origin row is `piece.origin.1 + row_offset - 1`.
///
/// Pure function: does not mutate anything.
pub fn ghost_y(board: &Board, piece: &Piece) -> i32 {
    let mut offset = 0i32;
    loop {
        let candidate = Piece {
            origin: (piece.origin.0, piece.origin.1 + offset + 1),
            ..*piece
        };
        let blocked = candidate
            .cells()
            .iter()
            .any(|&(c, r)| board.is_occupied(c, r));
        if blocked {
            break;
        }
        offset += 1;
    }
    piece.origin.1 + offset
}

/// Format a score with space-as-thousands-separator, right-aligned to 9 chars.
///
/// Examples:
///   `format_score(0)`       → `"        0"`
///   `format_score(1234567)` → `"1 234 567"`
pub fn format_score(score: u32) -> String {
    let s = score.to_string();
    // Insert separators every 3 digits from the right.
    let mut out = String::with_capacity(s.len() + (s.len().saturating_sub(1)) / 3);
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(' ');
        }
        out.push(ch);
    }
    let formatted: String = out.chars().rev().collect();
    format!("{:>9}", formatted)
}

/// Format a line count, right-aligned to 6 chars.
pub fn format_lines(n: u32) -> String {
    format!("{:>6}", n)
}

/// Format a level number, right-aligned to 3 chars.
pub fn format_level(level: u8) -> String {
    format!("{:>3}", level)
}

/// Map the first `limit` entries in `next_queue` to `(PieceKind, glyph)` pairs.
///
/// Pure: reads the theme glyph table without any terminal interaction.
pub fn next_preview_glyphs<'a>(
    next_queue: &'a VecDeque<PieceKind>,
    theme: &'a Theme,
) -> impl Iterator<Item = (PieceKind, char)> + 'a {
    next_queue.iter().map(|&k| (k, theme.glyphs[k as usize]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_score_zero() {
        assert_eq!(format_score(0), "        0");
    }

    #[test]
    fn format_score_thousands() {
        assert_eq!(format_score(1_234_567), "1 234 567");
    }
}
