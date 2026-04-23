//! DAS/ARR input model tests (SPEC §5 test 12).
//!
//! Parameterized over two timing profiles. A `FakeClock`-equivalent is
//! simulated by manually advancing `Instant` via the scripted approach:
//! we construct a base `Instant` and offset it with `Duration`.
//!
//! All timing is driven by `Instant::now()` values synthesised from a
//! fixed reference point so tests are fully deterministic.

use std::time::{Duration, Instant};

use blocktxt::input::{HeldDir, InputTranslator, KittySupport};
use blocktxt::Input;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

// ── helpers ───────────────────────────────────────────────────────────────

/// Create a key-press event for the given code.
fn key_press(code: KeyCode) -> crossterm::event::Event {
    crossterm::event::Event::Key(KeyEvent {
        code,
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: crossterm::event::KeyEventState::empty(),
    })
}

/// Create a key-release event for the given code.
fn key_release(code: KeyCode) -> crossterm::event::Event {
    crossterm::event::Event::Key(KeyEvent {
        code,
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Release,
        state: crossterm::event::KeyEventState::empty(),
    })
}

/// Timing profile for parameterized tests.
struct Profile {
    das: Duration,
    arr: Duration,
    release_infer: Duration,
}

fn profile_default() -> Profile {
    Profile {
        das: Duration::from_millis(160),
        arr: Duration::from_millis(30),
        release_infer: Duration::from_millis(160),
    }
}

fn profile_slow() -> Profile {
    Profile {
        das: Duration::from_millis(250),
        arr: Duration::from_millis(50),
        release_infer: Duration::from_millis(250),
    }
}

fn make_translator(p: &Profile) -> InputTranslator {
    InputTranslator::with_timing(KittySupport::Heuristic, p.das, p.arr, p.release_infer)
}

// ── tests ─────────────────────────────────────────────────────────────────

/// Pressing Left emits one immediate MoveLeft (no DAS yet).
#[test]
fn initial_press_emits_one_move() {
    let p = profile_default();
    let mut t = make_translator(&p);
    let base = Instant::now();

    let (inputs, quit) = t.translate_event(&key_press(KeyCode::Left), base);
    assert!(!quit);
    assert_eq!(inputs, vec![Input::MoveLeft]);

    // No repeat yet — DAS window not elapsed.
    let mut ticked = Vec::new();
    t.tick(base + Duration::from_millis(100), &mut ticked);
    assert!(ticked.is_empty(), "should be empty before DAS: {ticked:?}");
}

/// After DAS elapses, ARR repeats kick in at the correct rate.
///
/// Parameterized over default and slow profiles.
fn das_arr_repeat_inner(p: Profile) {
    let mut t = make_translator(&p);
    let base = Instant::now();

    let (first, _) = t.translate_event(&key_press(KeyCode::Right), base);
    assert_eq!(first, vec![Input::MoveRight], "initial press");

    // Nothing before DAS.
    let mut buf = Vec::new();
    t.tick(base + p.das - Duration::from_millis(1), &mut buf);
    assert!(buf.is_empty(), "no repeat before DAS");

    // First repeat exactly at DAS boundary.
    t.tick(base + p.das, &mut buf);
    assert_eq!(buf, vec![Input::MoveRight], "first ARR repeat at DAS");
    buf.clear();

    // Second repeat at DAS + ARR.
    t.tick(base + p.das + p.arr, &mut buf);
    assert_eq!(buf, vec![Input::MoveRight], "second ARR repeat");
    buf.clear();

    // Third repeat.
    t.tick(base + p.das + p.arr * 2, &mut buf);
    assert_eq!(buf, vec![Input::MoveRight], "third ARR repeat");
}

#[test]
fn das_arr_repeat_default_profile() {
    das_arr_repeat_inner(profile_default());
}

#[test]
fn das_arr_repeat_slow_profile() {
    das_arr_repeat_inner(profile_slow());
}

/// Release-inference timeout: no new event for `release_infer` ms → held cleared.
fn release_infer_inner(p: Profile) {
    let mut t = make_translator(&p);
    let base = Instant::now();

    let (first, _) = t.translate_event(&key_press(KeyCode::Left), base);
    assert_eq!(first, vec![Input::MoveLeft]);

    // Held is set.
    assert!(t.held.is_some());

    // Simulate passing release_infer + 1 ms with no new events.
    // The threshold is strictly greater-than, so we need one extra ms.
    let mut buf = Vec::new();
    t.tick(base + p.release_infer + Duration::from_millis(1), &mut buf);

    // Key should now be treated as released.
    assert!(
        t.held.is_none(),
        "held should be cleared after release-infer timeout"
    );
}

