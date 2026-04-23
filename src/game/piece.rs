/// The seven piece kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PieceKind {
    I,
    O,
    T,
    S,
    Z,
    J,
    L,
}

/// The four rotation states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rotation {
    Zero,
    R,
    Two,
    L,
}

/// A piece on the board: kind, rotation state, and grid origin.
///
/// `origin` is the top-left corner of the piece's bounding box in
/// board coordinates (col, row).  Cell offsets in the shape tables are
/// relative to this origin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Piece {
    pub kind: PieceKind,
    pub rotation: Rotation,
    pub origin: (i32, i32),
}

impl Piece {
    /// Return absolute board coordinates of this piece's occupied cells.
    pub fn cells(&self) -> Vec<(i32, i32)> {
        let offsets = shape_offsets(self.kind, self.rotation);
        offsets
            .iter()
            .map(|&(dc, dr)| (self.origin.0 + dc, self.origin.1 + dr))
            .collect()
    }
}

/// Cell offsets (col_delta, row_delta) relative to the piece origin.
///
/// Only `Rotation::Zero` is defined for Sprint 1; other rotations are
/// Sprint 2 (srs.rs).
fn shape_offsets(kind: PieceKind, rotation: Rotation) -> &'static [(i32, i32)] {
    // Sprint 1: only Zero rotation shapes are defined.
    // Sprint 2 will fill in R, Two, L via srs.rs.
    match rotation {
        Rotation::Zero => shape_offsets_zero(kind),
        // Placeholder: return same shape as Zero until Sprint 2.
        _ => shape_offsets_zero(kind),
    }
}

/// Zero-rotation cell offsets (col, row) relative to bounding-box top-left.
///
/// Bounding box convention:
///   I, J, L, S, T, Z — 4-wide (cols 0..4), 1–2 rows.
///   O                 — 2-wide (cols 0..2), 2 rows.
fn shape_offsets_zero(kind: PieceKind) -> &'static [(i32, i32)] {
    match kind {
        // I: row 1 of the 4×2 bbox (standard Guideline Zero state)
        //    ....
        //    XXXX
        PieceKind::I => &[(0, 1), (1, 1), (2, 1), (3, 1)],
        // O: 2×2
        //    XX
        //    XX
        PieceKind::O => &[(0, 0), (1, 0), (0, 1), (1, 1)],
        // T:
        //    .X.
        //    XXX
        PieceKind::T => &[(1, 0), (0, 1), (1, 1), (2, 1)],
        // S:
        //    .XX
        //    XX.
        PieceKind::S => &[(1, 0), (2, 0), (0, 1), (1, 1)],
        // Z:
        //    XX.
        //    .XX
        PieceKind::Z => &[(0, 0), (1, 0), (1, 1), (2, 1)],
        // J:
        //    X..
        //    XXX
        PieceKind::J => &[(0, 0), (0, 1), (1, 1), (2, 1)],
        // L:
        //    ..X
        //    XXX
        PieceKind::L => &[(2, 0), (0, 1), (1, 1), (2, 1)],
    }
}

/// Spawn a piece at the Guideline-inspired spawn position.
///
/// Per SPEC §4 (round-2 decision):
/// - O: 2-wide bbox top-left at (col=4, row=18) → cells at cols 4..=5.
/// - I: 4-wide bbox top-left at (col=3, row=18) → bbox cols 3..7.
/// - J, L, S, T, Z: 4-wide bbox top-left at (col=3, row=18) → bbox cols 3..7.
pub fn spawn(kind: PieceKind) -> Piece {
    let origin = match kind {
        PieceKind::O => (4, 18),
        _ => (3, 18),
    };
    Piece {
        kind,
        rotation: Rotation::Zero,
        origin,
    }
}
