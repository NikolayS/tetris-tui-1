//! DAS/ARR boundary sweep stress tests (SPEC §5 / Sprint 4 Track D).
//!
//! Sweeps `held_ms` from 0 to 500 in 10 ms steps and asserts the timing
//! contract at each boundary.  Uses `InputTranslator` directly with
//! synthesised `Instant` offsets (no real sleeps).

use std::time::{Duration, Instant};

use blocktxt::input::{InputTranslator, KittySupport};
use blocktxt::Input;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

// ── helpers ───────────────────────────────────────────────────────────────

const DAS_MS: u64 = 160;
const ARR_MS: u64 = 30;
const RELEASE_INFER_MS: u64 = 160;

fn make_translator() -> InputTranslator {
    InputTranslator::with_timing(
        KittySupport::Heuristic,
        Duration::from_millis(DAS_MS),
        Duration::from_millis(ARR_MS),
        Duration::from_millis(RELEASE_INFER_MS),
    )
}

fn key_press(code: KeyCode) -> crossterm::event::Event {
    crossterm::event::Event::Key(KeyEvent {
        code,
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: crossterm::event::KeyEventState::empty(),
    })
}

// ── sweep tests ───────────────────────────────────────────────────────────

/// For every held duration from 0 to DAS-1, no Move* should be emitted
/// by `tick()` (only the initial press emits one move, not tick repeats).
#[test]
fn no_repeat_before_das_boundary() {
    let base = Instant::now();

    for held_ms in (0..DAS_MS).step_by(10) {
        let mut t = make_translator();

        // Initial press.
        let (init, _) = t.translate_event(&key_press(KeyCode::Left), base);
        assert_eq!(
            init,
            vec![Input::MoveLeft],
            "held_ms={held_ms}: initial press must emit MoveLeft"
        );

        // Tick at held_ms — must produce no repeat before DAS.
        let mut buf = Vec::new();
        t.tick(base + Duration::from_millis(held_ms), &mut buf);
        assert!(
            buf.is_empty(),
            "held_ms={held_ms}: tick before DAS should not emit repeat; got {buf:?}"
        );
    }
}

/// At exactly DAS ms, exactly one Move* repeat is emitted.
#[test]
fn exactly_one_move_at_das_boundary() {
    let base = Instant::now();

    for step in [KeyCode::Left, KeyCode::Right] {
        let mut t = make_translator();
        let (_, _) = t.translate_event(&key_press(step), base);

        let mut buf = Vec::new();
        // One tick just before DAS — no repeat.
        t.tick(base + Duration::from_millis(DAS_MS - 1), &mut buf);
        assert!(
            buf.is_empty(),
            "{step:?}: tick at DAS-1 should produce nothing; got {buf:?}"
        );

        // One tick at exactly DAS — exactly one repeat.
        t.tick(base + Duration::from_millis(DAS_MS), &mut buf);
        let expected = if step == KeyCode::Left {
            Input::MoveLeft
        } else {
            Input::MoveRight
        };
        assert_eq!(
            buf,
            vec![expected],
            "{step:?}: tick at DAS must emit exactly one repeat"
        );
    }
}

/// After DAS, repeated ticks every ARR ms each produce exactly one Move*.
#[test]
fn arr_repeats_at_correct_interval() {
    let base = Instant::now();
    let mut t = make_translator();
    let (_, _) = t.translate_event(&key_press(KeyCode::Right), base);

    // DAS first repeat.
    let mut buf = Vec::new();
    t.tick(base + Duration::from_millis(DAS_MS), &mut buf);
    assert_eq!(buf, vec![Input::MoveRight], "first ARR at DAS");
    buf.clear();

    // 10 ARR repeats after DAS.
    for n in 1u64..=10 {
        t.tick(base + Duration::from_millis(DAS_MS + n * ARR_MS), &mut buf);
        assert_eq!(
            buf,
            vec![Input::MoveRight],
            "ARR repeat #{n} should be exactly one MoveRight"
        );
        buf.clear();
    }
}

