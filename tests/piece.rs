use blocktxt::game::piece::{spawn, PieceKind, Rotation};

#[test]
fn o_spawns_at_cols_4_and_5() {
    let piece = spawn(PieceKind::O);
    assert!(matches!(piece.rotation, Rotation::Zero));

    // O piece cells: (4,18),(5,18),(4,19),(5,19)
    let cells = piece.cells();
    let cols: Vec<i32> = cells.iter().map(|&(c, _)| c).collect();
    assert!(cols.contains(&4), "O should occupy col 4; cells={cells:?}");
    assert!(cols.contains(&5), "O should occupy col 5; cells={cells:?}");
    // Must not occupy cols outside 4..=5
    for &(c, _) in &cells {
        assert!((4..=5).contains(&c), "O cell col {c} out of range 4..=5");
    }
}

#[test]
fn jlstz_spawn_bounding_box_is_cols_3_to_7() {
    for kind in [
        PieceKind::J,
        PieceKind::L,
        PieceKind::S,
        PieceKind::T,
        PieceKind::Z,
    ] {
        let piece = spawn(kind);
        let cells = piece.cells();
        let min_col = cells.iter().map(|&(c, _)| c).min().unwrap();
        let max_col = cells.iter().map(|&(c, _)| c).max().unwrap();
        assert!(
            min_col >= 3,
            "{kind:?} min col {min_col} should be >= 3; cells={cells:?}"
        );
        assert!(
            max_col <= 6,
            "{kind:?} max col {max_col} should be <= 6; cells={cells:?}"
        );
    }
}

#[test]
fn i_spawn_bounding_box_is_cols_3_to_7() {
    let piece = spawn(PieceKind::I);
    let cells = piece.cells();
    let min_col = cells.iter().map(|&(c, _)| c).min().unwrap();
    let max_col = cells.iter().map(|&(c, _)| c).max().unwrap();
    assert!(
        min_col >= 3,
        "I min col {min_col} should be >= 3; cells={cells:?}"
    );
    assert!(
        max_col <= 6,
        "I max col {max_col} should be <= 6; cells={cells:?}"
    );
}
