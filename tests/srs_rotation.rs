use blocktxt::game::piece::{spawn, PieceKind};
use blocktxt::game::srs::{rotate, RotationDir};

/// Red test — Sprint 2 will implement srs::rotate.
///
/// Asserts that rotating the I piece clockwise from state Zero via
/// `srs::rotate` produces a piece whose origin has moved by the
/// expected SRS kick offset. Sprint 1's srs::rotate is
/// `unimplemented!()`, so this test panics; #[ignore] keeps CI green.
#[test]
#[ignore = "red: Sprint 2 will implement srs::rotate"]
fn srs_i_rotation_clockwise_applies_kick_offset() {
    let piece = spawn(PieceKind::I);
    // Expected: CW rotation from Zero→R succeeds (no board, no collision).
    let rotated = rotate(&piece, RotationDir::Cw)
        .expect("CW rotation of I from Zero should succeed with no board obstacles");
    // The rotated piece origin must differ from the spawn origin by the
    // SRS Zero→R kick offset for I: (0, 0) is the first test point
    // (no displacement needed on an open board), but the rotation
    // state must change.
    assert_ne!(
        rotated.origin, piece.origin,
        "expected origin to shift under SRS kick for I CW"
    );
}
