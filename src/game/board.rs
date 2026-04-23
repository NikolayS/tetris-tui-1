use crate::game::piece::PieceKind;

/// 10-column × 40-row playfield.
///
/// Rows 0..20 are the hidden buffer (spawn area).
/// Rows 20..40 are the visible playfield.
/// Origin (0, 0) is top-left; x increases rightward, y increases downward.
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
    pub fn clear_full_rows(&mut self) -> u8 {
        let mut cleared: u8 = 0;
        let mut write_row = 39_usize;

        // Walk rows from bottom to top; copy non-full rows into write_row.
        let mut read_row = 39_i64;
        while read_row >= 0 {
            let r = read_row as usize;
            if self.cells[r].iter().all(|c| c.is_some()) {
                // Full row — skip it (don't copy), count as cleared.
                cleared += 1;
            } else {
                // Non-full row — copy to write_row.
                self.cells[write_row] = self.cells[r];
                write_row = write_row.saturating_sub(1);
            }
            read_row -= 1;
        }

        // Fill the top rows that were vacated with empty rows.
        for r in 0..=write_row {
            self.cells[r] = [None; 10];
        }

        cleared
    }
}