/// Between ARR boundaries, no extra move is emitted.
#[test]
fn no_extra_move_between_arr_boundaries() {
    let base = Instant::now();
    let mut t = make_translator();
    let (_, _) = t.translate_event(&key_press(KeyCode::Left), base);

    // Consume DAS boundary.
    let mut buf = Vec::new();
    t.tick(base + Duration::from_millis(DAS_MS), &mut buf);
    buf.clear();

    // Tick halfway through the first ARR window — no extra move.
    t.tick(
        base + Duration::from_millis(DAS_MS + ARR_MS / 2),
        &mut buf,
    );
    assert!(
        buf.is_empty(),
        "mid-ARR tick should not emit move; got {buf:?}"
    );
}

/// Release-inference after key is held for less than DAS:
/// stop sending press events, advance clock past release_infer threshold,
/// assert exactly one synthetic SoftDropOff (or held cleared) is emitted.
///
/// For directional keys the release-infer just clears `held`, emitting no
/// extra event.  This test verifies `held` is cleared.
#[test]
fn release_inference_clears_held_before_das() {
    let base = Instant::now();

    // Sweep held_ms values strictly below DAS so no repeat fires.
    for held_ms in (10u64..DAS_MS).step_by(20) {
        let mut t = make_translator();
        let (init, _) = t.translate_event(&key_press(KeyCode::Left), base);
        assert!(!init.is_empty());

        // Held is set.
        assert!(
            t.held.is_some(),
            "held_ms={held_ms}: held should be set after press"
        );

        // Advance to held_ms — still in window, no release yet.
        let mut buf = Vec::new();
        t.tick(base + Duration::from_millis(held_ms), &mut buf);
        assert!(
            t.held.is_some(),
            "held_ms={held_ms}: held still set at {held_ms}ms"
        );

        // Advance past release_infer from the press time.
        t.tick(
            base + Duration::from_millis(held_ms + RELEASE_INFER_MS + 1),
            &mut buf,
        );
        assert!(
            t.held.is_none(),
            "held_ms={held_ms}: held must clear after release-infer timeout"
        );
    }
}

/// Sweep held_ms 0..=500 in 10 ms steps, simulating a truly held key by
/// re-sending the press event every 10 ms to keep release-inference at bay.
///
///   - If held_ms < DAS  → no tick-repeat before DAS boundary.
///   - If held_ms >= DAS → at least one repeat emitted by the tick at DAS.
///
/// Each scenario is self-contained: we create a fresh translator, replay
/// 10 ms synthetic press events up to `held_ms`, then call one final tick.
#[test]
fn das_boundary_sweep_0_to_500() {
    let base = Instant::now();

    for held_ms in (0u64..=500).step_by(10) {
        let mut t = make_translator();

        // Send initial press at t=0.
        let (init, _) = t.translate_event(&key_press(KeyCode::Right), base);
        assert_eq!(
            init.len(),
            1,
            "held_ms={held_ms}: initial press must emit exactly one MoveRight"
        );

        // Re-send press every 10 ms up to held_ms to simulate a real held
        // key (prevents release-inference from clearing `held`).
        let mut step = 10u64;
        while step < held_ms {
            t.translate_event(&key_press(KeyCode::Right), base + Duration::from_millis(step));
            step += 10;
        }

        // Collect tick output across all 10 ms boundaries up to held_ms.
        // Running ticks at each step matches real game loop behaviour and
        // correctly drains the DAS/ARR repeat schedule.
        let mut buf = Vec::new();
        step = 10;
        while step <= held_ms {
            t.tick(base + Duration::from_millis(step), &mut buf);
            step += 10;
        }

        if held_ms < DAS_MS {
            assert!(
                buf.is_empty(),
                "held_ms={held_ms}: no repeat before DAS; got {buf:?}"
            );
        } else {
            // At or after DAS → at least one repeat across all ticks.
            assert!(
                !buf.is_empty(),
                "held_ms={held_ms}: expected ≥1 repeat at/after DAS"
            );
            for mv in &buf {
                assert_eq!(
                    *mv,
                    Input::MoveRight,
                    "held_ms={held_ms}: unexpected input {mv:?}"
                );
            }
        }
    }
}
