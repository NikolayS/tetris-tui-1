//! Unit tests for the pure helpers in `game::rules`.

use blocktxt::game::rules::{
    gravity_duration, level_after_lines, score_line_clear, soft_drop_effective_dt, LockState,
    LOCK_DELAY, RESET_CAP,
};
use std::time::Duration;

// ── gravity_duration ──────────────────────────────────────────────────────────

#[test]
fn gravity_level1_is_1000ms() {
    // Level 1: (0.8 - 0*0.007)^0 = 1.0 s exactly.
    assert_eq!(gravity_duration(1), Duration::from_secs(1));
}

#[test]
fn gravity_level5_spot_check() {
    // (0.8 - 4*0.007)^4 = (0.772)^4 ≈ 0.3548 s.
    let d = gravity_duration(5);
    let ms = d.as_secs_f64() * 1000.0;
    assert!(
        (354.0..=356.0).contains(&ms),
        "gravity(5) = {ms:.1}ms, expected ~355ms"
    );
}

#[test]
fn gravity_level10_spot_check() {
    // (0.8 - 9*0.007)^9 = (0.737)^9 ≈ 64.15 ms.
    let d = gravity_duration(10);
    let ms = d.as_secs_f64() * 1000.0;
    assert!(
        (62.0..=66.0).contains(&ms),
        "gravity(10) = {ms:.1}ms, expected ~64ms"
    );
}

#[test]
fn gravity_level15_spot_check() {
    // (0.8 - 14*0.007)^14 = (0.702)^14 ≈ 7.06 ms.
    let d = gravity_duration(15);
    let ms = d.as_secs_f64() * 1000.0;
    assert!(
        (6.0..=8.0).contains(&ms),
        "gravity(15) = {ms:.1}ms, expected ~7ms"
    );
}

#[test]
fn gravity_is_monotone_decreasing() {
    let mut prev = gravity_duration(1);
    for l in 2..=20u8 {
        let cur = gravity_duration(l);
        assert!(cur < prev, "not decreasing at level {l}");
        prev = cur;
    }
}

#[test]
fn gravity_clamped_at_level_20() {
    assert_eq!(gravity_duration(20), gravity_duration(21));
    assert_eq!(gravity_duration(20), gravity_duration(30));
}

// ── soft_drop_effective_dt ────────────────────────────────────────────────────

#[test]
fn soft_drop_level1_is_natural_div_20() {
    // Level 1: natural = 1000ms → 1000/20 = 50ms > 30ms floor → 50ms.
    let natural = gravity_duration(1);
    let eff = soft_drop_effective_dt(1);
    assert_eq!(eff, natural / 20);
    assert_eq!(eff, Duration::from_millis(50));
}

#[test]
fn soft_drop_cap_floor_30ms_at_high_levels() {
    // At levels ≥ 15 natural gravity / 20 < 30ms; floor kicks in.
    for level in 15..=20u8 {
        let eff = soft_drop_effective_dt(level);
        assert_eq!(
            eff,
            Duration::from_millis(30),
            "level {level}: expected 30ms floor, got {eff:?}"
        );
    }
}

#[test]
fn soft_drop_always_at_least_30ms() {
    for level in 1..=20u8 {
        let eff = soft_drop_effective_dt(level);
        assert!(
            eff >= Duration::from_millis(30),
            "level {level}: {eff:?} < 30ms floor"
        );
    }
}

// ── score_line_clear ──────────────────────────────────────────────────────────

#[test]
fn scoring_empty_lock_preserves_b2b_state() {
    // 0 lines cleared: score = 0, B2B unchanged.
    let (delta, new_b2b) = score_line_clear(0, 1, false);
    assert_eq!(delta, 0);
    assert!(!new_b2b);

    let (delta, new_b2b) = score_line_clear(0, 1, true);
    assert_eq!(delta, 0);
    assert!(new_b2b, "empty lock must not break B2B chain");
}

#[test]
fn scoring_single_100x_level_breaks_b2b() {
    for level in [1u8, 5, 10] {
        let (delta, new_b2b) = score_line_clear(1, level, true);
        assert_eq!(delta, 100 * u32::from(level));
        assert!(!new_b2b, "single must break B2B");
    }
}

