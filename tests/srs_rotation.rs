use blocktxt::game::piece::{spawn, PieceKind, Rotation};
use blocktxt::game::srs::{rotate, RotationDir};

/// Red test — Sprint 2 will implement srs::rotate.
///
/// For the I piece, the SRS Zero→R wall-kick table starts with the
/// `(0, 0)` offset on an open board — so rotation succeeds without
/// displacing the origin. Sprint 1's `srs::rotate` is
/// `unimplemented!()`, so this test panics; `#[ignore]` keeps CI green
/// until Sprint 2.
#[test]
#[ignore = "red: Sprint 2 will implement srs::rotate"]
fn srs_i_rotation_clockwise_applies_kick_offset() {
    let piece = spawn(PieceKind::I);
    let rotated = rotate(&piece, RotationDir::Cw).expect("rotation ok");
    assert_eq!(rotated.rotation, Rotation::R);
    // I Zero→R: (0, 0) kick on an open board — origin does not move.
    assert_eq!(rotated.origin, piece.origin);
}
