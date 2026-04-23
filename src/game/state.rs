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
    Playing,
    Paused,
    GameOver { reason: GameOverReason },
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

    bag: Bag<StdRng>,
    pub lock_state: Option<LockState>,
    soft_drop_held: bool,
    gravity_acc: Duration,

    // Injected clock — stored so tests can share the FakeClock handle.
    #[allow(dead_code)]
    clock: Box<dyn Clock>,

    // Wall-clock reference point (last step).
    #[allow(dead_code)]
    last_tick: Instant,
}

impl GameState {
    /// Create a new game seeded with `seed`.
    ///
    /// The clock is used to initialise the starting `Instant`; all timing
    /// thereafter uses the `dt` passed to `step`.
    pub fn new(seed: u64, clock: Box<dyn Clock>) -> Self {
        let rng = StdRng::seed_from_u64(seed);
        let mut bag = Bag::new(rng);

        let mut next_queue: VecDeque<PieceKind> =
            (0..NEXT_QUEUE_LEN).map(|_| bag.next().unwrap()).collect();

        // Pop the first piece kind and spawn it.
        let first_kind = next_queue.pop_front().unwrap();
        next_queue.push_back(bag.next().unwrap());
        let active = spawn(first_kind);

        let now = clock.now();
        Self {
            board: Board::empty(),
            active: Some(active),
            next_queue,
            score: 0,
            level: 1,
            lines_cleared: 0,
            b2b_active: false,
            phase: Phase::Playing,
            seed,
            bag,
            lock_state: None,
            soft_drop_held: false,
            gravity_acc: Duration::ZERO,
            clock,
            last_tick: now,
        }
    }

    // ── step ────────────────────────────────────────────────────────────────

    /// Advance the game by `dt`, processing `inputs` in order.
    ///
    /// Returns events that occurred during this step.
    /// No-ops if in `Paused` or `GameOver` state (except Pause/Restart inputs).
    pub fn step(&mut self, dt: Duration, inputs: &[Input]) -> Vec<Event> {
        let mut events = Vec::new();

        match &self.phase {
            Phase::GameOver { .. } => {
                // Only Restart is meaningful in GameOver.
                for &inp in inputs {
                    if inp == Input::Restart {
                        self.restart();
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
            Input::Restart => self.restart(),
            Input::SoftDropOn => self.soft_drop_held = true,
            Input::SoftDropOff => self.soft_drop_held = false,
            Input::HardDrop => self.hard_drop(events),
            Input::MoveLeft => self.try_shift(-1, events),
            Input::MoveRight => self.try_shift(1, events),
            Input::RotateCw => self.try_rotate(RotationDir::Cw, events),
            Input::RotateCcw => self.try_rotate(RotationDir::Ccw, events),
        }
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
            self.phase = Phase::GameOver {
                reason: GameOverReason::LockOut,
            };
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

        // Clear full rows and score.
        let level_before = self.level;
        let cleared = self.board.clear_full_rows();
        self.lines_cleared += u32::from(cleared);
        let new_level = level_after_lines(self.lines_cleared);
        self.level = new_level;

        let (delta, new_b2b) = score_line_clear(cleared, level_before, self.b2b_active);
        self.score += delta;
        let b2b_was = self.b2b_active;
        self.b2b_active = new_b2b;

        if cleared > 0 {
            events.push(Event::LinesCleared {
                count: cleared,
                b2b: cleared == 4 && b2b_was,
                level_before,
                level_after: new_level,
                score_delta: delta,
            });
        }

        // Reset lock state and gravity accumulator.
        self.lock_state = None;
        self.gravity_acc = Duration::ZERO;

        // Spawn next piece.
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
            self.phase = Phase::GameOver {
                reason: GameOverReason::BlockOut,
            };
            events.push(Event::GameOver(GameOverReason::BlockOut));
            return;
        }

        self.active = Some(piece);
    }

    // ── Restart ──────────────────────────────────────────────────────────────

    fn restart(&mut self) {
        // Reuse the original seed so bag order is reproducible (#24).
        let rng = StdRng::seed_from_u64(self.seed);
        let mut bag = Bag::new(rng);
        let mut next_queue: VecDeque<PieceKind> =
            (0..NEXT_QUEUE_LEN).map(|_| bag.next().unwrap()).collect();
        let first_kind = next_queue.pop_front().unwrap();
        next_queue.push_back(bag.next().unwrap());
        let active = spawn(first_kind);

        self.board = Board::empty();
        self.active = Some(active);
        self.next_queue = next_queue;
        self.score = 0;
        self.level = 1;
        self.lines_cleared = 0;
        self.b2b_active = false;
        self.phase = Phase::Playing;
        self.bag = bag;
        self.lock_state = None;
        self.soft_drop_held = false;
        self.gravity_acc = Duration::ZERO;
    }
}