#[test]
fn scoring_double_300x_level_breaks_b2b() {
    for level in [1u8, 5, 10] {
        let (delta, new_b2b) = score_line_clear(2, level, true);
        assert_eq!(delta, 300 * u32::from(level));
        assert!(!new_b2b, "double must break B2B");
    }
}

#[test]
fn scoring_triple_500x_level_breaks_b2b() {
    for level in [1u8, 5, 10] {
        let (delta, new_b2b) = score_line_clear(3, level, true);
        assert_eq!(delta, 500 * u32::from(level));
        assert!(!new_b2b, "triple must break B2B");
    }
}

#[test]
fn scoring_4line_clear_800x_first_sets_b2b() {
    // First 4-line clear (b2b_going_in = false): 800×level, sets b2b.
    let (delta, new_b2b) = score_line_clear(4, 1, false);
    assert_eq!(delta, 800);
    assert!(new_b2b);

    let (delta, new_b2b) = score_line_clear(4, 3, false);
    assert_eq!(delta, 2400);
    assert!(new_b2b);
}

#[test]
fn scoring_4line_clear_b2b_is_1_5x() {
    // Second consecutive 4-line clear (b2b_going_in = true): 1200×level.
    let (delta, new_b2b) = score_line_clear(4, 1, true);
    assert_eq!(delta, 1200, "B2B 4-line clear at L1 must be 1200");
    assert!(new_b2b);

    let (delta, new_b2b) = score_line_clear(4, 2, true);
    assert_eq!(delta, 2400, "B2B 4-line clear at L2 must be 2400");
    assert!(new_b2b);

    let (delta, new_b2b) = score_line_clear(4, 5, true);
    assert_eq!(delta, 6000, "B2B 4-line clear at L5 must be 6000");
    assert!(new_b2b);
}

#[test]
fn b2b_chain_breaks_on_non_4line() {
    // Establish B2B.
    let (_, b2b) = score_line_clear(4, 1, false);
    assert!(b2b);
    // Break with a single.
    let (_, b2b) = score_line_clear(1, 1, b2b);
    assert!(!b2b);
    // Next 4-line clear is not B2B.
    let (delta, b2b) = score_line_clear(4, 1, b2b);
    assert_eq!(delta, 800, "first 4-line after chain break = 800");
    assert!(b2b);
}

// ── level_after_lines ─────────────────────────────────────────────────────────

#[test]
fn level_starts_at_1() {
    assert_eq!(level_after_lines(0), 1);
    assert_eq!(level_after_lines(9), 1);
}

#[test]
fn level_every_10_lines() {
    assert_eq!(level_after_lines(10), 2);
    assert_eq!(level_after_lines(19), 2);
    assert_eq!(level_after_lines(20), 3);
    assert_eq!(level_after_lines(99), 10);
    assert_eq!(level_after_lines(100), 11);
}

// ── LockState ─────────────────────────────────────────────────────────────────

#[test]
fn lock_state_expires_after_500ms() {
    let mut ls = LockState::new();
    assert!(!ls.advance(Duration::from_millis(499)));
    assert!(ls.advance(Duration::from_millis(1))); // now ≥ 500ms
}

#[test]
fn lock_state_reset_clears_timer() {
    let mut ls = LockState::new();
    ls.advance(Duration::from_millis(400));
    assert!(ls.reset_timer()); // accepted
    assert!(!ls.advance(Duration::from_millis(400))); // not expired yet
}

#[test]
fn lock_state_reset_cap_is_15() {
    assert_eq!(RESET_CAP, 15);
    let mut ls = LockState::new();
    for i in 0..15 {
        assert!(ls.reset_timer(), "reset {i} should be accepted");
    }
    // 16th reset rejected.
    assert!(!ls.reset_timer(), "16th reset must be rejected (cap)");
    assert!(ls.is_capped());
}

#[test]
fn lock_delay_constant_is_500ms() {
    assert_eq!(LOCK_DELAY, Duration::from_millis(500));
}
