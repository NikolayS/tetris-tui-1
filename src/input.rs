//! DAS/ARR input model.
//!
//! Translates raw crossterm events into `Vec<Input>` while emulating
//! held-key repeats according to the SPEC §4 timing parameters.
//!
//! ## Kitty keyboard protocol probe
//!
//! On startup `InputTranslator::probe_kitty()` writes `CSI > 1u` to
//! stdout and waits up to 50 ms for a `CSI ? <flags> u` response from
//! the terminal.  If found, true press/release events are used.
//! Otherwise the 160 ms release-inference heuristic applies.
//!
//! ## DAS / ARR timing (SPEC §4 round-2 authoritative)
//!
//! - **DAS:** 160 ms initial delay before auto-repeat starts.
//! - **ARR:** 30 ms between subsequent moves during auto-repeat.
//! - **Release inference:** 160 ms — if `tick()` is called and no new
//!   event for the held direction has arrived within 160 ms, treat the
//!   key as released.
//!
//! ## Soft-drop
//!
//! Soft-drop uses edge events (`SoftDropOn` / `SoftDropOff`), not the
//! DAS/ARR repeat loop.  The `InputTranslator` emits `SoftDropOn` on
//! the first press and `SoftDropOff` on release (via kitty events) or
//! when no soft-drop event arrives for 160 ms.

use std::io::{self, Write};
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};

use crate::game::state::Input;

// ── timing constants ──────────────────────────────────────────────────────

/// Initial delay before auto-repeat starts (DAS).
pub const DAS_MS: u64 = 160;
/// Delay between each repeated move after DAS expires (ARR).
pub const ARR_MS: u64 = 30;
/// Release-inference timeout: no event for this long → treat as released.
pub const RELEASE_INFER_MS: u64 = 160;

// ── types ─────────────────────────────────────────────────────────────────

/// Which horizontal direction is currently held.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeldDir {
    Left,
    Right,
}

/// State for one held directional key.
#[derive(Debug, Clone)]
pub struct HeldDirection {
    pub kind: HeldDir,
    /// When the key was first pressed.
    pub pressed_at: Instant,
    /// When the last repeat move was emitted.
    pub last_repeat: Option<Instant>,
}

/// Whether the terminal supports the kitty keyboard protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KittySupport {
    /// True press/release events available.
    Supported,
    /// Fall back to release-inference heuristic.
    Heuristic,
}

// ── translator ────────────────────────────────────────────────────────────

/// Stateful input translator.
///
/// Call `translate_event()` for each crossterm event, then call `tick()`
/// once per game loop iteration to emit any DAS/ARR or release-inferred
/// inputs that are due.
pub struct InputTranslator {
    pub kitty: KittySupport,
    pub held: Option<HeldDirection>,
    /// Last time we received any event for the held direction.
    last_dir_event: Option<Instant>,
    /// Whether soft-drop is currently held.
    soft_drop_held: bool,
    /// Last time we received a soft-drop event (for release inference).
    last_soft_drop: Option<Instant>,
    /// DAS delay.
    das: Duration,
    /// ARR interval.
    arr: Duration,
    /// Release-inference timeout.
    release_infer: Duration,
}

impl InputTranslator {
    /// Creates a new translator with default SPEC §4 timing.
    pub fn new(kitty: KittySupport) -> Self {
        Self::with_timing(
            kitty,
            Duration::from_millis(DAS_MS),
            Duration::from_millis(ARR_MS),
            Duration::from_millis(RELEASE_INFER_MS),
        )
    }

    /// Creates a translator with custom timing (used in tests).
    pub fn with_timing(
        kitty: KittySupport,
        das: Duration,
        arr: Duration,
        release_infer: Duration,
    ) -> Self {
        Self {
            kitty,
            held: None,
            last_dir_event: None,
            soft_drop_held: false,
            last_soft_drop: None,
            das,
            arr,
            release_infer,
        }
    }

