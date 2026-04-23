//! Pure game-logic helpers (no I/O).
//!
//! SPEC §4 authoritative formulas implemented here:
//!
//! - **Gravity:** `gravity_seconds_per_cell = (0.8 − (level−1) × 0.007)
//!   ^ (level−1)`, clamped at level 20.
//! - **Soft-drop cap:** `effective_dt = max(natural_gravity / 20, 30 ms/cell)`.
//! - **Scoring:** 1→100×L, 2→300×L, 3→500×L, 4→800×L (×1.5 if B2B chain).
//! - **B2B:** 4-line clear starts/continues the chain; non-4-line clears
//!   break it; 0-line locks do NOT change B2B state.
//! - **Level:** every 10 lines → +1 level; max level 20 for gravity.
//! - **Lock delay:** 500 ms timer, up to 15 grounded-move resets per piece.

use std::time::Duration;

// ── Gravity ───────────────────────────────────────────────────────────────────

/// Return the natural fall duration per cell for the given level.
///
/// Formula (Guideline-inspired): `(0.8 − (level−1) × 0.007)^(level−1)` seconds.
/// Clamped so that levels above 20 use the level-20 value.
pub fn gravity_duration(level: u8) -> Duration {
    let l = level.min(20) as f64;
    let secs = (0.8 - (l - 1.0) * 0.007_f64).powf(l - 1.0);
    // Clamp to at least 1 µs to avoid zero (unreachable in practice).
    Duration::from_secs_f64(secs.max(0.000_001))
}

/// Return the effective fall duration per cell while soft-drop is held.
///
/// SPEC §4 (authoritative): `max(natural_gravity / 20, 30 ms/cell)`.
pub fn soft_drop_effective_dt(level: u8) -> Duration {
    let natural = gravity_duration(level);
    let divided = natural / 20;
    let floor = Duration::from_millis(30);
    divided.max(floor)
}

// ── Scoring ───────────────────────────────────────────────────────────────────

/// Scoring table per line-clear count (Guideline-style, no T-spin in v0.1).
///
/// Returns `(score_delta, new_b2b_active)`.
///
/// Rules (SPEC §4 authoritative):
/// - 0 lines locked: 0 pts, B2B state **unchanged**.
/// - 1 line: 100×level, **breaks** B2B (b2b_active → false).
/// - 2 lines: 300×level, **breaks** B2B.
/// - 3 lines: 500×level, **breaks** B2B.
/// - 4-line clear: 800×level; if B2B was already active → ×1.5 = 1200×level.
///   Sets b2b_active → true regardless.
pub fn score_line_clear(lines: u8, level: u8, b2b_going_in: bool) -> (u32, bool) {
    let lv = u32::from(level);
    match lines {
        0 => (0, b2b_going_in),
        1 => (100 * lv, false),
        2 => (300 * lv, false),
        3 => (500 * lv, false),
        4 => {
            let base = 800 * lv;
            let delta = if b2b_going_in {
                // B2B bonus: 800 × level × 1.5 = 1200 × level.
                // Use integer arithmetic: 800 * 3 / 2 = 1200.
                base * 3 / 2
            } else {
                base
            };
            (delta, true)
        }
        _ => (0, b2b_going_in), // unreachable in standard play
    }
}

// ── Level progression ─────────────────────────────────────────────────────────

/// Level after clearing `total_lines` total lines.
///
/// Starts at level 1; +1 every 10 lines cleared. No cap on score level
/// (gravity clamps internally at 20).
pub fn level_after_lines(total_lines: u32) -> u8 {
    let level = 1 + (total_lines / 10);
    level.min(u32::from(u8::MAX)) as u8
}

// ── Lock state ────────────────────────────────────────────────────────────────

/// Per-piece lock-delay state (SPEC §4 authoritative).
///
/// - Timer starts when a piece first touches the floor/stack.
/// - Each successful grounded move/rotation resets the timer, up to
///   `RESET_CAP` total resets.
/// - Airborne: timer pauses, reset counter preserved.
/// - At cap: piece locks on the next ground-touch.
/// - Timer expiry while airborne: no-op.
pub const LOCK_DELAY: Duration = Duration::from_millis(500);
pub const RESET_CAP: u8 = 15;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockState {
    /// How long the piece has been continuously grounded this touch.
    pub elapsed: Duration,
    /// How many grounded resets have been consumed so far.
    pub resets_used: u8,
}

impl LockState {
    pub fn new() -> Self {
        Self {
            elapsed: Duration::ZERO,
            resets_used: 0,
        }
    }

    /// Reset the 500 ms timer (called on a successful grounded move/rotation).
    /// Returns true if the reset was accepted (under cap); false if capped.
    pub fn reset_timer(&mut self) -> bool {
        if self.resets_used < RESET_CAP {
            self.elapsed = Duration::ZERO;
            self.resets_used += 1;
            true
        } else {
            false
        }
    }

    /// Advance the elapsed timer by `dt` while grounded.
    /// Returns true if the piece should now lock (elapsed ≥ LOCK_DELAY).
    pub fn advance(&mut self, dt: Duration) -> bool {
        self.elapsed += dt;
        self.elapsed >= LOCK_DELAY
    }

    /// Returns true if the reset cap has been reached, meaning the piece
    /// must lock on the next ground-touch regardless of the timer.
    pub fn is_capped(&self) -> bool {
        self.resets_used >= RESET_CAP
    }
}

impl Default for LockState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gravity_level1_approx_1000ms() {
        let d = gravity_duration(1);
        // Level 1: (0.8 - 0*0.007)^0 = 0.8^0 = 1.0 s
        assert_eq!(d, Duration::from_secs(1));
    }

    #[test]
    fn gravity_is_monotone_decreasing() {
        let mut prev = gravity_duration(1);
        for l in 2..=20u8 {
            let cur = gravity_duration(l);
            assert!(
                cur < prev,
                "gravity not decreasing at level {l}: {cur:?} >= {prev:?}"
            );
            prev = cur;
        }
    }

    #[test]
    fn gravity_clamps_at_level_20() {
        assert_eq!(gravity_duration(20), gravity_duration(25));
    }

    #[test]
    fn soft_drop_cap_floor_30ms() {
        // At high levels natural/20 < 30 ms; effective must be exactly 30 ms.
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
    fn soft_drop_level1_is_natural_div_20() {
        let nat = gravity_duration(1); // 1000 ms
        let eff = soft_drop_effective_dt(1);
        // 1000ms / 20 = 50ms > 30ms → should be 50ms
        assert_eq!(eff, nat / 20);
        assert_eq!(eff, Duration::from_millis(50));
    }
}
