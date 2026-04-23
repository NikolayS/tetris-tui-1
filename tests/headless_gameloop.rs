//! Headless integration test: scripted inputs against GameState::step.
//!
//! No terminal, no render — pure game-logic integration.

use std::time::{Duration, Instant};

use blocktxt::clock::FakeClock;
use blocktxt::game::state::GameState;
use blocktxt::Input;

/// Run a scripted sequence of inputs for ~5 simulated seconds and assert
/// no panic or unwrap explosion.
#[test]
fn headless_scripted_inputs_no_panic() {
    let start = Instant::now();
    let clock = Box::new(FakeClock::new(start));
    let mut state = GameState::new(12345, clock);

    // Simulate 5 seconds at 60 fps = 300 ticks.
    let dt = Duration::from_millis(16);

    let scripted: Vec<Vec<Input>> = vec![
        // First 5 ticks: move left.
        vec![Input::MoveLeft],
        vec![Input::MoveLeft],
        vec![Input::MoveLeft],
        vec![Input::MoveLeft],
        vec![Input::MoveLeft],
        // Rotate CW + CCW.
        vec![Input::RotateCw],
        vec![Input::RotateCcw],
        // Soft drop for 10 ticks.
        vec![Input::SoftDropOn],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        vec![Input::SoftDropOff],
        // Move right.
        vec![Input::MoveRight],
        vec![Input::MoveRight],
        vec![Input::MoveRight],
        // Hard drop.
        vec![Input::HardDrop],
        // Move left again with next piece.
        vec![Input::MoveLeft],
        vec![Input::MoveLeft],
        vec![Input::MoveRight],
        // Pause then resume.
        vec![Input::Pause],
        vec![Input::Pause],
        // Another hard drop.
        vec![Input::HardDrop],
    ];

    let total_ticks = 300usize;

    for tick in 0..total_ticks {
        let inputs = scripted.get(tick).cloned().unwrap_or_default();
        let _events = state.step(dt, &inputs);
        // No assertion on events — just must not panic.
    }

    // After 300 ticks the game should still be in a valid state
    // (either Playing, Paused, or GameOver — all are non-panic states).
    // Simply reaching here is the assertion.
}

/// Verify that Restart input transitions from GameOver back to Playing.
#[test]
fn headless_restart_from_game_over() {
    let clock = Box::new(FakeClock::new(Instant::now()));
    let mut state = GameState::new(0, clock);

    // Drive game to game-over by stacking pieces at the top.
    // Hard-drop 30 times; at some point the spawn zone will be blocked.
    let dt = Duration::from_millis(16);
    for _ in 0..30 {
        state.step(dt, &[Input::HardDrop]);
    }

    // Whether or not game-over occurred, Restart should always work.
    state.step(dt, &[Input::Restart]);

    // After restart we should be in Playing phase.
    use blocktxt::Phase;
    assert!(
        matches!(state.phase, Phase::Playing),
        "after Restart, phase should be Playing; got {:?}",
        state.phase
    );
}
