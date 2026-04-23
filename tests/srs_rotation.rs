use blocktxt::game::board::Board;
use blocktxt::game::piece::{spawn, PieceKind, Rotation};
use blocktxt::game::srs::{rotate, RotationDir, SrsError};

/// Sprint 1 red test — now green with Sprint 2 implementation.
///
/// For the I piece, the SRS Zero→R wall-kick table starts with the
/// `(0, 0)` offset on an open board — so rotation succeeds without
/// displacing the origin.
#[test]
fn srs_i_rotation_clockwise_applies_kick_offset() {
    let piece = spawn(PieceKind::I);
    let board = Board::empty();
    let rotated = rotate(&piece, RotationDir::Cw, &board).expect("rotation ok");
    assert_eq!(rotated.rotation, Rotation::R);
    // I Zero→R: (0, 0) kick on an open board — origin does not move.
    assert_eq!(rotated.origin, piece.origin);
}

/// Any JLSTZ piece on an open board uses the (0,0) kick (first in the table)
/// when rotating Zero→R.
#[test]
fn srs_jlstz_open_board_zero_to_r_uses_0_0_kick() {
    let board = Board::empty();
    for kind in [
        PieceKind::J,
        PieceKind::L,
        PieceKind::S,
        PieceKind::T,
        PieceKind::Z,
    ] {
        let piece = spawn(kind);
        let rotated = rotate(&piece, RotationDir::Cw, &board)
            .unwrap_or_else(|_| panic!("{kind:?} Zero→R should succeed on open board"));
        assert_eq!(
            rotated.rotation,
            Rotation::R,
            "{kind:?} should reach state R"
        );
        // (0,0) kick — origin unchanged.
        assert_eq!(
            rotated.origin, piece.origin,
            "{kind:?} open-board kick should be (0,0)"
        );
    }
}

/// Z piece CW full cycle on an open board: Zero→R→2→L→Zero.
/// After four CW rotations the rotation state returns to Zero and the
/// origin is identical to the original (all kicks are (0,0) on open board).
#[test]
fn srs_jlstz_cw_full_cycle_returns_to_zero() {
    let board = Board::empty();
    for kind in [
        PieceKind::J,
        PieceKind::L,
        PieceKind::S,
        PieceKind::T,
        PieceKind::Z,
    ] {
        let p0 = spawn(kind);
        let p1 = rotate(&p0, RotationDir::Cw, &board)
            .unwrap_or_else(|_| panic!("{kind:?} step 1 failed"));
        let p2 = rotate(&p1, RotationDir::Cw, &board)
            .unwrap_or_else(|_| panic!("{kind:?} step 2 failed"));
        let p3 = rotate(&p2, RotationDir::Cw, &board)
            .unwrap_or_else(|_| panic!("{kind:?} step 3 failed"));
        let p4 = rotate(&p3, RotationDir::Cw, &board)
            .unwrap_or_else(|_| panic!("{kind:?} step 4 failed"));
        assert_eq!(
            p4.rotation,
            Rotation::Zero,
            "{kind:?} should complete cycle back to Zero"
        );
        assert_eq!(
            p4.origin, p0.origin,
            "{kind:?} origin should be unchanged after full cycle"
        );
    }
}

/// JLSTZ CCW full cycle on an open board: Zero→L→2→R→Zero.
/// After four CCW rotations the rotation state returns to Zero and the
/// origin is identical to the original (all kicks are (0,0) on open board).
#[test]
fn srs_jlstz_ccw_full_cycle_returns_to_zero() {
    let board = Board::empty();
    for kind in [
        PieceKind::J,
        PieceKind::L,
        PieceKind::S,
        PieceKind::T,
        PieceKind::Z,
    ] {
        let p0 = spawn(kind);
        let p1 = rotate(&p0, RotationDir::Ccw, &board)
            .unwrap_or_else(|_| panic!("{kind:?} CCW step 1 failed"));
        let p2 = rotate(&p1, RotationDir::Ccw, &board)
            .unwrap_or_else(|_| panic!("{kind:?} CCW step 2 failed"));
        let p3 = rotate(&p2, RotationDir::Ccw, &board)
            .unwrap_or_else(|_| panic!("{kind:?} CCW step 3 failed"));
        let p4 = rotate(&p3, RotationDir::Ccw, &board)
            .unwrap_or_else(|_| panic!("{kind:?} CCW step 4 failed"));
        assert_eq!(
            p4.rotation,
            Rotation::Zero,
            "{kind:?} CCW should complete cycle back to Zero"
        );
        assert_eq!(
            p4.origin, p0.origin,
            "{kind:?} CCW origin should be unchanged after full cycle"
        );
    }
}

