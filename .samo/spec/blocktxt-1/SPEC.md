# blocktxt-1 — SPEC v0.3

## 1. Goal & why it's needed

**Goal.** Ship a polished, single-player terminal falling-block puzzle game with Guideline-inspired mechanics (SRS rotation + kicks, 7-bag, ghost, next-preview, Guideline-style scoring for singles/doubles/triples/4-line clears), playable on macOS and Linux terminals (and Windows via WSL), with persistent high scores.

**Why this exists.** Existing terminal falling-block clones fall into two buckets: (a) toys with non-standard rotation and wrong scoring that feel "off" to anyone who has played the mainstream game, and (b) heavy GUI ports that require a windowing stack. There is a genuine niche for a *correct-enough*, lightweight, keyboard-only terminal game — fast to launch, zero deps beyond a single binary, and faithful enough to Guideline mechanics that muscle memory from mainstream play transfers cleanly for the core move set (movement, rotation with kicks, drops, line clears). This project exists to fill that niche.

**Guideline-compliance honesty.** v0.1 is explicitly **Guideline-inspired, not fully Guideline-compliant**. v0.1 ships: SRS rotation with JLSTZ + I kick tables, 7-bag, ghost, next-preview, Guideline-style scoring for non-T-spin clears including Back-to-Back 4-line clears, lock delay w/ reset cap. v0.1 does **not** ship: hold piece, T-spin detection/scoring, combo counter, color themes, configurable timings via a file — these are v0.2. README and title screen surface this honestly.

**Non-goals (explicit).** No multiplayer, no netcode, no sound, no GUI. These are out of scope per the brief and must not re-enter the design in disguise (no "stub network module", no audio crate pulled in "for later", no placeholder hold slot in v0.1 state).

## 1a. Branding

The project name `blocktxt` is non-derivative. Piece names (I, O, T, S, Z, J, L) refer to their shapes. The SRS rotation system, 7-piece-bag randomizer, and Guideline-style scoring are implemented from their public mathematical specifications — no third-party tables, artwork, or documentation are vendored or referenced by name. Piece color palette is chosen for distinct terminal rendering, not to match any specific canonical set; palette constants live in `render/theme.rs` with a comment noting the independent selection.

**Release checklist item.** Tagging v0.1.0 requires running `scripts/check-naming.sh`, which greps source + release artifacts for the prohibited trademark term and fails the release if any occurrence is found. CI runs the same check on `main`.

## 2. User stories

1. **Keyboard-native dev on macOS.** As a developer who lives in iTerm2/Ghostty/Alacritty, I want to run the binary in a terminal tab and immediately play a full game with arrow-key / WASD / vim-style bindings, so I can take a five-minute break without leaving my terminal workflow. Outcome: the binary launches into a playable board in under 200 ms, and I can clear lines, pause with `p`, and quit with `q` without reading docs.
2. **Returning player chasing a high score.** As a returning player, I want my best score persisted across sessions so I can try to beat it tomorrow. Outcome: after a game-over, my score is written to a local high-score file; next launch shows the top-5 on the title screen, and a new personal best is highlighted on the game-over screen. If persistence is unavailable (unwritable dir, missing HOME), the game still plays end-to-end and a one-line warning is printed **to stderr before entering raw mode**.
3. **Purist who expects real feel (core move set).** As a player who has played mainstream falling-block games, I want SRS rotation with standard wall kicks, a 7-bag randomizer, a ghost piece, and a next-piece preview, so the core feel matches what I know. Outcome: SRS kicks match the SRS specification values (tested exhaustively); I never get the same piece four times in a row impossibly; I can see where my piece will land and what's coming next. (Hold + T-spin are called out as v0.2 in the README so this story's expectations are scoped honestly.)
4. **Linux power user on a slow SSH session.** As a user playing over SSH from a laptop on hotel Wi-Fi, I want the renderer to only redraw changed cells so input feels responsive even at 30–50 ms RTT. Outcome: measurable acceptance — see §4 "SSH responsiveness" — including bytes/frame ceiling as a non-blocking perf *target* (not a hard CI gate), an input-to-visible-change latency target, and graceful behavior on resize and slow stdout.
5. **Accessibility-minded player.** As a player on a monochrome or 16-color terminal, I want the game to remain fully playable without 256-color support, using distinct ASCII glyphs per piece. Outcome: the game detects terminal color capability and falls back to glyph-based differentiation when color is unavailable or disabled via `NO_COLOR`.

## 3. Architecture

<!-- architecture:begin -->

```text
(architecture not yet specified)
```

<!-- architecture:end -->

**Contracts at the seams.**

- **`game/` ↔ everything else** is the hard seam. `GameState` exposes one mutator: `fn step(&mut self, dt: Duration, inputs: &[Input]) -> Vec<Event>`. It never touches stdin/stdout/files/env/clock/rand directly; `Clock` and `Rng` are injected at construction. Renderer holds `&GameState` and reads; it never mutates. This seam is what makes parallel ownership in §7 possible and what every headless test in §5 relies on.
- **`app.rs` ↔ `signals.rs`** is the other hard seam. Signal handlers only do async-signal-safe work: `AtomicBool::store(true, Relaxed)` on the relevant flag. All terminal I/O (restore on SIGTSTP, re-enter on SIGCONT, full redraw on SIGWINCH, exit on SIGINT/SIGTERM) is done by the main loop observing the flag on its next tick. This is spelled out in "Signal handling" below and is what justifies v0.3's rewrite from v0.2's "handler does restore" design.
- **`terminal.rs` ↔ main loop**: the guard is owned by the main-loop scope; its `Drop` is the single restore path. The panic hook calls the same restore routine via a `static` indirection but does not touch the guard's state; restore is idempotent.

