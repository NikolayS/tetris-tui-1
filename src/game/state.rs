//! Core game state machine.
//!
//! `GameState::step(dt, inputs)` is the single authoritative mutation point
//! for all game logic. No I/O occurs here; rendering and persistence read
//! `&GameState` separately.
//!
//! Clock is injected as `Box<dyn Clock>` so tests can use `FakeClock`
//! without monomorphising the entire state machine.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use rand::rngs::StdRng;
use rand::SeedableRng;

use crate::clock::Clock;
use crate::game::bag::Bag;
use crate::game::board::Board;
use crate::game::piece::{spawn, Piece, PieceKind};
use crate::game::rules::{
    gravity_duration, level_after_lines, score_line_clear, soft_drop_effective_dt, LockState,
};
use crate::game::srs::{rotate, RotationDir};

// ── Line-clear animation constants ───────────────────────────────────────────

/// Duration of the flash phase (phase 1): 0–100 ms.
pub const ANIM_FLASH_MS: u64 = 100;
/// Duration of the dim phase (phase 2): 100–200 ms.
pub const ANIM_DIM_MS: u64 = 100;
/// Total animation budget: ≤ 200 ms.
pub const ANIM_TOTAL_MS: u64 = ANIM_FLASH_MS + ANIM_DIM_MS;

// ── Spawn-fade animation constants ────────────────────────────────────────────

/// Phase 1 of spawn fade: 0–40 ms at 60% intensity.
pub const SPAWN_FADE1_MS: u64 = 40;
/// Phase 2 of spawn fade: 40–80 ms at 80% intensity.
pub const SPAWN_FADE2_MS: u64 = 40;
/// Total spawn-fade budget: 80 ms.
pub const SPAWN_FADE_TOTAL_MS: u64 = SPAWN_FADE1_MS + SPAWN_FADE2_MS;

// ── Score rollup animation constant ───────────────────────────────────────────

/// Duration over which the displayed score ticks up: 250 ms.
pub const SCORE_ROLLUP_MS: u64 = 250;

// ── Game-over zoom animation constant ─────────────────────────────────────────

/// Duration of the game-over overlay zoom-in: 200 ms.
pub const GAMEOVER_ZOOM_MS: u64 = 200;

// ── Line-clear animation state ────────────────────────────────────────────────

/// Three-phase line-clear animation driven by elapsed wall time.
///
/// Phase 1 (0–100 ms): cleared rows shown flashed (inverse/bright fill).
/// Phase 2 (100–200 ms): rows shown dimmer (transition).
/// Phase 3 (≥ 200 ms): rows removed, cells above shift down (handled by
///   transitioning `pending_clear` → actual `Board::clear_full_rows`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineClearPhase {
    /// Flash phase: `elapsed` < 100 ms.
    Flash,
    /// Dim/transition phase: 100 ms ≤ `elapsed` < 200 ms.
    Dim,
}

/// Animation state held in `GameState` while a line-clear plays out.
#[derive(Debug, Clone)]
pub struct LineClearAnim {
    /// Board row indices (0-based, absolute) being animated.
    pub rows: Vec<usize>,
    /// When the animation began (from the injected clock).
    pub started_at: Instant,
    /// Current visual phase.
    pub phase: LineClearPhase,
    /// Board snapshot before clearing (contains the full rows for rendering).
    pub board_snapshot: Board,
    /// Deferred line count (filled in `finish_anim`).
    pub pending_count: u8,
    /// Level before this clear (for scoring).
    pub pending_level_before: u8,
    /// B2B state before this clear (for scoring).
    pub pending_b2b_active: bool,
}

// ── Spawn-fade animation state ────────────────────────────────────────────────

/// Spawn-fade animation: tracks how long the newly spawned piece has existed.
///
/// - 0–40 ms  → render at 60% intensity.
/// - 40–80 ms → render at 80% intensity.
/// - ≥ 80 ms  → full intensity; animation done.
#[derive(Debug, Clone)]
pub struct SpawnAnim {
    /// When the piece became active (from the injected clock).
    pub started_at: Instant,
    /// The piece kind being faded in (for renderer to tint correctly).
    pub kind: PieceKind,
}

// ── Score rollup state ────────────────────────────────────────────────────────

