//! Behavioral tests for Track J juice animations (#50).
//!
//! Each animation has at least one behavioral test verifying:
//!   - output differs during the animation window
//!   - output stabilises to the correct final value after the window

use std::time::{Duration, Instant};

use blocktxt::clock::{Clock, FakeClock};
use blocktxt::game::board::Board;
use blocktxt::game::piece::PieceKind;
use blocktxt::game::state::{
    GameState, LineClearAnim, LineClearPhase, SCORE_ROLLUP_MS, SPAWN_FADE1_MS, SPAWN_FADE_TOTAL_MS,
};
use blocktxt::{GameOverZoom, Input, Phase};

// ── helpers ───────────────────────────────────────────────────────────────────

fn make_game(seed: u64) -> (GameState, FakeClock) {
    let origin = Instant::now();
    let clock = FakeClock::new(origin);
    let gs = GameState::new(seed, Box::new(clock.clone()));
    (gs, clock)
}

fn fill_row(board: &mut Board, row: usize) {
    for col in 0..10usize {
        board.set(col, row, PieceKind::O);
    }
}

// ── 1. Spawn-fade animation ───────────────────────────────────────────────────

/// A fresh game starts with a SpawnAnim set.
#[test]
fn spawn_anim_present_on_new_game() {
    let (gs, _clock) = make_game(42);
    assert!(
        gs.spawn_anim.is_some(),
        "spawn_anim must be Some on game creation"
    );
}

/// spawn_anim is still active at 39 ms (< SPAWN_FADE1_MS boundary).
#[test]
fn spawn_anim_active_before_40ms() {
    let (mut gs, clock) = make_game(42);
    let dt = Duration::from_millis(SPAWN_FADE1_MS - 1);
    clock.advance(dt);
    gs.step(dt, &[]);
    assert!(
        gs.spawn_anim.is_some(),
        "spawn_anim must still be active before 40 ms"
    );
}

/// spawn_anim is cleared after SPAWN_FADE_TOTAL_MS (80 ms) elapses.
#[test]
fn spawn_anim_cleared_after_80ms() {
    let (mut gs, clock) = make_game(42);
    let dt = Duration::from_millis(SPAWN_FADE_TOTAL_MS);
    clock.advance(dt);
    gs.step(dt, &[]);
    assert!(
        gs.spawn_anim.is_none(),
        "spawn_anim must be cleared after 80 ms"
    );
}

/// spawn_anim is reset when a new piece spawns after a lock.
#[test]
fn spawn_anim_resets_on_new_piece() {
    let (mut gs, clock) = make_game(42);
    // Clear spawn anim from first piece.
    let dt = Duration::from_millis(SPAWN_FADE_TOTAL_MS + 1);
    clock.advance(dt);
    gs.step(dt, &[]);
    assert!(gs.spawn_anim.is_none(), "must be None after initial fade");

    // Hard-drop locks the active piece, which triggers spawn_next and
    // should set a new spawn_anim. We check immediately after the hard drop
    // step (before 80ms elapses) to verify the new anim was set.
    gs.step(Duration::ZERO, &[Input::HardDrop]);
    // After lock, there may be a line-clear anim. Drive it to completion
    // in small increments so spawn_anim is not yet expired (< 80ms).
    if gs.line_clear_anim.is_some() {
        let dt2 = Duration::from_millis(200);
        clock.advance(dt2);
        gs.step(dt2, &[]);
    }

    if matches!(gs.phase, Phase::Playing) {
        assert!(
            gs.spawn_anim.is_some(),
            "spawn_anim must be set after new piece spawns"
        );
    }
}

// ── 2. Score rollup animation ─────────────────────────────────────────────────

/// score_display.current starts at 0 and target at 0.
#[test]
fn score_display_starts_zero() {
    let (gs, _) = make_game(42);
    assert_eq!(gs.score_display.current, 0);
    assert_eq!(gs.score_display.target, 0);
}

/// After a line clear, score_display.target jumps but current lags.
#[test]
fn score_rollup_target_leads_current() {
    let (mut gs, clock) = make_game(42);

    // Fill bottom row to trigger line clear on hard drop.
    fill_row(&mut gs.board, 39);

    // Hard drop locks the piece (existing active piece won't complete the row
    // since it starts at top). Instead inject the anim directly.
    gs.line_clear_anim = Some(LineClearAnim {
        rows: vec![39],
        started_at: clock.now(),
        phase: LineClearPhase::Flash,
        board_snapshot: gs.board.clone(),
        pending_count: 1,
        pending_level_before: 1,
        pending_b2b_active: false,
    });

    // Drive to end of animation so score is computed.
    let anim_done = Duration::from_millis(200);
    clock.advance(anim_done);
    gs.step(anim_done, &[]);

    // Score should now be non-zero (line cleared).
    assert!(gs.score > 0, "score must increase after line clear");

    // target should now equal score; current should be < target.
    assert_eq!(
        gs.score_display.target, gs.score,
        "score_display.target must match game score"
    );
    assert!(
        gs.score_display.current < gs.score_display.target,
        "current must lag target immediately after clear"
    );
}

/// After SCORE_ROLLUP_MS of stepping, current catches up to target.
#[test]
fn score_rollup_catches_up_after_250ms() {
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

    // Drive anim to completion.
    let anim_done = Duration::from_millis(200);
    clock.advance(anim_done);
    gs.step(anim_done, &[]);

    // Now advance the rollup window.
    let rollup_dt = Duration::from_millis(SCORE_ROLLUP_MS);
    clock.advance(rollup_dt);
    gs.step(rollup_dt, &[]);

    assert_eq!(
        gs.score_display.current, gs.score_display.target,
        "score_display.current must equal target after rollup window"
    );
}

