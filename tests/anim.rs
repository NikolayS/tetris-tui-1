//! Unit tests for the line-clear animation phase transitions.
//!
//! All timing is deterministic via `FakeClock::advance`.

use std::time::{Duration, Instant};

use blocktxt::clock::{Clock, FakeClock};
use blocktxt::game::board::Board;
use blocktxt::game::piece::PieceKind;
use blocktxt::game::state::{
    GameState, Input, LineClearAnim, LineClearPhase, ANIM_DIM_MS, ANIM_FLASH_MS, ANIM_TOTAL_MS,
};
use blocktxt::Event;

// ── helpers ───────────────────────────────────────────────────────────────────

fn make_game(seed: u64) -> (GameState, FakeClock) {
    let origin = Instant::now();
    let clock = FakeClock::new(origin);
    let gs = GameState::new(seed, Box::new(clock.clone()));
    (gs, clock)
}

/// Fill all 10 columns of `row` (absolute board row) with O pieces.
fn fill_row(board: &mut Board, row: usize) {
    for col in 0..10usize {
        board.set(col, row, PieceKind::O);
    }
}

// ── constants ─────────────────────────────────────────────────────────────────

#[test]
fn anim_constants_match_spec() {
    assert_eq!(ANIM_FLASH_MS, 100, "flash phase must be 100 ms");
    assert_eq!(ANIM_DIM_MS, 100, "dim phase must be 100 ms");
    assert_eq!(
        ANIM_TOTAL_MS, 200,
        "total animation budget must be ≤ 200 ms"
    );
}

// ── phase machine via FakeClock ───────────────────────────────────────────────

/// Injecting a LineClearAnim in Flash phase and advancing 50 ms leaves it
/// in Flash (< 100 ms threshold).
#[test]
fn anim_stays_flash_before_100ms() {
    let (mut gs, clock) = make_game(42);

    fill_row(&mut gs.board, 39);
    gs.line_clear_anim = Some(LineClearAnim {
        rows: vec![39],
        started_at: clock.now(),
        phase: LineClearPhase::Flash,
        board_snapshot: gs.board.clone(),
        pending_count: 1,
        pending_level_before: 1,
        pending_b2b_active: false,
    });

    // Advance 50 ms — still in flash.
    clock.advance(Duration::from_millis(50));
    gs.step(Duration::from_millis(50), &[]);

    let phase = gs
        .line_clear_anim
        .as_ref()
        .expect("animation should still be active")
        .phase
        .clone();
    assert_eq!(phase, LineClearPhase::Flash);
}

/// Advancing exactly 100 ms transitions from Flash → Dim.
#[test]
fn anim_transitions_to_dim_at_100ms() {
    let (mut gs, clock) = make_game(42);

    fill_row(&mut gs.board, 39);
    gs.line_clear_anim = Some(LineClearAnim {
        rows: vec![39],
        started_at: clock.now(),
        phase: LineClearPhase::Flash,
        board_snapshot: gs.board.clone(),
        pending_count: 1,
        pending_level_before: 1,
        pending_b2b_active: false,
    });

    // Advance exactly 100 ms — crosses into dim.
    clock.advance(Duration::from_millis(100));
    gs.step(Duration::from_millis(100), &[]);

    let phase = gs
        .line_clear_anim
        .as_ref()
        .expect("animation should still be active at 100 ms")
        .phase
        .clone();
    assert_eq!(phase, LineClearPhase::Dim);
}

/// Advancing 200 ms completes the animation: `line_clear_anim` becomes `None`,
/// the board row is cleared, and `LinesCleared` event is emitted.
#[test]
fn anim_finishes_at_200ms_and_clears_board() {
    let (mut gs, clock) = make_game(42);

    // Fill bottom row with pieces to create a clearable line.
    fill_row(&mut gs.board, 39);
    let board_snap = gs.board.clone();

    gs.line_clear_anim = Some(LineClearAnim {
        rows: vec![39],
        started_at: clock.now(),
        phase: LineClearPhase::Flash,
        board_snapshot: board_snap,
        pending_count: 1,
        pending_level_before: 1,
        pending_b2b_active: false,
    });

    // Advance 200 ms — animation should complete.
    clock.advance(Duration::from_millis(200));
    let events = gs.step(Duration::from_millis(200), &[]);

    assert!(
        gs.line_clear_anim.is_none(),
        "animation must be cleared after 200 ms"
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::LinesCleared { count: 1, .. })),
        "LinesCleared(1) event must be emitted on animation finish"
    );
    // Row 39 should now be empty.
    assert!(
        (0..10).all(|c| gs.board.cell_kind(c, 39).is_none()),
        "cleared row must be empty after animation"
    );
}