#[test]
fn release_infer_timeout_default() {
    release_infer_inner(profile_default());
}

#[test]
fn release_infer_timeout_slow() {
    release_infer_inner(profile_slow());
}

/// Soft-drop: SoftDropOn on first press, SoftDropOff via release-inference.
#[test]
fn soft_drop_edge_events_via_inference() {
    let p = profile_default();
    let mut t = make_translator(&p);
    let base = Instant::now();

    let (inputs, _) = t.translate_event(&key_press(KeyCode::Down), base);
    assert_eq!(
        inputs,
        vec![Input::SoftDropOn],
        "first down press → SoftDropOn"
    );

    // Press again quickly — no second SoftDropOn.
    let (inputs2, _) =
        t.translate_event(&key_press(KeyCode::Down), base + Duration::from_millis(20));
    assert!(
        !inputs2.contains(&Input::SoftDropOn),
        "no second SoftDropOn while held"
    );

    // Wait for release-inference from the SECOND press (at +20 ms).
    // Must tick past: +20ms + release_infer + 1ms.
    let mut buf = Vec::new();
    let tick_time = base + Duration::from_millis(20) + p.release_infer + Duration::from_millis(1);
    t.tick(tick_time, &mut buf);
    assert!(
        buf.contains(&Input::SoftDropOff),
        "SoftDropOff via release-inference: {buf:?}"
    );
}

/// Kitty mode: release event immediately clears held direction.
#[test]
fn kitty_release_clears_held() {
    let mut t = InputTranslator::with_timing(
        KittySupport::Supported,
        Duration::from_millis(160),
        Duration::from_millis(30),
        Duration::from_millis(160),
    );
    let base = Instant::now();

    let _ = t.translate_event(&key_press(KeyCode::Left), base);
    assert!(t.held.is_some());

    // Real release event.
    let (_, _) = t.translate_event(
        &key_release(KeyCode::Left),
        base + Duration::from_millis(50),
    );
    assert!(t.held.is_none(), "kitty release should clear held");
}

/// Direction change: pressing right while left is held switches direction.
#[test]
fn direction_change_switches_held() {
    let p = profile_default();
    let mut t = make_translator(&p);
    let base = Instant::now();

    let (first, _) = t.translate_event(&key_press(KeyCode::Left), base);
    assert_eq!(first, vec![Input::MoveLeft]);
    assert_eq!(t.held.as_ref().unwrap().kind, HeldDir::Left);

    // Switch to right.
    let (second, _) =
        t.translate_event(&key_press(KeyCode::Right), base + Duration::from_millis(50));
    assert_eq!(second, vec![Input::MoveRight]);
    assert_eq!(t.held.as_ref().unwrap().kind, HeldDir::Right);
}

/// Full 500 ms scripted run: press Left at t=0, hold for 400 ms, then
/// stop events for 200 ms.  Assert total repeats and release inference.
#[test]
fn scripted_500ms_run() {
    let p = profile_default();
    let mut t = make_translator(&p);
    let base = Instant::now();

    // t=0: press
    let (init, _) = t.translate_event(&key_press(KeyCode::Left), base);
    assert_eq!(init.len(), 1);

    let mut total_repeats = 0usize;

    // Simulate ticks every 10 ms up to 400 ms (while key is "held").
    let mut ms = 10u64;
    while ms <= 400 {
        let now = base + Duration::from_millis(ms);
        // Simulate re-press every 40 ms to prevent release inference.
        if ms % 40 == 0 {
            t.translate_event(&key_press(KeyCode::Left), now);
        }
        let mut buf = Vec::new();
        t.tick(now, &mut buf);
        total_repeats += buf.iter().filter(|i| **i == Input::MoveLeft).count();
        ms += 10;
    }

    // There should be some ARR repeats after DAS (160 ms DAS, 30 ms ARR).
    // From 160 ms to 400 ms = 240 ms / 30 ms = ~8 repeats.
    assert!(
        total_repeats >= 5,
        "expected ≥5 ARR repeats in 400 ms, got {total_repeats}"
    );

    // Now let 200 ms pass with no events → release inference.
    let mut buf = Vec::new();
    t.tick(base + Duration::from_millis(600), &mut buf);
    assert!(
        t.held.is_none(),
        "should release-infer after 200 ms silence"
    );
}
