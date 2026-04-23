//! Unit tests for `render::helpers` — pure functions only.

use std::collections::VecDeque;

use blocktxt::game::board::Board;
use blocktxt::game::piece::{Piece, PieceKind, Rotation};
use blocktxt::render::helpers::{
    format_level, format_lines, format_score, ghost_y, next_preview_glyphs,
};
use blocktxt::render::theme::Theme;

// ── ghost_y ──────────────────────────────────────────────────────────────────

/// I-piece at col 3, row 18 on an empty board should land at row 36
/// (the piece occupies rows origin+1 = 19..23 but the board floor is row 40
/// and `is_occupied` returns true for row 40; cells are at origin+1 so the
/// piece body lands with its lowest cell at row 39 ← offset 21 from origin.18).
///
/// I piece in Zero rotation: offsets are (0,1),(1,1),(2,1),(3,1).
/// origin=(3,18) → cells at rows 19. Floor is row 40 (out-of-bounds = true).
/// ghost_y drops until row+1 hits the floor.
/// body bottom is origin.1 + 1 = 19 for the piece.
/// Can drop: 19 → 20 → ... → 39. floor triggers at row 40.
/// ghost_y = origin.1 + offset where piece bottom == 39.
/// Original origin.1 = 18, so offset = 39 - 19 = 20, ghost_origin = 38.
#[test]
fn ghost_y_open_column_lands_on_floor() {
    let board = Board::empty();
    // I piece, Zero rotation, origin (3, 18).
    // Body cells at (3,19),(4,19),(5,19),(6,19).
    // is_occupied returns true for row=40 (out of bounds).
    // So piece can drop while bottom cell row < 39 (row+1 < 40).
    // Bottom cell = origin.1 + 1. Stops when origin.1 + 1 + 1 == 40 → origin.1 = 38.
    let piece = Piece {
        kind: PieceKind::I,
        rotation: Rotation::Zero,
        origin: (3, 18),
    };
    let g = ghost_y(&board, &piece);
    // ghost origin row should be 38 (bottom cell at row 39, row 40 is floor).
    assert_eq!(
        g, 38,
        "I piece should land with origin at row 38 on empty board"
    );
}

#[test]
fn ghost_y_stops_on_stack() {
    let mut board = Board::empty();
    // Place a blocker at row 30, columns 3-6.
    for col in 0..10usize {
        board.set(col, 30, PieceKind::O);
    }
    // I piece Zero rotation, origin (3, 18): body cells at row origin+1.
    // Piece body row = origin.1 + 1. Blocker at row 30.
    // Piece can drop until origin.1 + 1 + 1 == 30 → origin.1 = 28.
    let piece = Piece {
        kind: PieceKind::I,
        rotation: Rotation::Zero,
        origin: (3, 18),
    };
    let g = ghost_y(&board, &piece);
    assert_eq!(g, 28, "I piece should stop above blocker at row 30");
}

// ── format_score ─────────────────────────────────────────────────────────────

#[test]
fn format_score_zero_pad() {
    let s = format_score(0);
    assert_eq!(s, "        0", "zero should be right-aligned to 9 chars");
}

#[test]
fn format_score_thousands_separator() {
    let s = format_score(1_234_567);
    assert_eq!(s, "1 234 567");
}

#[test]
fn format_score_small() {
    let s = format_score(42);
    assert_eq!(s, "       42");
}

// ── format_lines / format_level ───────────────────────────────────────────────

#[test]
fn format_lines_width() {
    assert_eq!(format_lines(0).len(), 6);
    assert_eq!(format_lines(999_999).len(), 6);
}

#[test]
fn format_level_width() {
    assert_eq!(format_level(1).len(), 3);
    assert_eq!(format_level(99).len(), 3);
}

// ── next_preview_glyphs ───────────────────────────────────────────────────────

#[test]
fn next_preview_glyphs_maps_kind_to_char() {
    let theme = Theme::monochrome();
    let mut queue: VecDeque<PieceKind> = VecDeque::new();
    queue.push_back(PieceKind::I);
    queue.push_back(PieceKind::O);
    queue.push_back(PieceKind::T);

    let pairs: Vec<(PieceKind, char)> = next_preview_glyphs(&queue, &theme).collect();
    assert_eq!(pairs.len(), 3);
    assert_eq!(pairs[0], (PieceKind::I, 'I'));
    assert_eq!(pairs[1], (PieceKind::O, 'O'));
    assert_eq!(pairs[2], (PieceKind::T, 'T'));
}