/// Tracks the rolling display score shown in the HUD.
///
/// When a line-clear grants `score_delta`, `target` jumps immediately while
/// `current` ticks toward it over `SCORE_ROLLUP_MS` ms.
#[derive(Debug, Clone)]
pub struct ScoreDisplay {
    /// The value currently rendered in the HUD.
    pub current: u32,
    /// The value we are rolling toward.
    pub target: u32,
    /// When the last step advanced `current`.
    pub last_tick: Instant,
}

impl ScoreDisplay {
    /// Create a new display starting at zero.
    fn new(now: Instant) -> Self {
        Self {
            current: 0,
            target: 0,
            last_tick: now,
        }
    }

    /// Advance `current` toward `target` proportional to `dt`.
    ///
    /// Uses a linear interpolation: each millisecond of `dt` advances
    /// `current` by `(delta / SCORE_ROLLUP_MS)` points (at least 1 if there
    /// is any remaining delta, to guarantee eventual convergence).
    pub fn tick(&mut self, dt: Duration) {
        if self.current >= self.target {
            self.current = self.target;
            return;
        }
        let remaining = self.target - self.current;
        let rollup_ms = SCORE_ROLLUP_MS.max(1);
        let dt_ms = dt.as_millis() as u64;
        let advance = (remaining as u64 * dt_ms / rollup_ms) as u32;
        // Always advance at least 1 so we converge even for tiny dt.
        let advance = advance.max(1).min(remaining);
        self.current += advance;
    }

    /// Jump `target` by `delta` and record the current tick time.
    pub fn add_target(&mut self, delta: u32, now: Instant) {
        self.target = self.target.saturating_add(delta);
        self.last_tick = now;
    }
}

// ── Game-over zoom state ──────────────────────────────────────────────────────

/// Tracks the zoom-in animation for the game-over overlay.
///
/// `scale` goes from 0.5 → 1.0 over `GAMEOVER_ZOOM_MS` ms.
#[derive(Debug, Clone)]
pub struct GameOverZoom {
    /// When the game-over phase began (from the injected clock).
    pub started_at: Instant,
}

impl GameOverZoom {
    /// Return the current scale in [0.5, 1.0].
    pub fn scale(&self, now: Instant) -> f32 {
        let elapsed_ms = now.saturating_duration_since(self.started_at).as_millis() as f32;
        let t = (elapsed_ms / GAMEOVER_ZOOM_MS as f32).clamp(0.0, 1.0);
        // Scale from 0.5 → 1.0
        0.5 + 0.5 * t
    }
}

// Number of next-pieces kept in the preview queue.
const NEXT_QUEUE_LEN: usize = 5;

// ── Public enums ──────────────────────────────────────────────────────────────

/// Player inputs consumed by `GameState::step`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Input {
    MoveLeft,
    MoveRight,
    RotateCw,
    RotateCcw,
    SoftDropOn,
    SoftDropOff,
    HardDrop,
    Pause,
    Restart,
    /// Start the game from the Title screen.
    StartGame,
    /// Confirm reset-scores dialog (y).
    ConfirmYes,
    /// Cancel reset-scores dialog (n).
    ConfirmNo,
    /// Hold the active piece (Guideline §1a).
    Hold,
}

/// Events emitted by `GameState::step` for the application layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    LinesCleared {
        count: u8,
        b2b: bool,
        level_before: u8,
        level_after: u8,
        score_delta: u32,
    },
    PieceLocked,
    GameOver(GameOverReason),
    Paused,
    Resumed,
}

/// Reason for the game ending.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameOverReason {
    /// The spawn zone was occupied when a new piece tried to spawn.
    BlockOut,
    /// A piece locked while entirely above the visible playfield (row < 20).
    LockOut,
}

/// High-level game phase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Phase {
    /// Attract / title screen. No active piece.
    Title,
    Playing,
    Paused,
    GameOver {
        reason: GameOverReason,
    },
    /// Confirmation dialog: reset high scores?
    ConfirmResetScores,
}

// ── GameState ─────────────────────────────────────────────────────────────────

/// All mutable game state.  Mutation only via `step`; reads are public.
pub struct GameState {
    pub board: Board,
    pub active: Option<Piece>,
    pub next_queue: VecDeque<PieceKind>,
    pub score: u32,
    pub level: u8,
    pub lines_cleared: u32,
    pub b2b_active: bool,
    pub phase: Phase,

    /// The seed this game was started with; reused on Restart so bag order
    /// is reproducible (SPEC §4, #24).
    pub seed: u64,