/// Midpoint of rollup has current strictly between 0 and target.
#[test]
fn score_rollup_interpolates_at_midpoint() {
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

    let anim_done = Duration::from_millis(200);
    clock.advance(anim_done);
    gs.step(anim_done, &[]);

    let target = gs.score_display.target;
    assert!(target > 0, "target must be non-zero");

    // Advance half the rollup window.
    let half_dt = Duration::from_millis(SCORE_ROLLUP_MS / 2);
    clock.advance(half_dt);
    gs.step(half_dt, &[]);

    let mid = gs.score_display.current;
    assert!(
        mid > 0 && mid < target,
        "current ({mid}) must be between 0 and target ({target}) at midpoint"
    );
}

// ── 3. Game-over zoom animation ───────────────────────────────────────────────

/// gameover_zoom is None while playing.
#[test]
fn gameover_zoom_none_while_playing() {
    let (gs, _) = make_game(42);
    assert!(
        gs.gameover_zoom.is_none(),
        "gameover_zoom must be None while playing"
    );
}

/// gameover_zoom is set when game over occurs.
#[test]
fn gameover_zoom_set_on_game_over() {
    let (mut gs, clock) = make_game(42);

    // Drive to game over by filling the board via hard drops.
    for _ in 0..400 {
        gs.step(Duration::from_millis(5), &[Input::HardDrop]);
        clock.advance(Duration::from_millis(5));
        if matches!(gs.phase, Phase::GameOver { .. }) {
            break;
        }
    }

    assert!(
        matches!(gs.phase, Phase::GameOver { .. }),
        "game must be over"
    );
    assert!(
        gs.gameover_zoom.is_some(),
        "gameover_zoom must be Some when game is over"
    );
}

/// scale() starts at 0.5 and reaches 1.0 after GAMEOVER_ZOOM_MS.
#[test]
fn gameover_zoom_scale_range() {
    let origin = Instant::now();
    let zoom = GameOverZoom { started_at: origin };

    // At t=0, scale = 0.5.
    let scale_start = zoom.scale(origin);
    assert!(
        (scale_start - 0.5).abs() < 0.01,
        "scale at t=0 must be ~0.5, got {scale_start}"
    );

    // At t=200ms (GAMEOVER_ZOOM_MS), scale = 1.0.
    let end = origin + Duration::from_millis(blocktxt::GAMEOVER_ZOOM_MS);
    let scale_end = zoom.scale(end);
    assert!(
        (scale_end - 1.0).abs() < 0.01,
        "scale at t=200ms must be ~1.0, got {scale_end}"
    );

    // At t=100ms, scale = ~0.75.
    let mid = origin + Duration::from_millis(blocktxt::GAMEOVER_ZOOM_MS / 2);
    let scale_mid = zoom.scale(mid);
    assert!(
        scale_mid > 0.5 && scale_mid < 1.0,
        "scale at midpoint must be between 0.5 and 1.0, got {scale_mid}"
    );
}

// ── 4. Flash pop — renderer reads LineClearPhase::Flash ──────────────────────

/// The Flash phase style differs from the Dim phase style.
/// This is a pure logic test: Flash produces REVERSED+BOLD, Dim produces DIM.
#[test]
fn flash_phase_is_distinct_from_dim_phase() {
    use blocktxt::LineClearPhase;

    // The renderer applies different styles per phase.
    // We assert the phases themselves are distinct enum variants.
    assert_ne!(
        LineClearPhase::Flash,
        LineClearPhase::Dim,
        "Flash and Dim phases must be distinct"
    );

    // And that the animation transitions correctly.
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

    // At 50ms still Flash.
    clock.advance(Duration::from_millis(50));
    gs.step(Duration::from_millis(50), &[]);
    assert_eq!(
        gs.line_clear_anim.as_ref().unwrap().phase,
        LineClearPhase::Flash
    );

    // At 100ms transitions to Dim.
    clock.advance(Duration::from_millis(50));
    gs.step(Duration::from_millis(50), &[]);
    assert_eq!(
        gs.line_clear_anim.as_ref().unwrap().phase,
        LineClearPhase::Dim
    );
}

/// The spawn_anim kind field matches the active piece kind on spawn.
#[test]
fn spawn_anim_kind_matches_active_piece() {
    let (gs, _clock) = make_game(42);
    if let (Some(sa), Some(active)) = (&gs.spawn_anim, &gs.active) {
        assert_eq!(
            sa.kind, active.kind,
            "SpawnAnim kind must match active piece kind"
        );
    } else {
        panic!("Expected both spawn_anim and active to be Some");
    }
}

/// On game-over, `score_display.current` must snap to `target` so the HUD
/// behind the overlay shows the final score, not a frozen mid-rollup value.
///
/// Reproduces the issue where `step()` returns early in the GameOver branch
/// before ticking the rollup, leaving `current < target` indefinitely (#54).
#[test]
fn score_display_snaps_to_target_on_game_over() {
    let (mut gs, clock) = make_game(42);

    // Stage a pending rollup: target ahead of current.
    gs.score_display.current = 100;
    gs.score_display.target = 1_000;

    // Drive to game over via hard drops.
    for _ in 0..400 {
        gs.step(Duration::from_millis(5), &[Input::HardDrop]);
        clock.advance(Duration::from_millis(5));
        if matches!(gs.phase, Phase::GameOver { .. }) {
            break;
        }
    }

    assert!(
        matches!(gs.phase, Phase::GameOver { .. }),
        "game must be over"
    );
    assert_eq!(
        gs.score_display.current, gs.score_display.target,
        "current must snap to target on game-over so the HUD does not freeze"
    );
}
