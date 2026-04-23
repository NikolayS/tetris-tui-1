use crate::game::piece::PieceKind;

/// 10-column × 40-row playfield.
///
/// Rows 0..20 are the hidden buffer (spawn area).
/// Rows 20..40 are the visible playfield.
/// Origin (0, 0) is top-left; x increases rightward, y increases downward.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Board {
    cells: [[Option<PieceKind>; 10]; 40],
}

impl Board {
    /// Return an empty board with no occupied cells.
    pub fn empty() -> Self {
        Self {
            cells: [[None; 10]; 40],
        }
    }

    /// Return true if the cell at (col, row) is occupied or out-of-bounds.
    ///
    /// Out-of-bounds coordinates always return true so that collision
    /// checks treat walls and floor as solid.
    pub fn is_occupied(&self, col: i32, row: i32) -> bool {
        if !(0..10).contains(&col) || !(0..40).contains(&row) {
            return true;
        }
        self.cells[row as usize][col as usize].is_some()
    }

    /// Place a piece kind at (col, row). Panics if out of bounds.
    pub fn set(&mut self, col: usize, row: usize, piece: PieceKind) {
        self.cells[row][col] = Some(piece);
    }

    /// Clear all fully-occupied rows, shift everything above down,
    /// and return the number of rows cleared.
    ///
    /// When no rows are full, the board is left completely untouched.
    pub fn clear_full_rows(&mut self) -> u8 {
        // First pass: count full rows. If zero, return without mutating
        // any cells — this avoids the off-by-one that would erase row 0.
        let cleared: u8 = self
            .cells
            .iter()
            .filter(|row| row.iter().all(|c| c.is_some()))
            .count() as u8;
        if cleared == 0 {
            return 0;
        }

        // Second pass: compact non-full rows downward, bottom-up.
        // `write_row` is the destination; iterate from row 39 down to 0.
        let mut write_row: i64 = 39;
        for read_row in (0..40_i64).rev() {
            let r = read_row as usize;
            if self.cells[r].iter().all(|c| c.is_some()) {
                // Full row — skip (do not copy).
                continue;
            }
            self.cells[write_row as usize] = self.cells[r];
            write_row -= 1;
        }

        // Any rows above the final write_row were vacated — fill them
        // with empty cells. `write_row` now points one row above the
        // topmost compacted row (may be -1 if every row was cleared,
        // which is impossible here since cleared <= 40 and we only
        // reach this branch when some full rows existed).
        for r in 0..=write_row {
            self.cells[r as usize] = [None; 10];
        }

        cleared
    }
}