    /// Hold slot: the kind of the piece currently held, if any.
    ///
    /// `None` until the player holds for the first time in a game.
    pub hold: Option<PieceKind>,

    /// True after a successful hold; prevents holding again until the
    /// current active piece locks (one hold per piece cycle).
    pub hold_used_this_cycle: bool,

    bag: Bag<StdRng>,
    pub lock_state: Option<LockState>,
    soft_drop_held: bool,
    gravity_acc: Duration,

    /// Active line-clear animation, if any.
    ///
    /// While `Some`, `board` still contains the full rows (for rendering).
    /// They are removed when the animation completes (phase 3 transition).
    pub line_clear_anim: Option<LineClearAnim>,

    /// Active spawn-fade animation, if any.
    ///
    /// Set whenever a new active piece spawns; cleared after 80 ms.
    pub spawn_anim: Option<SpawnAnim>,

    /// Rolling score display for the HUD.
    ///
    /// Ticks toward `score` over 250 ms. Renderer reads this instead of
    /// `state.score` directly for the animated rollup effect.
    pub score_display: ScoreDisplay,

    /// Game-over zoom animation, if any.
    ///
    /// Set when `phase` transitions to `Phase::GameOver`; renderer uses
    /// `scale()` to shrink the overlay at t=0 and grow to full at t=200 ms.
    pub gameover_zoom: Option<GameOverZoom>,

    // Injected clock — stored so tests can share the FakeClock handle.
    clock: Box<dyn Clock>,

    // Wall-clock reference point (last step).
    #[allow(dead_code)]
    last_tick: Instant,
}

impl GameState {
    /// Return the current clock instant.
    ///
    /// Used by the renderer to compute animation phases without taking a
    /// mutable borrow of the state.
    pub fn now(&self) -> std::time::Instant {
        self.clock.now()
    }

    /// Create a new game seeded with `seed`.
    ///
    /// The clock is used to initialise the starting `Instant`; all timing
    /// thereafter uses the `dt` passed to `step`.
    pub fn new(seed: u64, clock: Box<dyn Clock>) -> Self {
        let rng = StdRng::seed_from_u64(seed);
        let bag = Bag::new(rng);

        let now = clock.now();
        Self {
            board: Board::empty(),
            // No active piece until the player leaves the Title screen.
            active: None,
            next_queue: VecDeque::new(),
            score: 0,
            level: 1,
            lines_cleared: 0,
            b2b_active: false,
            phase: Phase::Title,
            seed,
            hold: None,
            hold_used_this_cycle: false,
            bag,
            lock_state: None,
            soft_drop_held: false,
            gravity_acc: Duration::ZERO,
            line_clear_anim: None,
            // No spawn-fade until the first piece actually spawns on StartGame.
            spawn_anim: None,
            score_display: ScoreDisplay::new(now),
            gameover_zoom: None,
            clock,
            last_tick: now,
        }
    }

    /// Initialise a fresh game round: fill the next queue and spawn the first
    /// piece. Called when transitioning Title → Playing for the first time, or
    /// on Restart.
    fn init_round(&mut self) {
        let rng = StdRng::seed_from_u64(self.seed);
        let mut bag = Bag::new(rng);
        let mut next_queue: VecDeque<PieceKind> =
            (0..NEXT_QUEUE_LEN).map(|_| bag.next().unwrap()).collect();
        let first_kind = next_queue.pop_front().unwrap();
        next_queue.push_back(bag.next().unwrap());
        let active = spawn(first_kind);

        let now = self.clock.now();
        self.board = Board::empty();
        self.active = Some(active);
        self.next_queue = next_queue;
        self.score = 0;
        self.level = 1;
        self.lines_cleared = 0;
        self.b2b_active = false;
        self.hold = None;
        self.hold_used_this_cycle = false;
        self.bag = bag;
        self.lock_state = None;
        self.soft_drop_held = false;
        self.gravity_acc = Duration::ZERO;
        self.line_clear_anim = None;
        // Spawn-fade for the very first piece of this round.
        self.spawn_anim = Some(SpawnAnim {
            started_at: now,
            kind: first_kind,
        });
        self.score_display = ScoreDisplay::new(now);
        self.gameover_zoom = None;
    }

    // ── step ────────────────────────────────────────────────────────────────