    /// Probes for kitty keyboard protocol support by writing `CSI > 1u`
    /// to stdout and waiting up to `timeout` for a response.
    ///
    /// # Protocol notes
    ///
    /// `CSI > 1 u` enables the kitty keyboard protocol (progressive
    /// enhancement level 1 — disambiguate escape codes).  A supporting
    /// terminal echoes `CSI ? <flags> u` back in the input stream.
    ///
    /// **Compatibility:**
    /// - **Kitty:** full support since 0.20.
    /// - **Ghostty:** full support.
    /// - **WezTerm:** full support.
    /// - **Alacritty:** partial — may not respond; falls back to heuristic.
    /// - **iTerm2:** no response; falls back to heuristic.
    /// - **xterm / linux console:** no response; falls back to heuristic.
    ///
    /// The probe is safe: on non-supporting terminals the query sequence
    /// is silently ignored and the 50 ms timeout elapses harmlessly.
    /// We do NOT rely on kitty support being available.
    pub fn probe_kitty(timeout: Duration) -> KittySupport {
        // Write the query: CSI > 1 u  (kitty progressive enhancement query).
        let mut stdout = io::stdout();
        if write!(stdout, "\x1b[>1u").is_err() || stdout.flush().is_err() {
            return KittySupport::Heuristic;
        }

        // Poll for a response within `timeout`.
        let deadline = Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break;
            }
            match event::poll(remaining.min(Duration::from_millis(10))) {
                Ok(true) => {
                    if let Ok(Event::Key(key)) = event::read() {
                        // The terminal responded; we have kitty support.
                        // The actual response is `CSI ? <n> u` which crossterm
                        // may surface as various key events depending on version.
                        // Any response within the window is sufficient evidence.
                        let _ = key;
                        return KittySupport::Supported;
                    }
                }
                _ => break,
            }
            if Instant::now() >= deadline {
                break;
            }
        }

        KittySupport::Heuristic
    }

    /// Translates a raw crossterm event into zero or more `Input` values.
    ///
    /// Returns `(inputs, quit_requested)`.
    pub fn translate_event(&mut self, ev: &Event, now: Instant) -> (Vec<Input>, bool) {
        let mut inputs = Vec::new();
        let mut quit = false;

        if let Event::Key(key) = ev {
            match key.kind {
                KeyEventKind::Release => {
                    // With kitty support we get real release events.
                    self.handle_release(key.code, now, &mut inputs);
                }
                KeyEventKind::Press | KeyEventKind::Repeat => {
                    if self.is_quit(*key) {
                        quit = true;
                        return (inputs, quit);
                    }
                    self.handle_press(key.code, now, &mut inputs);
                }
            }
        }

        (inputs, quit)
    }

    fn handle_press(&mut self, code: KeyCode, now: Instant, inputs: &mut Vec<Input>) {
        match code {
            KeyCode::Left | KeyCode::Char('a') | KeyCode::Char('h') => {
                self.press_dir(HeldDir::Left, now, inputs);
            }
            KeyCode::Right | KeyCode::Char('d') | KeyCode::Char('l') => {
                self.press_dir(HeldDir::Right, now, inputs);
            }
            KeyCode::Down | KeyCode::Char('s') | KeyCode::Char('j') => {
                if !self.soft_drop_held {
                    self.soft_drop_held = true;
                    inputs.push(Input::SoftDropOn);
                }
                self.last_soft_drop = Some(now);
            }
            KeyCode::Char(' ') => inputs.push(Input::HardDrop),
            KeyCode::Char('z') => inputs.push(Input::RotateCcw),
            KeyCode::Char('x') => inputs.push(Input::RotateCw),
            KeyCode::Char('c') => inputs.push(Input::Hold),
            KeyCode::Char('p') => inputs.push(Input::Pause),
            KeyCode::Char('r') => inputs.push(Input::Restart),
            _ => {}
        }
    }

    fn handle_release(&mut self, code: KeyCode, _now: Instant, inputs: &mut Vec<Input>) {
        match code {
            KeyCode::Left | KeyCode::Char('a') | KeyCode::Char('h')
                if matches!(self.held.as_ref().map(|h| h.kind), Some(HeldDir::Left)) =>
            {
                self.held = None;
                self.last_dir_event = None;
            }
            KeyCode::Right | KeyCode::Char('d') | KeyCode::Char('l')
                if matches!(self.held.as_ref().map(|h| h.kind), Some(HeldDir::Right)) =>
            {
                self.held = None;
                self.last_dir_event = None;
            }
            KeyCode::Down | KeyCode::Char('s') | KeyCode::Char('j') if self.soft_drop_held => {
                self.soft_drop_held = false;
                self.last_soft_drop = None;
                inputs.push(Input::SoftDropOff);
            }
            _ => {}
        }
    }

    fn press_dir(&mut self, dir: HeldDir, now: Instant, inputs: &mut Vec<Input>) {
        if matches!(&self.held, Some(h) if h.kind == dir) {
            // Same direction — update timestamp (release-inference reset).
            self.last_dir_event = Some(now);
            return;
        }
        // New direction or direction change — emit one immediate move.
        let input = match dir {
            HeldDir::Left => Input::MoveLeft,
            HeldDir::Right => Input::MoveRight,
        };
        inputs.push(input);
        self.held = Some(HeldDirection {
            kind: dir,
            pressed_at: now,
            last_repeat: None,
        });
        self.last_dir_event = Some(now);
    }

    /// Called each tick to emit any DAS/ARR moves or release-inferred events.
    pub fn tick(&mut self, now: Instant, inputs: &mut Vec<Input>) {
        // Release-inference for soft-drop.
        // Use strictly-greater so that the threshold boundary itself
        // does not accidentally fire before DAS can emit a repeat.
        if let Some(last) = self.last_soft_drop {
            if self.soft_drop_held && now.duration_since(last) > self.release_infer {
                self.soft_drop_held = false;
                self.last_soft_drop = None;
                inputs.push(Input::SoftDropOff);
            }
        }

        // Release-inference for direction (heuristic mode only).
        if self.kitty == KittySupport::Heuristic {
            if let Some(last) = self.last_dir_event {
                if now.duration_since(last) > self.release_infer {
                    self.held = None;
                    self.last_dir_event = None;
                    return;
                }
            }
        }

        // DAS/ARR repeat.
        let held = match &mut self.held {
            Some(h) => h,
            None => return,
        };

        let input = match held.kind {
            HeldDir::Left => Input::MoveLeft,
            HeldDir::Right => Input::MoveRight,
        };

        let since_press = now.duration_since(held.pressed_at);
        if since_press < self.das {
            // Still in DAS window — no repeat yet.
            return;
        }

        // DAS has expired — check ARR.
        // On first repeat (no last_repeat), use `pressed_at + das - arr` as
        // the sentinel so the first repeat fires immediately at the DAS
        // boundary (since_repeat = arr ≥ arr → fires).
        let last_repeat = held
            .last_repeat
            .unwrap_or_else(|| held.pressed_at + self.das - self.arr);
        let since_repeat = now.duration_since(last_repeat);
        if since_repeat >= self.arr {
            inputs.push(input);
            held.last_repeat = Some(now);
            // Update last_dir_event so release-inference does not fire
            // while active ARR repeating is occurring.
            self.last_dir_event = Some(now);
        }
    }

    fn is_quit(&self, key: crossterm::event::KeyEvent) -> bool {
        matches!(key.code, KeyCode::Char('q'))
            || (key.modifiers.contains(KeyModifiers::CONTROL)
                && matches!(key.code, KeyCode::Char('c') | KeyCode::Char('d')))
    }
}
