use blocktxt::game::board::Board;
use blocktxt::game::piece::PieceKind;

#[test]
fn empty_board_has_no_occupancy_on_visible_rows() {
    let board = Board::empty();
    for row in 20..40 {
        for col in 0..10 {
            assert!(
                !board.is_occupied(col, row),
                "expected empty at col={col} row={row}"
            );
        }
    }
}

#[test]
fn set_and_is_occupied_round_trip() {
    let mut board = Board::empty();
    board.set(3, 25, PieceKind::T);
    assert!(board.is_occupied(3, 25));
    assert!(!board.is_occupied(4, 25));
    assert!(!board.is_occupied(3, 26));
}

#[test]
fn out_of_bounds_reads_as_occupied() {
    let board = Board::empty();
    // negative col
    assert!(board.is_occupied(-1, 20));
    // negative row
    assert!(board.is_occupied(0, -1));
    // col >= 10
    assert!(board.is_occupied(10, 20));
    // row >= 40
    assert!(board.is_occupied(0, 40));
}

#[test]
fn clear_full_rows_counts_and_shifts_down() {
    let mut board = Board::empty();
    // Fill row 39 (bottom visible row) completely.
    for col in 0..10 {
        board.set(col, 39, PieceKind::I);
    }
    // Partially fill row 38.
    board.set(0, 38, PieceKind::O);

    let cleared = board.clear_full_rows();
    assert_eq!(cleared, 1, "expected 1 cleared row");

    // The partial row should have shifted down by one.
    assert!(
        board.is_occupied(0, 39),
        "partial row should shift to row 39"
    );
    assert!(!board.is_occupied(0, 38), "row 38 should now be empty");

    // Row 39 col 1 should be empty (was not filled in partial row).
    assert!(!board.is_occupied(1, 39));
}

#[test]
fn clear_full_rows_on_empty_board_does_not_mutate() {
    let mut b = Board::empty();
    b.set(3, 5, PieceKind::I);
    b.set(3, 10, PieceKind::T);
    let snapshot_before = b.clone();
    let cleared = b.clear_full_rows();
    assert_eq!(cleared, 0);
    assert_eq!(b, snapshot_before);
}

#[test]
fn clear_full_rows_leaves_partial_row_intact() {
    let mut b = Board::empty();
    for c in 0..9 {
        b.set(c, 25, PieceKind::I);
    }
    let cleared = b.clear_full_rows();
    assert_eq!(cleared, 0);
    for c in 0..9 {
        assert!(b.is_occupied(c, 25));
    }
    assert!(!b.is_occupied(9, 25));
}

#[test]
fn block_out_detected() {
    // Spawn cells for O are cols 4 and 5, rows 18..20.
    // Fill one of those cells to simulate block-out condition.
    let mut board = Board::empty();
    board.set(4, 18, PieceKind::I);

    // A newly spawning O piece would have cells at (4,18),(5,18),(4,19),(5,19).
    // Collision against board means block-out.
    let spawn_cells: [(i32, i32); 4] = [(4, 18), (5, 18), (4, 19), (5, 19)];
    let blocked = spawn_cells.iter().any(|&(c, r)| board.is_occupied(c, r));
    assert!(
        blocked,
        "block-out should be detected when spawn cells are occupied"
    );
}