/// I piece CCW full cycle on an open board: Zero→L→2→R→Zero.
/// After four CCW rotations the rotation state returns to Zero.
#[test]
fn srs_i_ccw_full_cycle_returns_to_zero() {
    let board = Board::empty();
    let p0 = spawn(PieceKind::I);
    let p1 = rotate(&p0, RotationDir::Ccw, &board).expect("I step 1 CCW");
    let p2 = rotate(&p1, RotationDir::Ccw, &board).expect("I step 2 CCW");
    let p3 = rotate(&p2, RotationDir::Ccw, &board).expect("I step 3 CCW");
    let p4 = rotate(&p3, RotationDir::Ccw, &board).expect("I step 4 CCW");
    assert_eq!(p4.rotation, Rotation::Zero, "I should return to Zero");
    assert_eq!(
        p4.origin, p0.origin,
        "I origin unchanged after full CCW cycle"
    );
}

/// Verifies that SRS tries kicks in order and picks the first unblocked one.
///
/// Four cells are blocked so that kicks 0–3 all collide; **kick 4** (-1,-2)
/// is the first unblocked candidate and the expected winner.
///
/// JLSTZ Zero→R kicks (col_delta, row_delta):
///   kick 0: ( 0,  0)
///   kick 1: (-1,  0)
///   kick 2: (-1, +1)
///   kick 3: ( 0, -2)
///   kick 4: (-1, -2)  ← winning kick on this board
///
/// T in state R offsets from piece.rs: (1,0),(1,1),(2,1),(1,2).
/// Block (4,18),(4,19),(5,19),(4,20) → kicks 0–3 all collide;
/// kick 4 candidate origin (2,16): cells (3,16),(3,17),(4,17),(3,18) — clear.
#[test]
fn srs_rotation_uses_first_unblocked_kick() {
    // T in state R offsets relative to origin: (1,0),(1,1),(2,1),(1,2).
    // Kick 0 candidate origin = spawn origin (3,18):
    //   cells (4,18),(4,19),(5,19),(4,20) — block all four.
    // Kick 1 candidate origin = (3-1, 18+0) = (2,18):
    //   cells (3,18),(3,19),(4,19),(3,20) — (4,19) is blocked from above,
    //   so kick 1 also collides.
    // Kick 2 candidate origin = (3-1, 18+1) = (2,19):
    //   cells (3,19),(3,20),(4,20),(3,21) — (4,20) is blocked → collides.
    // Kick 3 candidate origin = (3+0, 18-2) = (3,16):
    //   cells (4,16),(4,17),(5,17),(4,18) — (4,18) is blocked → collides.
    // Kick 4 candidate origin = (3-1, 18-2) = (2,16):
    //   cells (3,16),(3,17),(4,17),(3,18) — all free → accepted!
    //
    // So on this board, kick 4 is the first unblocked: origin shifts by (-1,-2).
    let mut board = Board::empty();
    board.set(4, 18, PieceKind::T); // block kick-0 cell
    board.set(4, 19, PieceKind::T); // block kick-0 and kick-1 cell
    board.set(5, 19, PieceKind::T); // block kick-0 cell
    board.set(4, 20, PieceKind::T); // block kick-0 and kick-2 cell

    let piece = spawn(PieceKind::T); // origin (3,18), Zero state
    let rotated = rotate(&piece, RotationDir::Cw, &board).expect("kick 4 (-1,-2) should succeed");
    assert_eq!(rotated.rotation, Rotation::R);
    // Kick 4: col -1, row -2
    assert_eq!(
        rotated.origin,
        (piece.origin.0 - 1, piece.origin.1 - 2),
        "kick 4 should shift origin by (-1,-2)"
    );
}

/// When ALL 5 kicks are blocked, rotate returns Err(SrsError::BlockedAfterAllKicks).
///
/// Strategy: place the T piece near the left wall in the Zero state,
/// then surround the entire candidate region so every kick position
/// is obstructed.
#[test]
fn srs_rotation_blocked_returns_err() {
    // Place T at col 1, row 1 (hidden buffer) and block every cell
    // that any of the 5 Zero→R kick candidates could reach.
    let mut board2 = Board::empty();
    let origin = (1_i32, 1_i32);
    // JLSTZ Zero→R kicks: (0,0),(-1,0),(-1,+1),(0,-2),(-1,-2)
    // For each kick, T in state R cells are (0,0),(0,1),(0,2),(1,1)
    // relative to candidate origin. Block every cell that could be reached.
    let t_r_offsets: &[(i32, i32)] = &[(0, 0), (0, 1), (0, 2), (1, 1)];
    let kicks: &[(i32, i32)] = &[(0, 0), (-1, 0), (-1, 1), (0, -2), (-1, -2)];
    for &(kc, kr) in kicks {
        let cand_origin = (origin.0 + kc, origin.1 + kr);
        for &(dc, dr) in t_r_offsets {
            let col = cand_origin.0 + dc;
            let row = cand_origin.1 + dr;
            if (0..10).contains(&col) && (0..40).contains(&row) {
                board2.set(col as usize, row as usize, PieceKind::T);
            }
        }
    }

    let piece = blocktxt::game::piece::Piece {
        kind: PieceKind::T,
        rotation: Rotation::Zero,
        origin,
    };
    let result = rotate(&piece, RotationDir::Cw, &board2);
    assert_eq!(
        result,
        Err(SrsError::BlockedAfterAllKicks),
        "all kicks blocked should return Err"
    );
}
