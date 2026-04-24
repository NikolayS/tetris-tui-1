//! Hold-piece mechanic tests (Guideline §1a, PR #012).
//!
//! RED phase: all tests compile but fail until the feature is implemented.

use blocktxt::clock::{Clock, FakeClock};
use blocktxt::game::state::{Event, GameOverReason, GameState, Input, Phase};
use std::time::{Duration, Instant};

// ── helpers ───────────────────────────────────────────────────────────────────

fn make_game(seed: u64) -> (GameState, FakeClock) {
    let origin = Instant::now();
    let clock = FakeClock::new(origin);
    let mut gs = GameState::new(seed, Box::new(clock.clone()));
    gs.step(Duration::ZERO, &[Input::StartGame]);
    (gs, clock)
}

fn tick(gs: &mut GameState, dt: Duration) -> Vec<Event> {
    gs.step(dt, &[])
}

// ── hold_once_swaps_with_bag ──────────────────────────────────────────────────

/// First hold on an empty slot: active kind goes to hold, a new piece comes
/// from the bag (not from the empty hold slot).
#[test]
fn hold_once_swaps_with_bag() {
    let (mut gs, _clock) = make_game(42);

    // Nothing in hold yet.
    assert!(gs.hold.is_none(), "hold slot must start empty");

    let active_kind = gs.active.unwrap().kind;
    gs.step(Duration::ZERO, &[Input::Hold]);

    // Active went to hold.
    assert_eq!(
        gs.hold,
        Some(active_kind),
        "active kind must move to hold slot"
    );
    // A fresh piece (from the bag) is now active.
    assert!(gs.active.is_some(), "a new piece must be active after hold");
    // The new active piece is different from the held kind (generally true).
    // More importantly: hold_used_this_cycle must now be set.
    assert!(
        gs.hold_used_this_cycle,
        "hold_used_this_cycle must be true after hold"
    );
}

// ── hold_twice_swaps_active_and_hold ─────────────────────────────────────────

/// After locking and re-spawning, a second hold should swap active ↔ hold.
#[test]
fn hold_twice_swaps_active_and_hold() {
    let (mut gs, _clock) = make_game(42);

    // First hold: active (kind A) → hold, new piece (kind B) becomes active.
    let kind_a = gs.active.unwrap().kind;
    gs.step(Duration::ZERO, &[Input::Hold]);
    let kind_b = gs.active.unwrap().kind;

    // Lock the current piece (hard-drop) so cycle resets.
    gs.step(Duration::ZERO, &[Input::HardDrop]);

    // Wait for any line-clear anim to finish.
    for _ in 0..30 {
        tick(&mut gs, Duration::from_millis(10));
        if gs.active.is_some() && matches!(gs.phase, Phase::Playing) {
            break;
        }
    }

    // hold_used_this_cycle should now be false (new piece cycle).
    assert!(
        !gs.hold_used_this_cycle,
        "hold_used_this_cycle must reset after lock"
    );

    // Second hold: new piece goes to hold, kind_a comes back.
    let kind_c = gs.active.unwrap().kind;
    gs.step(Duration::ZERO, &[Input::Hold]);
    // After second hold, hold should contain kind_c and active should be kind_a.
    assert_eq!(
        gs.hold,
        Some(kind_c),
        "current active must move into hold slot"
    );
    assert_eq!(
        gs.active.unwrap().kind,
        kind_a,
        "held kind must come back as active"
    );
    let _ = kind_b;
}

// ── hold_locks_until_next_cycle ───────────────────────────────────────────────

/// After one hold in a cycle, a second hold in the same cycle must be a no-op.
#[test]
fn hold_locks_until_next_cycle() {
    let (mut gs, _clock) = make_game(42);

    let kind_a = gs.active.unwrap().kind;

    // First hold succeeds.
    gs.step(Duration::ZERO, &[Input::Hold]);
    assert_eq!(gs.hold, Some(kind_a));
    let kind_b = gs.active.unwrap().kind;

    // Second hold in the same cycle must be rejected.
    gs.step(Duration::ZERO, &[Input::Hold]);
    // Nothing changes: hold still has kind_a, active still kind_b.
    assert_eq!(gs.hold, Some(kind_a), "hold must not change on second hold");
    assert_eq!(
        gs.active.unwrap().kind,
        kind_b,
        "active must not change on locked hold"
    );
}

// ── hold_unlocks_after_lock ───────────────────────────────────────────────────

/// After a lock, hold_used_this_cycle resets and hold works again.
#[test]
fn hold_unlocks_after_lock() {
    let (mut gs, _clock) = make_game(42);

    // Hold once.
    gs.step(Duration::ZERO, &[Input::Hold]);
    assert!(gs.hold_used_this_cycle);

    // Lock the piece.
    gs.step(Duration::ZERO, &[Input::HardDrop]);

    // Drain any line-clear animation.
    for _ in 0..30 {
        tick(&mut gs, Duration::from_millis(10));
        if gs.active.is_some() && !gs.hold_used_this_cycle {
            break;
        }
    }

    assert!(
        !gs.hold_used_this_cycle,
        "hold must unlock after piece locks"
    );

    // Hold should succeed now.
    let kind_before = gs.active.unwrap().kind;
    gs.step(Duration::ZERO, &[Input::Hold]);
    assert!(gs.hold.is_some(), "hold must work after unlock");
    assert_ne!(
        gs.active.unwrap().kind,
        kind_before,
        "active must change after successful hold"
    );
}