    /// Advance the game by `dt`, processing `inputs` in order.
    ///
    /// Returns events that occurred during this step.
    /// No-ops if in `Paused` or `GameOver` state (except Pause/Restart inputs).
    pub fn step(&mut self, dt: Duration, inputs: &[Input]) -> Vec<Event> {
        let mut events = Vec::new();

        match &self.phase {
            Phase::Title => {
                for &inp in inputs {
                    match inp {
                        // Any "action" input starts the game.
                        Input::StartGame
                        | Input::MoveLeft
                        | Input::MoveRight
                        | Input::RotateCw
                        | Input::RotateCcw
                        | Input::HardDrop
                        | Input::SoftDropOn => {
                            self.phase = Phase::Playing;
                            self.init_round();
                        }
                        _ => {}
                    }
                }
                return events;
            }
            Phase::ConfirmResetScores => {
                for &inp in inputs {
                    match inp {
                        Input::ConfirmYes | Input::ConfirmNo => {
                            // Both choices return to Title.
                            // The caller (main.rs) is responsible for
                            // clearing the store when ConfirmYes was pressed.
                            self.phase = Phase::Title;
                        }
                        _ => {}
                    }
                }
                return events;
            }
            Phase::GameOver { .. } => {
                // Restart goes back to Title (not directly into Playing).
                for &inp in inputs {
                    if inp == Input::Restart {
                        self.reset_to_title();
                    }
                }
                return events;
            }
            Phase::Paused => {
                for &inp in inputs {
                    if inp == Input::Pause {
                        self.phase = Phase::Playing;
                        events.push(Event::Resumed);
                    }
                }
                return events;
            }
            Phase::Playing => {}
        }

        // ── Tick spawn-fade animation ────────────────────────────────────────
        if let Some(ref sa) = self.spawn_anim {
            let now = self.clock.now();
            let elapsed = now.saturating_duration_since(sa.started_at);
            if elapsed >= Duration::from_millis(SPAWN_FADE_TOTAL_MS) {
                self.spawn_anim = None;
            }
        }

        // ── Advance score rollup ─────────────────────────────────────────────
        if self.score_display.current < self.score_display.target {
            self.score_display.tick(dt);
        }

        // ── Tick line-clear animation ────────────────────────────────────────
        // Animation runs concurrently with input handling (non-blocking).
        // When the animation completes we finalize the board clear and spawn
        // the next piece.
        if self.line_clear_anim.is_some() {
            self.tick_anim(&mut events);
            // While animating, skip gravity/lock so the board stays stable.
            if self.line_clear_anim.is_some() {
                return events;
            }
            // Animation just finished — next piece already spawned; done.
            return events;
        }

        // ── Handle inputs ────────────────────────────────────────────────────
        for &inp in inputs {
            self.handle_input(inp, &mut events);
            // If an input caused game-over, stop processing.
            if !matches!(self.phase, Phase::Playing) {
                return events;
            }
        }

        // ── Apply gravity ────────────────────────────────────────────────────
        self.apply_gravity(dt, &mut events);
        if !matches!(self.phase, Phase::Playing) {
            return events;
        }

        // ── Check lock ───────────────────────────────────────────────────────
        self.check_lock(dt, &mut events);

        events
    }

    // ── Input handling ───────────────────────────────────────────────────────

    fn handle_input(&mut self, inp: Input, events: &mut Vec<Event>) {
        match inp {
            Input::Pause => {
                self.phase = Phase::Paused;
                events.push(Event::Paused);
            }
            Input::Restart => self.reset_to_title(),
            Input::SoftDropOn => self.soft_drop_held = true,
            Input::SoftDropOff => self.soft_drop_held = false,
            Input::HardDrop => self.hard_drop(events),
            Input::MoveLeft => self.try_shift(-1, events),
            Input::MoveRight => self.try_shift(1, events),
            Input::RotateCw => self.try_rotate(RotationDir::Cw, events),
            Input::RotateCcw => self.try_rotate(RotationDir::Ccw, events),
            Input::Hold => self.try_hold(events),
            // Already handled at the phase level; ignore if seen in Playing.
            Input::StartGame | Input::ConfirmYes | Input::ConfirmNo => {}
        }
    }