/// During the animation (< 200 ms), gravity/lock are suspended and no
/// new piece is spawned prematurely.
#[test]
fn anim_suspends_gravity_during_play() {
    let (mut gs, clock) = make_game(42);

    fill_row(&mut gs.board, 39);
    // Remove active piece so we can test board-only state cleanly.
    gs.active = None;

    gs.line_clear_anim = Some(LineClearAnim {
        rows: vec![39],
        started_at: clock.now(),
        phase: LineClearPhase::Flash,
        board_snapshot: gs.board.clone(),
        pending_count: 1,
        pending_level_before: 1,
        pending_b2b_active: false,
    });

    // 150 ms — into dim but not finished.
    clock.advance(Duration::from_millis(150));
    gs.step(Duration::from_millis(150), &[]);

    // Row 39 must still be full (board not yet cleared).
    assert!(
        (0..10usize).all(|c| gs.board.cell_kind(c, 39).is_some()),
        "row must remain intact while animating"
    );
    // Animation still live.
    assert!(gs.line_clear_anim.is_some());
}

/// Inputs are accepted non-blocking during animation (no panic / hang).
#[test]
fn anim_accepts_inputs_without_blocking() {
    let (mut gs, clock) = make_game(42);

    fill_row(&mut gs.board, 39);
    gs.line_clear_anim = Some(LineClearAnim {
        rows: vec![39],
        started_at: clock.now(),
        phase: LineClearPhase::Flash,
        board_snapshot: gs.board.clone(),
        pending_count: 1,
        pending_level_before: 1,
        pending_b2b_active: false,
    });

    // Should not panic or deadlock even with movement inputs mid-animation.
    clock.advance(Duration::from_millis(50));
    gs.step(
        Duration::from_millis(50),
        &[Input::MoveLeft, Input::RotateCw],
    );
    // No assertion needed beyond "didn't panic".
    assert!(gs.line_clear_anim.is_some());
}

// ── GameOver new-best logic ───────────────────────────────────────────────────

/// `is_new_best(score, None)` → false (no store).
#[test]
fn new_best_none_store_returns_false() {
    assert!(!blocktxt::render::hud::is_new_best(999_999, None));
}

/// `is_new_best(score, Some(empty_store))` → true (any score beats nothing).
#[test]
fn new_best_empty_store_returns_true() {
    let store = blocktxt::persistence::HighScoreStore::new();
    assert!(blocktxt::render::hud::is_new_best(1, Some(&store)));
}

/// `is_new_best` returns false when score ≤ existing best.
#[test]
fn new_best_below_existing_returns_false() {
    use blocktxt::persistence::HighScore;
    let mut store = blocktxt::persistence::HighScoreStore::new();
    store.insert(HighScore {
        name: "prev".into(),
        score: 10_000,
        level: 1,
        lines: 5,
        ts: 0,
    });
    assert!(!blocktxt::render::hud::is_new_best(9_999, Some(&store)));
    assert!(!blocktxt::render::hud::is_new_best(10_000, Some(&store)));
}

/// `is_new_best` returns true when score beats existing best.
#[test]
fn new_best_above_existing_returns_true() {
    use blocktxt::persistence::HighScore;
    let mut store = blocktxt::persistence::HighScoreStore::new();
    store.insert(HighScore {
        name: "prev".into(),
        score: 10_000,
        level: 1,
        lines: 5,
        ts: 0,
    });
    assert!(blocktxt::render::hud::is_new_best(10_001, Some(&store)));
}