// ── hold_resets_lock_delay ────────────────────────────────────────────────────

/// A piece that is mid-lock-delay gets replaced by hold; the new piece
/// arrives at its spawn position with a clear lock state (airborne).
#[test]
fn hold_resets_lock_delay() {
    let (mut gs, _clock) = make_game(42);

    // Bring the active piece to the floor.
    gs.step(Duration::ZERO, &[Input::SoftDropOn]);
    for _ in 0..50 {
        let row_before = gs.active.map(|p| p.origin.1).unwrap_or(-1);
        tick(&mut gs, Duration::from_millis(50));
        let row_after = gs.active.map(|p| p.origin.1).unwrap_or(-1);
        if row_after <= row_before {
            break;
        }
    }
    gs.step(Duration::ZERO, &[Input::SoftDropOff]);

    // Piece is grounded; lock_state should exist with some elapsed time.
    assert!(
        gs.lock_state.is_some(),
        "lock_state must be Some when grounded"
    );

    // Now hold: the grounded piece swaps out; new piece comes in fresh.
    gs.step(Duration::ZERO, &[Input::Hold]);

    // After hold, lock_state must be cleared (new piece is airborne at spawn).
    assert!(
        gs.lock_state.is_none(),
        "lock_state must be None after hold (new piece is not grounded)"
    );
}

// ── hold_on_spawn_block_out_game_overs ───────────────────────────────────────

/// When the piece returning from hold would spawn into occupied cells,
/// the game must transition to GameOver(BlockOut).
#[test]
fn hold_on_spawn_block_out_game_overs() {
    let (mut gs, _clock) = make_game(42);

    // Do an initial hold so the hold slot is populated.
    gs.step(Duration::ZERO, &[Input::Hold]);
    // Lock current piece so we can hold again next cycle.
    gs.step(Duration::ZERO, &[Input::HardDrop]);
    for _ in 0..30 {
        tick(&mut gs, Duration::from_millis(10));
        if gs.active.is_some() && !gs.hold_used_this_cycle {
            break;
        }
    }
    // At this point hold has the first piece kind.
    let held_kind = gs.hold.unwrap();

    // Fill the spawn rows so the held piece cannot spawn.
    // Spawn position: most pieces at (col=3, row=18), cells at rows 18-19.
    // Fill rows 18 and 19 completely.
    for col in 0..10usize {
        gs.board.set(col, 18, blocktxt::game::piece::PieceKind::O);
        gs.board.set(col, 19, blocktxt::game::piece::PieceKind::O);
    }

    // Trigger hold: the held kind should try to spawn into occupied cells.
    gs.step(Duration::ZERO, &[Input::Hold]);

    assert!(
        matches!(
            gs.phase,
            Phase::GameOver {
                reason: GameOverReason::BlockOut
            }
        ),
        "hold spawning into occupied cells must trigger BlockOut game-over \
         (held={held_kind:?}, phase={:?})",
        gs.phase
    );
}

// ── hold_on_title_or_paused_does_nothing ─────────────────────────────────────

/// Hold is a no-op when not in Playing phase.
#[test]
fn hold_on_title_or_paused_does_nothing() {
    // Title phase.
    let clock = FakeClock::new(Instant::now());
    let mut gs = GameState::new(42, Box::new(clock));
    assert!(matches!(gs.phase, Phase::Title));
    gs.step(Duration::ZERO, &[Input::Hold]);
    assert!(gs.hold.is_none(), "hold must not work in Title phase");
    assert!(matches!(gs.phase, Phase::Title), "phase must stay Title");

    // Paused phase.
    let (mut gs, _clock) = make_game(42);
    gs.step(Duration::ZERO, &[Input::Pause]);
    assert!(matches!(gs.phase, Phase::Paused));
    gs.step(Duration::ZERO, &[Input::Hold]);
    assert!(gs.hold.is_none(), "hold must not work in Paused phase");
    assert!(matches!(gs.phase, Phase::Paused), "phase must stay Paused");
}

// ── hold_during_line_clear_does_nothing ──────────────────────────────────────

/// Hold input during a line-clear animation must be ignored.
#[test]
fn hold_during_line_clear_does_nothing() {
    use blocktxt::{LineClearAnim, LineClearPhase};

    let clock = FakeClock::new(Instant::now());
    let mut gs = GameState::new(42, Box::new(clock.clone()));
    gs.step(Duration::ZERO, &[Input::StartGame]);

    // Fill row 39 (bottom) completely to trigger a line-clear anim.
    for col in 0..10usize {
        gs.board.set(col, 39, blocktxt::game::piece::PieceKind::I);
    }
    // Inject anim directly (simpler than playing to clear).
    gs.line_clear_anim = Some(LineClearAnim {
        rows: vec![39],
        started_at: clock.now(),
        phase: LineClearPhase::Flash,
        board_snapshot: gs.board.clone(),
        pending_count: 1,
        pending_level_before: 1,
        pending_b2b_active: false,
    });

    let hold_before = gs.hold;
    let active_kind_before = gs.active.map(|p| p.kind);

    // Send Hold during the animation.
    gs.step(Duration::ZERO, &[Input::Hold]);

    assert_eq!(
        gs.hold, hold_before,
        "hold must not change during line-clear animation"
    );
    assert_eq!(
        gs.active.map(|p| p.kind),
        active_kind_before,
        "active kind must not change during line-clear animation"
    );
}