    /// Attempt to hold the active piece (Guideline §1a).
    ///
    /// Returns immediately (no-op) if:
    /// - `hold_used_this_cycle` is set (one hold per piece cycle).
    /// - There is no active piece.
    ///
    /// Behaviour:
    /// - If the hold slot is empty: active goes to hold, next piece spawns
    ///   from the bag (next_queue).
    /// - If the hold slot is occupied: active ↔ hold swap. The returning
    ///   piece respawns at its normal spawn position (Zero rotation).
    ///
    /// After a successful hold, `hold_used_this_cycle` is set and
    /// `lock_state` is cleared (fresh start for the new active piece).
    fn try_hold(&mut self, events: &mut Vec<Event>) {
        if self.hold_used_this_cycle {
            return;
        }
        let active = match self.active.take() {
            Some(p) => p,
            None => return,
        };
        let incoming_kind = match self.hold.take() {
            Some(prev_kind) => prev_kind,
            None => {
                // Hold slot was empty — draw from the bag (next_queue).
                match self.next_queue.pop_front() {
                    Some(k) => {
                        self.next_queue.push_back(self.bag.next().unwrap());
                        k
                    }
                    None => {
                        // Bag exhausted (should not happen in normal play).
                        self.active = Some(active);
                        return;
                    }
                }
            }
        };

        // Stash the current active piece kind in the hold slot.
        self.hold = Some(active.kind);

        // Spawn the incoming piece at its standard spawn position.
        let new_piece = spawn(incoming_kind);

        // Clear lock counters — new piece gets a fresh start.
        self.lock_state = None;
        self.gravity_acc = Duration::ZERO;

        // Start spawn-fade animation for the incoming piece.
        let now = self.clock.now();
        self.spawn_anim = Some(SpawnAnim {
            started_at: now,
            kind: incoming_kind,
        });

        // Mark hold as used for this piece cycle.
        self.hold_used_this_cycle = true;

        // Check block-out: spawning into occupied cells ends the game.
        if new_piece
            .cells()
            .iter()
            .any(|&(c, r)| self.board.is_occupied(c, r))
        {
            self.phase = Phase::GameOver {
                reason: GameOverReason::BlockOut,
            };
            self.gameover_zoom = Some(GameOverZoom { started_at: now });
            self.score_display.current = self.score_display.target;
            events.push(Event::GameOver(GameOverReason::BlockOut));
            return;
        }

        self.active = Some(new_piece);
    }

    fn try_shift(&mut self, delta_col: i32, events: &mut Vec<Event>) {
        let piece = match &self.active {
            Some(p) => *p,
            None => return,
        };
        let candidate = crate::game::piece::Piece {
            origin: (piece.origin.0 + delta_col, piece.origin.1),
            ..piece
        };
        if candidate
            .cells()
            .iter()
            .all(|&(c, r)| !self.board.is_occupied(c, r))
        {
            self.active = Some(candidate);
            // If grounded, a successful move resets the lock timer.
            if self.is_grounded(&candidate) {
                if let Some(ls) = &mut self.lock_state {
                    ls.reset_timer();
                }
            }
        }
        let _ = events; // no event emitted for moves (per spec)
    }

    fn try_rotate(&mut self, dir: RotationDir, events: &mut Vec<Event>) {
        let piece = match &self.active {
            Some(p) => *p,
            None => return,
        };
        if let Ok(rotated) = rotate(&piece, dir, &self.board) {
            self.active = Some(rotated);
            // If the rotated piece is grounded, reset the lock timer.
            if self.is_grounded(&rotated) {
                if let Some(ls) = &mut self.lock_state {
                    ls.reset_timer();
                }
            }
        }
        let _ = events;
    }

    fn hard_drop(&mut self, events: &mut Vec<Event>) {
        let piece = match self.active {
            Some(p) => p,
            None => return,
        };
        // Drop as far as possible, accumulating cells for scoring.
        let mut current = piece;
        let mut cells_dropped: u32 = 0;
        loop {
            let below = crate::game::piece::Piece {
                origin: (current.origin.0, current.origin.1 + 1),
                ..current
            };
            if below
                .cells()
                .iter()
                .any(|&(c, r)| self.board.is_occupied(c, r))
            {
                break;
            }
            current = below;
            cells_dropped += 1;
        }
        // Hard-drop scoring: +2 per cell.
        self.score += cells_dropped * 2;
        self.active = Some(current);
        // Hard drop bypasses lock delay — lock immediately.
        self.lock_piece(events);
    }

    // ── Gravity ──────────────────────────────────────────────────────────────

