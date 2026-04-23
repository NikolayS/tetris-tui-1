use crate::game::piece::Piece;

/// Rotation direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RotationDir {
    Cw,
    Ccw,
}

/// Errors returned by `rotate`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SrsError {
    /// All SRS kick offsets were blocked; rotation rejected.
    Blocked,
}

/// Rotate `piece` in direction `dir` using SRS kick tables.
///
/// Full implementation deferred to Sprint 2.
pub fn rotate(_piece: &Piece, _dir: RotationDir) -> Result<Piece, SrsError> {
    unimplemented!("srs::rotate — Sprint 2 (issue TBD)")
}