**Language & stack.** Rust (stable, MSRV **1.75**, verified in CI via `cargo +1.75 check`). Chosen because: (a) single static binary, trivial cross-platform distribution; (b) `ratatui` + `crossterm` is the most mature cross-platform TUI stack today; (c) strong type system makes the state machine (SRS kicks, lock delay, line-clear phases) much easier to get right than in a dynamic language.

**Crate dependencies (v0.1, locked via `Cargo.lock`).** Cargo.toml uses standard caret requirements; reproducibility comes from the committed `Cargo.lock`. CI enforces `Cargo.lock` is committed and `cargo +1.75 check` passes without modifying it. Advisory response: `cargo deny check advisories` runs in CI; a daily scheduled job opens an issue on new advisories; security-severity advisories are addressed within 7 days by a minor-version bump. No pinned versions are claimed.

Runtime:

- `ratatui` (0.28.x) — widget/framebuffer layer.
- `crossterm` (0.28.x) — cross-platform terminal I/O backend.
- `rand` (0.8.x), `rand_chacha` (0.3.x) — seedable RNG for the 7-bag.
- `serde` (1.x, `derive`), `serde_json` (1.x) — high-score persistence.
- `tempfile` (3.x) — named temp files for atomic rename.
- `directories` (5.x) — platform config/data dirs.
- `anyhow` (1.x), `thiserror` (1.x) — error handling at boundaries.
- `clap` (4.x, `derive`) — CLI flags.
- `signal-hook` (0.3.x) — unix signal flags (gated `#[cfg(unix)]`).

Dev-only:

- `proptest` (1.x) — property tests for bag + scoring.
- `insta` (1.x) — snapshot tests for renderer.
- `assert_cmd` (2.x), `predicates` (3.x) — CLI integration tests.
- `rexpect` (0.5.x, unix only, `#[cfg(unix)]`) — PTY-based terminal lifecycle tests.

No `toml` crate in v0.1: configuration file support is deferred to v0.2 (see §4 "Configuration"). No async runtime. No audio. No networking.

**Module layout (`src/`):**

```
main.rs            // CLI parsing, pre-TTY actions, panic hook, signal install, terminal setup/teardown
app.rs             // AppState enum; main loop; observes signal flags; owns all terminal I/O
terminal.rs        // TerminalGuard (RAII): ordered setup + rollback; idempotent restore
game/
  mod.rs
  board.rs         // 10x40 playfield; coord system + top-out cases
  piece.rs         // Piece enum + shape tables + rotation states
  srs.rs           // SRS rotation + wall-kick tables (derivation-commented)
  bag.rs           // 7-bag with seedable RNG
  rules.rs         // gravity curve, lock delay, scoring, level progression
  score.rs         // Score + line-clear classification + B2B state
input.rs           // crossterm::Event → Input; DAS/ARR state machine driven by dt
render/
  mod.rs
  board_view.rs
  hud.rs
  theme.rs         // 256/16-color + NO_COLOR fallback
  helpers.rs       // pure functions (ghost Y, HUD format) — unit-testable without Buffer
persistence.rs     // high-score file load/save (atomic); directory setup + validation
clock.rs           // Clock trait (real + fake)
signals.rs         // flag struct; handlers do atomic stores only; #[cfg(unix)] for unix signals
```

**Key abstractions.**

- `GameState` — pure, deterministic. All mutation via `GameState::step(dt, &[Input]) -> Vec<Event>`. No I/O.
- `Clock` trait — real wraps `Instant`; fake is a manually-advanced counter.
- `Rng` (generic parameter on `GameState`) — real is `ChaCha8Rng`; tests inject seeded instances.
- `Renderer` — pure read on `&GameState`, writes into a `ratatui::Buffer`.
- `Input` enum — `MoveLeft`, `MoveRight`, `SoftDropOn`, `SoftDropOff`, `HardDrop`, `RotateCW`, `RotateCCW`, `Pause`, `Quit`, `Restart`. (`Hold` deferred to v0.2.) Note `SoftDropOn`/`SoftDropOff` instead of a single event — §4 "Input handling" explains why.
- `TerminalGuard` — RAII wrapper: enter raw → alt → hide-cursor on construction with rollback on partial failure; restore unconditionally on drop *and* via panic hook *and* via the main-loop's signal-observation path. All paths call the same idempotent restore routine.
- `SignalFlags` — struct of `AtomicBool`s (`shutdown`, `tstp_pending`, `cont_pending`, `winch_pending`); the only thing signal handlers touch.

**Data flow (per tick).**

```
  ┌─────────────────────────────────────────────────────────────────────┐
  │ 1. Check SignalFlags (ordered): shutdown → exit clean;              │
  │    tstp_pending → restore terminal, raise SIGTSTP, on wake re-enter │
  │    raw/alt/hide, mark winch_pending (forces redraw);                │
  │    winch_pending → query size, transition to too-small overlay or   │
  │    force full redraw.                                               │
  │ 2. poll(8ms) events → translate to Inputs (+ DAS/ARR ticks from dt).│
  │ 3. Clock::now(), compute dt since last tick.                        │
  │ 4. GameState::step(dt, &inputs) → Vec<Event>.                       │
  │ 5. If any visible state changed OR winch_pending was set: draw.     │
  │ 6. BufWriter flush; on BrokenPipe → exit clean.                     │
  └─────────────────────────────────────────────────────────────────────┘
```