    fn apply_gravity(&mut self, dt: Duration, events: &mut Vec<Event>) {
        let piece = match self.active {
            Some(p) => p,
            None => return,
        };

        let cell_dt = if self.soft_drop_held {
            soft_drop_effective_dt(self.level)
        } else {
            gravity_duration(self.level)
        };

        self.gravity_acc += dt;

        // Drop one cell at a time so we don't skip through locked pieces.
        while self.gravity_acc >= cell_dt {
            self.gravity_acc -= cell_dt;

            let current = self.active.unwrap();
            let below = crate::game::piece::Piece {
                origin: (current.origin.0, current.origin.1 + 1),
                ..current
            };

            if below
                .cells()
                .iter()
                .any(|&(c, r)| self.board.is_occupied(c, r))
            {
                // Cannot descend further — grounded.  Lock timer handled
                // in check_lock; clear accumulator so we don't retry this
                // cell until re-grounded.
                self.gravity_acc = Duration::ZERO;
                break;
            }

            // Piece descended one cell.
            if self.soft_drop_held {
                self.score += 1; // +1/cell for soft-drop
            }
            self.active = Some(below);
        }
        let _ = (piece, events);
    }

    // ── Lock delay ────────────────────────────────────────────────────────────

    fn check_lock(&mut self, dt: Duration, events: &mut Vec<Event>) {
        let piece = match self.active {
            Some(p) => p,
            None => return,
        };

        let grounded = self.is_grounded(&piece);

        if grounded {
            let ls = self.lock_state.get_or_insert_with(LockState::new);

            // Re-grounding after being airborne counts as a reset (SPEC §4).
            if ls.airborne {
                ls.airborne = false;
                ls.reset_timer(); // consumes a reset slot, resets elapsed
            }

            // If cap was already reached on a previous touch, lock now.
            if ls.is_capped() {
                self.lock_piece(events);
                return;
            }

            // Advance timer; lock if expired.
            if ls.advance(dt) {
                self.lock_piece(events);
            }
        } else {
            // Airborne: mark as lifted, pause the timer (preserve resets_used).
            if let Some(ls) = &mut self.lock_state {
                ls.airborne = true;
                ls.elapsed = Duration::ZERO;
            }
        }
    }

    // ── Lock + spawn ─────────────────────────────────────────────────────────

    fn is_grounded(&self, piece: &Piece) -> bool {
        let below = crate::game::piece::Piece {
            origin: (piece.origin.0, piece.origin.1 + 1),
            ..*piece
        };
        below
            .cells()
            .iter()
            .any(|&(c, r)| self.board.is_occupied(c, r))
    }

    fn lock_piece(&mut self, events: &mut Vec<Event>) {
        let piece = match self.active.take() {
            Some(p) => p,
            None => return,
        };

        // Check lock-out: piece entirely above visible playfield (row < 20).
        let all_above = piece.cells().iter().all(|&(_, r)| r < 20);
        if all_above {
            let now = self.clock.now();
            self.phase = Phase::GameOver {
                reason: GameOverReason::LockOut,
            };
            self.gameover_zoom = Some(GameOverZoom { started_at: now });
            // Snap the rolling display to its target so the HUD behind the
            // overlay shows the final score immediately (no frozen rollup).
            self.score_display.current = self.score_display.target;
            events.push(Event::GameOver(GameOverReason::LockOut));
            return;
        }

        // Place piece on the board.
        for (c, r) in piece.cells() {
            if c >= 0 && r >= 0 && c < 10 && r < 40 {
                self.board.set(c as usize, r as usize, piece.kind);
            }
        }

        events.push(Event::PieceLocked);

        // Detect full rows before clearing them.
        let full_rows: Vec<usize> = (0..40)
            .filter(|&r| (0..10usize).all(|c| self.board.cell_kind(c, r).is_some()))
            .collect();

        // Reset lock state and gravity accumulator.
        self.lock_state = None;
        self.gravity_acc = Duration::ZERO;
        // A piece has locked — allow hold again next cycle.
        self.hold_used_this_cycle = false;

        if full_rows.is_empty() {
            // No lines cleared — spawn immediately.
            self.spawn_next(events);
            return;
        }

        // Start the line-clear animation. Board is NOT mutated yet; the full
        // rows remain for the flash + dim frames. Scoring and spawn are
        // deferred to `finish_anim`.
        let now = self.clock.now();
        self.line_clear_anim = Some(LineClearAnim {
            rows: full_rows,
            started_at: now,
            phase: LineClearPhase::Flash,
            board_snapshot: self.board.clone(),
            pending_count: 0, // computed in finish_anim
            pending_level_before: self.level,
            pending_b2b_active: self.b2b_active,
        });
    }