One loop, no threads. Redraw cadence is event-driven: we draw when state changed since last draw OR when the clock ticked ≥ 16 ms and any animation is active. Worst-case input-to-visible-change latency = `poll_timeout (8ms) + step + diff + write`. See §4 "Frame cadence" for the detailed bound.

**Terminal rendering details.**

- Each board cell is drawn **two characters wide** (`"[]"` or `"██"`) so blocks look roughly square. Playfield: `20 rows × 10 cols × 2 chars = 20×20` character grid.
- Colors: 8 piece colors when 256-color is available; fall back to 16-color; fall back to per-piece ASCII glyphs (`I`,`O`,`T`,`S`,`Z`,`J`,`L`) when `NO_COLOR` is set or color is unsupported.
- Only dirty regions are redrawn (ratatui diffs against its internal buffer).

**Terminal lifecycle, signals, and teardown guarantees.** A TUI that leaves the terminal in raw/alternate/cursor-hidden state on exit is a core ops failure. v0.1 guarantees restore on:

1. **Normal exit** — `TerminalGuard::drop` runs.
2. **Panic** — panic hook installed *before* `TerminalGuard::enter()`; hook calls the restore routine (via a `static OnceLock<fn()>` so it does not need the guard), then the default hook. The guard's `drop` is idempotent so a second restore is a no-op. The restore routine writes a fixed byte sequence (known ANSI resets) to `stderr` raw FD, avoiding the buffered writer.
3. **SIGINT / SIGTERM** — signal handler sets `flags.shutdown`. Main loop observes next tick and exits cleanly, invoking the guard's drop.
4. **SIGTSTP (Ctrl-Z) / SIGCONT** — signal handlers set `flags.tstp_pending` / `flags.cont_pending` only. Main loop on next tick: on TSTP, restore terminal, then `kill(getpid(), SIGTSTP)` to actually suspend the process. On resumption, when the loop wakes (the `kill` returns after SIGCONT), it re-enters raw/alt/hide and forces a full redraw. No terminal I/O happens inside a signal handler. Windows (WSL) takes the unix path since we require WSL for Windows.
5. **SIGWINCH / resize** — signal handler sets `flags.winch_pending` (belt-and-suspenders alongside `crossterm::Event::Resize`). Main loop queries size; if below minimum (`44×24` chars), transitions to "terminal too small" overlay state; otherwise forces full redraw.
6. **Setup failure partway through** — `TerminalGuard::enter()` is ordered (raw → alt → hide) and on any intermediate error, reverses the steps already taken before returning the error. Tested.
7. **Slow stdout / backpressure** — writes are buffered via `BufWriter<Stdout>`; on `ErrorKind::BrokenPipe` or other write failure, the loop exits cleanly via the same guard path.

**Pre-TTY actions.** CLI actions that need user interaction or can fail loudly run **before** `TerminalGuard::enter()`: `--reset-scores` (with stdin y/N prompt, line-buffered), `--version`, `--help`, and path validation for any explicitly-provided path arguments. This avoids the well-known TUI failure mode of prompting while the terminal is in raw mode.

## 4. Implementation details

**Coordinate system & top-out (fully specified).**

- Board is a `10 cols × 40 rows` grid. Origin `(0,0)` is **top-left**. `x` increases rightward (0..10). `y` increases downward (0..40).
- Rows `0..20` are the **hidden buffer** (used for spawn + above-visible locks). Rows `20..40` are the **visible playfield**. (This is the inverse of the Guideline's bottom-origin indexing but consistent within our codebase — we pick top-origin because ratatui addresses rows top-down and this eliminates one coordinate flip.)
- **Spawn position.** Each piece spawns with its bounding-box top at row `18`. Horizontal placement: JLSTZ and I spawn with their 4-wide bounding box at cols `3..7` (inclusive of 3, exclusive of 7 — i.e., cols 3,4,5,6). O spawns with its 2-wide filled cells at cols `4..6` (cells at cols 4 and 5). (v0.2 draft's "O centered on cols 3..6" was ambiguous and is replaced here.)
- If *any* cell of the spawned piece overlaps an occupied cell → **block-out**, game over immediately.
- **Lock-out.** After a piece locks, if *every* cell of the just-locked piece has `y < 20` (entirely in the hidden buffer, above the visible area) → **lock-out**, game over.
- **Partial lock.** If some cells are in the hidden buffer and some visible, the game continues.
- Test cases for each of {block-out, lock-out, partial lock, normal lock, O-spawn position, I-spawn position, JLSTZ-spawn position} are committed as part of §5.

**Rotation system — SRS.**

- 4 rotation states per piece (0, R, 2, L).
- JLSTZ share one wall-kick table (5 offsets per transition, 8 transitions).
- I piece has its own kick table.
- O piece does not kick.
- Rotation attempt: test base rotation; if blocked, test kicks in order; first non-colliding offset wins; if all fail, rotation is rejected.
- Kick tables are `const` arrays in `srs.rs` annotated with derivation comments (not copies of external tables) and unit-tested against the SRS reference values in a golden test.

**7-bag randomizer.** Sequence of 7 pieces, shuffled (Fisher–Yates using the injected `Rng`), drained one at a time; when empty, refill + reshuffle. Seedable via `--seed`.

**Gravity & level curve.** `gravity_seconds_per_cell = (0.8 − (level−1) × 0.007) ^ (level−1)`, clamped at level 20. Hard drop = instant (+2 pt/cell).

**Soft drop (authoritative model, resolves v0.2 conflict).** Soft drop is governed by **a single rule**: while soft-drop is held, the effective gravity is `max(natural_gravity / 20, 30 ms/cell)`. That is, 20× faster than natural gravity, but **capped at ~33 cells/s** so at high levels where natural gravity is already very fast, soft drop never becomes instantaneous (which would be indistinguishable from hard drop). Points: +1/cell actually dropped under soft-drop control, accrued as each cell of descent occurs under the capped rate.

Implementation: the input model uses `SoftDropOn` / `SoftDropOff` edge events (not a 30 ms repeat). While the flag is on, `rules.rs` uses the capped rate for gravity; no key-repeat simulation is needed for soft drop.

**Lock delay (fully specified).** Lock delay follows the modern Guideline-style "extended placement with reset cap" model:

- When the piece first becomes **grounded** (cannot move down further), a 500 ms lock timer starts.
- Each **successful** move or rotation that keeps the piece grounded (or re-grounds it after a brief airborne interval) **resets the 500 ms timer**, up to a cap of **15 resets total per piece**.
- If the piece becomes airborne (e.g., kicked upward by a rotation), the lock timer **pauses** but the reset counter **does not decrement**. When the piece re-grounds, the timer resumes from zero (counts as a reset if under the cap) and the reset counter continues from where it left off.
- The piece locks when **either** the 500 ms timer expires while grounded **or** the 15-reset cap is reached and the piece is grounded (forced lock on the next ground-touch after cap is hit).
- If the timer expires while airborne, nothing happens; the piece continues falling under gravity.
- Hard drop bypasses lock delay entirely (piece locks instantly at the hard-drop destination).

Tests cover: single-reset, multi-reset under cap, cap reached mid-air (locks on re-ground), timer expiration mid-air (no lock), hard-drop short-circuit.

**Line clears.** After lock: detect full rows, play a brief clear animation (2 frames of inverted cells, ~100 ms), then collapse. Scoring (Guideline-style, no T-spin in v0.1):

| Clear | Points |
| --- | --- |
| Single | 100 × level |
| Double | 300 × level |
| Triple | 500 × level |
| 4-line clear | 800 × level |

**Back-to-Back multiplier (authoritative).** A "difficult" clear in v0.1 is only a 4-line clear (T-spins are v0.2). B2B state:

- `b2b_active = false` at game start.
- On a 4-line clear: if `b2b_active` was already true, score this 4-line clear as `800 × level × 1.5 = 1200 × level`; then set `b2b_active = true` regardless (it was or becomes true). On the **first** 4-line clear (`b2b_active` was false), score `800 × level` (no multiplier) and set `b2b_active = true`.
- On a Single/Double/Triple (non-difficult line clear): score normally and set `b2b_active = false`.
- On a lock that clears 0 lines: `b2b_active` is **unchanged** (only line clears reset the chain; empty locks neither break nor extend).

Soft-drop: +1/cell (under capped rate, see above). Hard-drop: +2/cell.

**Level progression.** Every 10 lines → level +1. Max level 20 for gravity; score keeps multiplying past that.

**State transitions.**

```
              start
                │
                ▼
          ┌──────────┐   Enter    ┌──────────┐   p      ┌──────────┐
          │  Title   ├──────────►│ Playing  ├────────►│  Paused  │
          └──────────┘            └────┬─────┘◄────────┴──────────┘
                ▲                      │ top-out   p/Enter
                │ r                    ▼
                │                ┌──────────┐
                └────────────────┤ GameOver │
                                 └──────────┘
```

**Input handling (DAS/ARR, terminal-reality-aware).**

Terminals do not reliably send key-release events (xterm/linux console do not; kitty's keyboard protocol does but we cannot require it). Held-key detection must therefore be synthesized from key-press timing, not from release events.

Model:

- `input.rs` tracks a `held_direction: Option<(Direction, Instant)>`.
- On `KeyPress(Left or Right)`: if held_direction is None or differs, set it to `(dir, now)` and emit one `MoveLeft`/`MoveRight`. If it matches the current held direction, emit one `MoveX` (this is the repeat event from the terminal's own key-repeat) and update the timestamp.
- On `KeyPress(Other)` or a tick where the last press was > **160 ms** ago: clear `held_direction` (terminal key-repeat interval is typically 30–50 ms; 160 ms is a conservative "the user definitely released" threshold).
- DAS (default 170 ms) and ARR (default 50 ms) are driven **from `held_direction` age**, not from raw key-repeat events: while held_direction has been set for ≥ DAS, `input.rs` emits `MoveX` events at ARR cadence from the clock, **in addition to** any natural terminal repeat events (which are debounced — two shifts within < ARR are collapsed to one).
- Soft-drop uses `SoftDropOn`/`SoftDropOff` derived identically: `SoftDropOn` on first press, `SoftDropOff` when no soft-drop press has been seen for > 160 ms.
- **Kitty keyboard protocol**: if detected at startup (probe sequence), the code prefers actual press/release events over the timing heuristic. Detection failure falls back silently to the heuristic.

Tunables (`DAS`, `ARR`, `SOFT_DROP_RELEASE_TIMEOUT`) are compiled-in constants in v0.1 (no config file). v0.2 may expose them.

**Persistence (atomic, hardened, degrades gracefully).**

- File: `{data_dir}/high_scores.json`, where `data_dir = ProjectDirs::from("", "", "blocktxt").data_dir()`.
- Format: JSON array of `{score, lines, level, date}`, top 10 kept.

**Directory setup & validation (v0.3 hardening).**

1. **Resolve.** If `ProjectDirs::from(...)` returns `None` (no HOME, unsupported platform): log one line to stderr *before* entering raw mode, disable persistence for the session, continue in-memory. Do not panic.
2. **Create.** On first use, create `data_dir` with `fs::create_dir_all`; on unix, immediately `chmod 0o700` the leaf directory (via `set_permissions`). Parent dirs are not chmod'd (they're system-owned).
3. **Validate (unix, on every startup).**
   - `symlink_metadata(data_dir)` — if the path is a symlink, refuse to use it (log warning, disable persistence for the session). This prevents path-swap attacks where an attacker redirects our data dir.
   - `metadata(data_dir).permissions().mode() & 0o077 == 0` — if group/world bits are set, log warning and refuse to write (disable persistence). A first-run self-created dir passes this.
   - `metadata(data_dir).uid() == geteuid()` — if owned by a different user, refuse. (Uses `std::os::unix::fs::MetadataExt`.)
4. **Windows/WSL.** We require WSL for Windows per the project brief, so the unix path applies. A native-Windows path is explicitly out of scope.
5. **Fail-closed.** Any of the above failures: disable persistence for the session, print one stderr warning *before* raw mode, continue. The game remains fully playable; only the save/load paths are inert.

**Atomic write procedure.**

1. Create a named temp file via `tempfile::NamedTempFile::new_in(data_dir)` — guarantees same-filesystem so `rename` is atomic.
2. On unix, `chmod 0o600` on the temp file before writing.
3. Write JSON; call `file.sync_all()` (fsync the data file).
4. `NamedTempFile::persist(target)` — atomic rename.
5. Open the parent directory (`File::open(data_dir)`) and call `sync_all()` on it so the rename is durable (unix only; no-op on windows).

**Symlink behavior.** The *target file* (`high_scores.json`) is re-validated before every save: if `symlink_metadata` shows it is a symlink, refuse to save (log warning), do not follow. We also refuse to save if target exists and is a directory (`persist` would fail loudly anyway; we pre-check for a better error).

**Corrupt-file recovery (v0.3 updated).** On parse failure at load time:

1. Move the bad file to `high_scores.json.corrupt.{unix_timestamp}` via atomic rename. This preserves all prior corrupt copies for forensic inspection; we do **not** overwrite existing `.corrupt.*` files.
2. Log to stderr (before raw mode) the corruption event and the preserved filename.
3. Start the session with an empty list; continue normally.
4. A housekeeping pass on load caps the number of `.corrupt.*` files at 5 (oldest deleted); this prevents unbounded growth from a buggy writer. Tested.

**Configuration (v0.1 scope cut).** Optional TOML config file is **deferred to v0.2**. v0.1 uses: compiled-in defaults for DAS/ARR/soft-drop, CLI flags for behavior toggles (`--seed`, `--no-color`, `--reset-scores`), and environment variables where standard (`NO_COLOR`). This removes the `toml` crate dependency, path-validation ambiguity, and the attack surface of parsing an arbitrary user file, for a v0.1 feature that has no user story. Tests for a TOML config belong in v0.2.

**SSH responsiveness (targets + tests).**

- **Bytes/frame target (informational, not a CI gate):** steady-state idle frame writes ≤ 16 bytes; mid-game one-cell move ≤ 128 bytes. Measured in a *benchmark* (`cargo bench`), reported in CI summaries, but not gated — terminal emulator realities (cursor positioning, color escape sequences, ratatui internals) make exact byte counts brittle as a hard gate. If a benchmark run exceeds 2× the target, CI flags it as a soft warning; the team reviews manually.
- **PTY integration test (new in v0.3).** A `rexpect`-based test (unix only) allocates a PTY, launches the binary with `--seed`, sends a scripted input sequence, and asserts (a) the PTY receives output within 50 ms of an input, (b) on `q` the process exits 0 and the PTY is left in cooked mode (verified via the post-exit `stty -a` on the *same PTY*, which is the harness correction from v0.2's review). This replaces v0.2's fragile byte-ceiling-as-CI-gate with a behaviorally meaningful test.
- **Input-to-visible-change latency (local):** < 20 ms p99 measured with fake clock, from `Input` enqueue to corresponding `Buffer` diff emission. Achievable because (see "Frame cadence" below) the loop does not gate draws on a fixed 60 Hz timer.
- **Event poll timeout:** 8 ms. See frame cadence.
- **Resize:** next tick after `Event::Resize` *or* `winch_pending`, renderer forces a full redraw; no panic on shrink below minimum (we enter the "too small" overlay state).
- **Slow stdout / backpressure:** `BufWriter<Stdout>` with an 8 KiB buffer; explicit `flush` once per frame. On `ErrorKind::BrokenPipe`, exit cleanly via guard.

**Frame cadence vs. event poll (authoritative).** The loop is **event-driven, not fixed-timestep**:

```
loop {
    if flags.shutdown.load() { break; }
    handle_tstp_cont_winch(&flags, &mut terminal);
    let deadline = last_draw + Duration::from_millis(16);
    let poll_timeout = deadline.saturating_duration_since(Instant::now()).min(Duration::from_millis(8));
    if event::poll(poll_timeout)? {
        let ev = event::read()?;
        inputs.extend(input.translate(ev, clock.now()));
    }
    let dt = clock.now() - last_step;
    last_step = clock.now();
    let events = state.step(dt, &inputs);
    inputs.clear();
    if state.dirty() || flags.winch_pending.swap(false, Relaxed) {
        renderer.draw(&state, &mut terminal)?;
        last_draw = clock.now();
    }
}
```

Worst-case input-to-visible-change latency: `poll_timeout (≤ 8 ms) + step (sub-ms) + draw (< 2 ms for a diffed frame) < 11 ms`. The 20 ms p99 target in §4 has comfortable headroom. Fixed 60 Hz is a ceiling (we never draw more often than that), not a floor.

**CLI flags.**

- `--seed <u64>` — deterministic bag.
- `--reset-scores` — prompts (y/N) **before** entering raw mode; writes empty score file atomically, then exits 0. If persistence is unavailable (per directory validation), prints a message and exits 1 without a prompt.
- `--no-color` — force monochrome (also honors `NO_COLOR` env var).
- `--version`, `--help` — standard clap output, no TTY required.

**v0.1 scope decisions (resolved).**

- **scope-and-modes:** single-player Marathon only.
- **rotation-and-rules:** full SRS kicks; Guideline-style scoring (no T-spins); lock delay with 15-reset cap, semantics fully specified; 7-bag.
- **rendering-stack:** Rust + `ratatui` over `crossterm`; double-wide cells; dirty-region diffing.
- **input-and-timing:** single-threaded main loop; event-driven with 8 ms poll; DAS/ARR synthesized from held_direction timing (terminal-reality-aware); soft drop with 20× gravity capped at 30 ms/cell.
- **persistence-and-config:** JSON high-scores via `directories` + `tempfile` atomic rename + parent-dir fsync + 0o700 dir + 0o600 file + symlink/ownership validation + timestamped `.corrupt` backups + fail-closed degradation; **TOML config deferred to v0.2**.
- **legal:** distributed as `blocktxt`; README carries branding/naming section; SRS tables encoded from derivation, not vendored.

**v0.1 "nice-to-have" cut line.** IN: 7-bag, next-piece preview, ghost piece. OUT (v0.2): hold piece, T-spin detection, combo, color themes, TOML config.

## 5. Tests plan

**Red/green TDD call-outs.** Built **test-first** — failing test committed before implementation:

- `srs.rs` wall-kick tables and rotation resolution.
- `bag.rs` 7-bag **aligned-bag** invariants (see §5 test 3).
- `rules.rs` scoring for single/double/triple/4-line-clear and B2B multiplier including empty-lock-does-not-break rule.
- `rules.rs` gravity formula values at levels 1, 5, 10, 15, 20.
- `rules.rs` lock-delay full semantics: single-reset, cap-reached-mid-air, timer-expire-airborne (no-op), hard-drop-short-circuit.
- `rules.rs` soft-drop capped rate (level-varying and cap-binding cases).
- `board.rs` line detection + row collapse.
- `board.rs` top-out: block-out, lock-out, partial-lock, normal-lock cases, plus spawn-position cases for I, O, JLSTZ.
- `persistence.rs` atomic write + timestamped corrupt-file recovery + `.corrupt.*` cap.
- `persistence.rs` directory validation: symlink data dir refused, world-writable dir refused, wrong-owner dir refused, missing HOME handled (persistence disabled, not panic).
- `terminal.rs` setup rollback on intermediate failure.
- `input.rs` DAS/ARR state machine under fake clock.
- `render/helpers.rs` pure helpers: ghost-Y computation, HUD numeric formatting.

Full-frame rendering is NOT TDD'd — developed against an in-memory `Buffer`, covered by snapshot tests. Pure render *helpers* ARE TDD'd.

**CI tests (must pass on every PR):**

1. **Unit tests** — `cargo test` across all modules. Fast, deterministic, seeded RNG.
2. **SRS conformance suite** — golden test of every kick from every rotation state, asserted against an embedded table derived from the SRS specification (see §1a legal).
3. **Bag invariant property test.** Using `proptest` over 1000 seeds, draw 700 pieces each and assert: (a) pieces `[7k .. 7k+7)` are a permutation of `IOTSZJL` for every `k` (aligned-bag invariant); (b) maximum gap between two occurrences of the same piece is ≤ 12; (c) minimum gap is ≥ 1 (no immediate repeats within a bag). Sliding-window uniqueness is explicitly not asserted.
4. **Scoring table test** — parameterized over 4 clear types × 5 levels × {b2b_active=T/F going in} × {line-clear, empty-lock after}. Asserts both the point value and the B2B state transition per §4.
5. **Soft-drop capped-rate test** — parameterized over levels 1, 10, 20: assert that under continuous `SoftDropOn` and a fake clock, descent rate matches `max(natural/20, 30ms/cell)` within ±1 ms.
6. **Lock-delay test suite** — the five scenarios above, each with a scripted input + fake-clock trace asserting lock frame and final score.
7. **Snapshot render tests** — render `GameState` fixtures into a ratatui `Buffer`; compare to committed `.snap` files via `insta`. Covers: empty board, mid-game, game-over overlay, pause overlay, no-color fallback, terminal-too-small overlay.
8. **Render helper unit tests** — `ghost_y(&board, &piece)` over a fixture matrix (flat floor, notched floor, overhang); `hud::format_score(12345, 7, 3)` golden-string. These sit under `render/helpers.rs` and have no Buffer dependency.
9. **Integration test: headless game loop** — `GameState` with fake `Clock` + seeded `Rng`, scripted input sequence, assert final score/lines/level.
10. **Persistence round-trip + failure modes** —
    - write, read back, assert equality;
    - corrupt the file, assert `.corrupt.{ts}` created and fresh list returned;
    - pre-existing `.corrupt.*` files are preserved (not overwritten) and capped at 5;
    - pre-existing symlink target for `high_scores.json` is refused (save is a no-op, warning emitted);
    - world-writable `data_dir` is refused (persistence disabled for the session);
    - missing HOME / `ProjectDirs::from` returns `None` → persistence disabled, game continues;
    - parent-dir fsync path exercised (unix only, `#[cfg(unix)]`).
11. **Top-out scenarios test** — scripted boards that trigger block-out, lock-out, partial lock, and normal lock.
12. **DAS/ARR parameterized test** — for each of (DAS=170, ARR=50) and (DAS=100, ARR=20): hold direction for 500 ms of fake-clock time, assert the exact sequence and timestamps of emitted `MoveLeft` events. Includes the held-direction timeout case (no press for 200 ms clears held state).
13. **Terminal lifecycle tests (rewritten for v0.3).**
    - **Panic-safety (PTY)**: use `rexpect` to allocate a PTY, launch the binary with a hidden `--panic-now` flag compiled under `cfg(debug_assertions)`, and after exit assert the PTY is in cooked mode by reading its termios via `nix::sys::termios::tcgetattr` on the PTY master (not parent's stdin). Also scan the captured stream for the restore escape sequences at the end.
    - **SIGINT (PTY)**: send SIGINT via `kill(pid, SIGINT)`; assert exit code 0, termios restored, restore escapes at stream end.
    - **SIGTSTP/SIGCONT (PTY, unix only)**: send SIGTSTP, wait for process to stop (`waitpid` with `WUNTRACED`), assert termios on the PTY master is cooked (restored), send SIGCONT, send a key, assert output resumes and termios is raw again.
    - **`--reset-scores` prompt runs in cooked mode**: `assert_cmd` (pipes stdin) feeds `n\n`, asserts file is unchanged and captured stdout contains no CSI escape sequences.
    - **Setup rollback**: unit-tested by injecting a failing stage via a test-only seam in `TerminalGuard`.
14. **Cross-platform CI** — GitHub Actions matrix: `ubuntu-latest` + `macos-latest`, stable toolchain + `1.75` MSRV check; `cargo fmt --check`; `cargo clippy -D warnings`; `cargo test`; `cargo build --release` artifact upload. PTY-based lifecycle tests run only on the two unix OSes (the `#[cfg(unix)]` gate).
15. **Dependency hygiene** — `cargo deny check` (licenses + advisories) and a check that `Cargo.lock` is committed and unchanged by `cargo check`. Daily scheduled workflow opens issues on new advisories.
16. **Naming/legal check** — `scripts/check-naming.sh` grep over release artifacts for prohibited strings (see §1a).

**Performance (benchmarks, not hard gates).** Byte-per-frame numbers from §4 are tracked via `cargo bench` with `criterion` on PRs and posted as a CI summary; a 2× regression from the baseline triggers a soft warning, not a failure. The PTY-based end-to-end latency test (above) is the behavioral gate.

**Manual test plan (mirrors user stories).** Each user story in §2 has a one-paragraph walkthrough in `docs/manual-test.md` — run before tagging a release. Story 4 includes an explicit SSH walkthrough (`mosh`/`ssh` into a Linux box, play a game, confirm responsiveness subjectively).

## 6. Team

v0.1 is scoped to **2 engineers** for sprints 1–2 and 4, plus **1 QA engineer** for sprint 3.

- **Veteran Rust systems + TUI engineer (1)** — owns build, CI matrix, MSRV check, dependency locking + advisory response, `terminal.rs` + signal design (flag-based, async-signal-safe), `persistence.rs` with directory hardening + atomic semantics + degraded-mode behavior, CLI, `render/` module, double-wide cells, color/glyph fallback, snapshot tests, PTY test harness.
- **Veteran game-logic engineer w/ falling-block domain knowledge (1)** — owns `game/` entirely: `srs.rs`, `bag.rs`, `rules.rs` (incl. full lock-delay semantics + soft-drop capped rate + B2B state machine), `board.rs`, `piece.rs`, `score.rs`, top-out. This is the hardest-to-get-right role and stays dedicated.
- **Veteran QA / test engineer (1, sprint 3 only)** — property tests, SRS conformance suite, DAS/ARR parameterized suite, terminal-lifecycle PTY tests, persistence failure-mode suite, snapshot fixtures, manual test doc, naming-check script. The other two engineers TDD their own code; QA provides an independent hardening pass before release.

Total: 2 engineers for sprints 1–2 and 4, +1 QA for sprint 3.

## 7. Implementation plan

Four sprints of ~1 week each.

### Sprint 1 — Foundation (serial handoff, then parallel)

- **Systems/TUI eng:** repo scaffold, `cargo` workspace, CI matrix (Ubuntu + macOS, stable + 1.75), `rustfmt`/`clippy`/`cargo deny` gates, `TerminalGuard` w/ ordered setup + rollback, panic hook (static restore fn), flag-based `signals.rs` + main-loop observer for SIGINT/SIGTERM/SIGTSTP/SIGCONT/SIGWINCH, CLI skeleton w/ `clap`, `--reset-scores` pre-TTY prompt wired, naming-check script skeleton. Ships a binary that opens + closes cleanly under normal exit, panic, Ctrl-C, and Ctrl-Z/fg.
- **Game-logic eng:** `board.rs` (incl. coord system + top-out cases), `piece.rs` shape tables + spawn positions, collision primitive, first failing SRS test (red). TDD.

Exit criteria: `cargo test` passes; binary opens/closes cleanly on both OSes under all five signal paths (including SIGWINCH); empty playfield renders; top-out + spawn-position test cases red-to-green.

### Sprint 2 — Core mechanics (parallel)

- **Game-logic eng:** implement `srs.rs` (turn red tests green), `bag.rs` with aligned-bag invariant tests, `rules.rs` gravity + full lock-delay semantics + soft-drop capped rate + scoring + B2B state. All TDD.
- **Systems/TUI eng:** `persistence.rs` with directory hardening (0o700 mkdir, symlink/ownership validation, degraded-mode fallback), `tempfile` + `sync_all` + parent-dir fsync + 0o600 perms + timestamped `.corrupt` backups; `clock.rs` abstraction; wire `Renderer` to real `GameState`; ghost piece + next preview; `NO_COLOR` + glyph fallback; `render/helpers.rs` pure helpers unit-tested.

Exit criteria: a human can play a full game end-to-end; all core TDD tests green; persistence tests green including all failure modes; soft-drop cap + lock-delay full semantics tested.

### Sprint 3 — Polish, hardening, QA pass (parallel, +QA eng)

- **Game-logic eng:** soft-drop/hard-drop scoring wiring, B2B wired into line-clear events, level progression wiring, top-out wiring into state machine.
- **Systems/TUI eng:** line-clear animation, pause overlay, game-over overlay w/ new-best highlight, title screen w/ top-5, DAS/ARR input model (held-direction timing + kitty protocol detection + heuristic fallback), resize + too-small overlay, `--seed`/`--reset-scores`/`--no-color` flags, finalize legal footer + naming check, PTY test harness skeleton.
- **QA eng:** SRS conformance golden suite; bag property test (aligned-bag + max-gap); scoring parameterized test incl. B2B transitions; DAS/ARR parameterized test under fake clock; snapshot fixtures incl. no-color + too-small; integration headless game loop test; terminal-lifecycle PTY tests (panic, SIGINT, SIGTSTP/SIGCONT, reset-scores in cooked mode); persistence failure-mode suite (symlink, world-writable, wrong-owner, no HOME, corrupt + .corrupt cap); PTY-based end-to-end latency test; naming-check test; manual test doc drafted.

Exit criteria: feature-complete for v0.1 scope; CI matrix green on both OSes; manual test doc complete; all lifecycle + persistence failure-mode tests green; legal/naming check passes.

### Sprint 4 — Release (converging)

- All: bug bash against manual test plan, fix regressions, finalize README (incl. honest Guideline-inspired scoping, branding/naming section, keybinds, install), tag v0.1.0 under distributed name `blocktxt`.
- **Systems/TUI eng:** release profile tuning (`lto = "thin"`, `codegen-units = 1`), binary size check, release artifacts for macOS (arm64 + x86_64) and Linux (x86_64), run naming-check script against the artifact set.
- **QA eng:** re-run snapshot + PTY suites on release build; sign off on user-story walkthroughs; run naming-check script against README and artifacts.

**Parallelization map.** After sprint 1, game-logic ↔ systems/TUI are independent because `GameState` is pure and `Renderer` only reads it — interface nailed down in sprint 1. QA in sprint 3 follows both by ~1 day per area.

## 8. Embedded Changelog

- **v0.3 (current)** — Ops & correctness hardening round. Fill in the architecture diagram with concrete component boundaries + contracts (closing the v0.2 placeholder). Rewrite signal handling to be async-signal-safe: handlers only set atomic flags; the main loop performs all terminal I/O (SIGTSTP restore + re-raise, SIGCONT re-enter + redraw, SIGWINCH redraw). Resolve soft-drop conflict to a single model: 20× natural gravity capped at 30 ms/cell. Fully specify lock-delay reset-counter semantics (re-ground behavior, timer-expire-airborne no-op, 15-reset cap, hard-drop short-circuit). Clarify O-piece and I/JLSTZ spawn positions. Specify B2B state machine (empty lock does not break chain; non-4-line-clear breaks; first 4-line clear establishes). Harden persistence directory: 0o700 mkdir, symlink/ownership/world-writable validation, fail-closed with in-memory fallback; switch corrupt-file recovery to timestamped `.corrupt.*` backups with a 5-file cap. Cut TOML config file from v0.1 (defer to v0.2) to shrink attack surface. Specify terminal-reality-aware DAS/ARR using held-direction timing + kitty-protocol probe with heuristic fallback. Add §1a Branding section: distributed as `blocktxt`, independently-derived SRS tables, release-time naming-check script. Downgrade SSH byte-budget from hard CI gate to benchmark-with-soft-warning; add PTY-based lifecycle + latency tests via `rexpect` (fixes v0.2's broken `stty` harness). Add render-helper unit tests and DAS/ARR parameterized test. Reframe dependency policy from "pinned" to "locked via Cargo.lock" with advisory response cadence.
- **v0.2** — Tighten correctness + ops: fix 7-bag test invariant to aligned bags + max-gap; specify coordinate system, block-out vs lock-out, partial-lock; expand terminal lifecycle to cover SIGINT/SIGTERM/SIGTSTP/SIGCONT/resize/backpressure/setup-rollback; sequence `--reset-scores` prompt before raw mode; pin crate versions + add missing `toml`/`tempfile`/`proptest`/`insta`/`signal-hook`/`assert_cmd` with MSRV 1.75 CI check; harden persistence with `tempfile`, `sync_all`, parent-dir fsync, `0o600` perms, symlink behavior; downgrade claim to "Guideline-inspired"; add measurable SSH responsiveness budgets; reduce team to 2 engineers + 1 sprint-3 QA.
- **v0.1** — Initial spec. Scope: Rust + ratatui/crossterm; Guideline SRS + 7-bag + ghost + next-preview; Marathon mode; JSON high-score persistence; macOS + Linux CI. Deferred to v0.2: hold piece, T-spin detection, color themes.