    // ── Animation ────────────────────────────────────────────────────────────

    /// Advance the line-clear animation by checking elapsed time against the
    /// clock. Transitions Flash → Dim → finished. On finish, calls
    /// `finish_anim` to apply board mutation and spawn the next piece.
    fn tick_anim(&mut self, events: &mut Vec<Event>) {
        let now = self.clock.now();
        let elapsed = {
            let anim = self.line_clear_anim.as_mut().unwrap();
            now.saturating_duration_since(anim.started_at)
        };

        let flash_dur = Duration::from_millis(ANIM_FLASH_MS);
        let total_dur = Duration::from_millis(ANIM_TOTAL_MS);

        if elapsed >= total_dur {
            // Phase 3: animation complete — finalize.
            let anim = self.line_clear_anim.take().unwrap();
            self.finish_anim(anim, events);
        } else if elapsed >= flash_dur {
            // Phase 2: dim.
            if let Some(anim) = &mut self.line_clear_anim {
                anim.phase = LineClearPhase::Dim;
            }
        }
        // Phase 1 (Flash) is the initial state — nothing to change.
    }

    /// Finalize the animation: apply scoring, clear the board rows, and spawn
    /// the next piece.
    fn finish_anim(&mut self, anim: LineClearAnim, events: &mut Vec<Event>) {
        // Restore the board snapshot (which still has the full rows) then
        // clear them properly via the authoritative method.
        self.board = anim.board_snapshot;
        let cleared = self.board.clear_full_rows();

        let level_before = anim.pending_level_before;
        self.lines_cleared += u32::from(cleared);
        let new_level = level_after_lines(self.lines_cleared);
        self.level = new_level;

        let (delta, new_b2b) = score_line_clear(cleared, level_before, anim.pending_b2b_active);
        self.score += delta;
        let b2b_was = anim.pending_b2b_active;
        self.b2b_active = new_b2b;

        // Trigger the score rollup animation.
        if delta > 0 {
            let now = self.clock.now();
            self.score_display.add_target(delta, now);
        }

        if cleared > 0 {
            events.push(Event::LinesCleared {
                count: cleared,
                b2b: cleared == 4 && b2b_was,
                level_before,
                level_after: new_level,
                score_delta: delta,
            });
        }

        self.spawn_next(events);
    }

    fn spawn_next(&mut self, events: &mut Vec<Event>) {
        let kind = match self.next_queue.pop_front() {
            Some(k) => k,
            None => return,
        };
        self.next_queue.push_back(self.bag.next().unwrap());

        let piece = spawn(kind);

        // Check block-out: any spawn cell is occupied.
        if piece
            .cells()
            .iter()
            .any(|&(c, r)| self.board.is_occupied(c, r))
        {
            let now = self.clock.now();
            self.phase = Phase::GameOver {
                reason: GameOverReason::BlockOut,
            };
            self.gameover_zoom = Some(GameOverZoom { started_at: now });
            // Snap the rolling display to its target so the HUD behind the
            // overlay shows the final score immediately (no frozen rollup).
            self.score_display.current = self.score_display.target;
            events.push(Event::GameOver(GameOverReason::BlockOut));
            return;
        }

        // Start spawn-fade for the new active piece.
        let now = self.clock.now();
        self.spawn_anim = Some(SpawnAnim {
            started_at: now,
            kind,
        });
        self.active = Some(piece);
    }

    // ── Restart / reset ──────────────────────────────────────────────────────

    /// Return to the Title screen. No active piece; ready for a new round.
    fn reset_to_title(&mut self) {
        let now = self.clock.now();
        self.board = Board::empty();
        self.active = None;
        self.next_queue = VecDeque::new();
        self.score = 0;
        self.level = 1;
        self.lines_cleared = 0;
        self.b2b_active = false;
        self.phase = Phase::Title;
        self.hold = None;
        self.hold_used_this_cycle = false;
        let rng = StdRng::seed_from_u64(self.seed);
        self.bag = Bag::new(rng);
        self.lock_state = None;
        self.soft_drop_held = false;
        self.gravity_acc = Duration::ZERO;
        self.line_clear_anim = None;
        // No active piece yet, so no spawn-fade until StartGame.
        self.spawn_anim = None;
        self.score_display = ScoreDisplay::new(now);
        self.gameover_zoom = None;
    }
}
