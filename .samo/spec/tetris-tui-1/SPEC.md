# tetris-tui-1 — SPEC v0.2

## 1. Goal & why it's needed

**Goal.** Ship a polished, single-player terminal Tetris clone with Guideline-inspired mechanics (SRS rotation + kicks, 7-bag, ghost, next-preview, Guideline scoring for singles/doubles/triples/tetrises), playable on macOS and Linux terminals (and Windows via WSL), with persistent high scores.

**Why this exists.** Existing TUI Tetris clones in the wild fall into two buckets: (a) toys with non-standard rotation and wrong scoring that feel "off" to anyone who has played real Tetris, and (b) heavy GUI ports that require a windowing stack. There is a genuine niche for a *correct-enough*, lightweight, keyboard-only Tetris that lives in a terminal tab — fast to launch, zero deps beyond a single binary, and faithful enough to Guideline mechanics that muscle memory from mainstream Tetris transfers cleanly for the core move set (movement, rotation with kicks, drops, line clears). This project exists to fill that niche.

**Guideline-compliance honesty.** v0.1 is explicitly **Guideline-inspired, not fully Guideline-compliant**. v0.1 ships: SRS rotation with JLSTZ + I kick tables, 7-bag, ghost, next-preview, Guideline scoring for non-T-spin clears including Back-to-Back-Tetris, lock delay w/ reset cap. v0.1 does **not** ship: hold piece, T-spin detection/scoring, combo counter, color themes — these are v0.2. README and title screen surface this honestly so a Guideline purist is not misled.

**Non-goals (explicit).** No multiplayer, no netcode, no sound, no GUI. These are out of scope per the brief and must not re-enter the design in disguise (e.g. no "stub network module", no audio crate pulled in "for later").

## 2. User stories

1. **Keyboard-native dev on macOS.** As a developer who lives in iTerm2/Ghostty/Alacritty, I want to run `tetris-tui` in a terminal tab and immediately play a full game with arrow-key / WASD / vim-style bindings, so I can take a five-minute break without leaving my terminal workflow. Outcome: the binary launches into a playable board in under 200 ms, and I can clear lines, pause with `p`, and quit with `q` without reading docs.
2. **Returning player chasing a high score.** As a returning player, I want my best score persisted across sessions so I can try to beat it tomorrow. Outcome: after a game-over, my score is written to a local high-score file; next launch shows the top-5 on the title screen, and a new personal best is highlighted on the game-over screen.
3. **Purist who expects real Tetris feel (core move set).** As a player who has played Guideline Tetris, I want SRS rotation with standard wall kicks, a 7-bag randomizer, a ghost piece, and a next-piece preview, so the core feel matches the Tetris I know. Outcome: SRS kicks match the published kick table (tested exhaustively); I never get the same piece four times in a row impossibly; I can see where my piece will land and what's coming next. (Hold + T-spin are called out as v0.2 in the README so this story's expectations are scoped honestly.)
4. **Linux power user on a slow SSH session.** As a user playing over SSH from a laptop on hotel Wi-Fi, I want the renderer to only redraw changed cells so input feels responsive even at 30–50 ms RTT. Outcome: measurable acceptance — see §4 "SSH responsiveness" — including bytes/frame ceiling, input-to-visible-change latency target, and graceful behavior on resize and slow stdout.
5. **Accessibility-minded player.** As a player on a monochrome or 16-color terminal, I want the game to remain fully playable without 256-color support, using distinct ASCII glyphs per tetromino. Outcome: the game detects terminal color capability and falls back to glyph-based differentiation when color is unavailable or disabled via `NO_COLOR`.

## 3. Architecture

<!-- architecture:begin -->

```text
(architecture not yet specified)
```

<!-- architecture:end -->

**Language & stack.** Rust (stable, MSRV **1.75**, verified in CI via `cargo +1.75 check`). Chosen because: (a) single static binary, trivial cross-platform distribution; (b) `ratatui` + `crossterm` is the most mature cross-platform TUI stack today; (c) strong type system makes the state machine (SRS kicks, lock delay, line-clear phases) much easier to get right than in a dynamic language.

**Crate dependencies (v0.1, pinned, MSRV-verified).** Versions below are the current targets; CI enforces `Cargo.lock` is committed and `cargo +1.75 check` passes.

Runtime:

- `ratatui = "0.28"` — widget/framebuffer layer.
- `crossterm = "0.28"` — cross-platform terminal I/O backend.
- `rand = "0.8"`, `rand_chacha = "0.3"` — seedable RNG for the 7-bag.
- `serde = { version = "1", features = ["derive"] }`, `serde_json = "1"` — high-score persistence.
- `toml = "0.8"` — optional config loader.
- `tempfile = "3"` — named temp files for atomic rename.
- `directories = "5"` — platform config/data dirs.
- `anyhow = "1"`, `thiserror = "1"` — error handling at boundaries.
- `clap = { version = "4", features = ["derive"] }` — CLI flags.
- `signal-hook = "0.3"` — SIGINT/SIGTERM/SIGTSTP/SIGCONT handling (unix only; gated `#[cfg(unix)]`).

Dev-only:

- `proptest = "1"` — property tests for bag + scoring.
- `insta = "1"` — snapshot tests for renderer.
- `assert_cmd = "2"`, `predicates = "3"` — CLI integration tests (panic-safety, reset-scores prompt).

No async runtime. No audio. No networking.

**Module layout (`src/`):**

```
main.rs            // CLI parsing, pre-TTY actions, terminal setup/teardown
app.rs             // AppState enum; main loop; signal + resize handling
terminal.rs        // TerminalGuard (RAII): raw+alt+hide on enter, restore on drop
game/
  mod.rs
  board.rs         // 10x40 playfield (coords + top-out defined in §4)
  piece.rs         // Tetromino enum + shape tables + rotation states
  srs.rs           // SRS rotation + wall-kick tables
  bag.rs           // 7-bag with seedable RNG
  rules.rs         // gravity curve, lock delay, scoring, level progression
  score.rs         // Score + line-clear classification
input.rs           // Key → Input translation; DAS/ARR state machine
render/
  mod.rs
  board_view.rs
  hud.rs
  theme.rs         // 256/16-color + NO_COLOR fallback
persistence.rs     // high-score file load/save (atomic)
config.rs          // optional TOML config
clock.rs           // Clock trait (real + fake)
signals.rs         // shutdown flag + SIGTSTP/SIGCONT handlers (#[cfg(unix)])
```

**Key abstractions.**

- `GameState` — pure, deterministic. All mutation via `GameState::step(dt, &[Input]) -> Vec<Event>`. No I/O.
- `Clock` trait — real wraps `Instant`; fake is a manually-advanced counter.
- `Rng` (trait-object or generic) — real is `ChaCha8Rng`; tests inject seeded instances.
- `Renderer` — pure read on `&GameState`, writes into a `ratatui::Buffer`.
- `Input` enum — `MoveLeft`, `MoveRight`, `SoftDrop`, `HardDrop`, `RotateCW`, `RotateCCW`, `Pause`, `Quit`, `Restart`. (`Hold` deferred to v0.2.)
- `TerminalGuard` — RAII wrapper: enter raw/alt/hide-cursor on construction; restore unconditionally on drop *and* via panic hook *and* via signal handler path.

**Data flow.**

```
terminal keys → crossterm::Event → input::translate → Input
                                                        │
  Clock::now() ─► main loop ─► GameState::step(dt, inputs) ─► Events
                                                        │
                                  Renderer::draw(&GameState) → ratatui Frame → stdout
```

Fixed ~60 Hz redraw cadence; simulation is dt-driven so render and sim rates are decoupled.

**Terminal rendering details.**

- Each board cell is drawn **two characters wide** (`"[]"` or `"██"`) so blocks look roughly square. Playfield: `20 rows × 10 cols × 2 chars = 20×20` character grid.
- Colors: 8 Guideline-like piece colors when 256-color is available; fall back to 16-color; fall back to per-piece ASCII glyphs (`I`,`O`,`T`,`S`,`Z`,`J`,`L`) when `NO_COLOR` is set or color is unsupported.
- Only dirty regions are redrawn (ratatui diffs against its internal buffer).

**Terminal lifecycle, signals, and teardown guarantees.** A TUI that leaves the terminal in raw/alternate/cursor-hidden state on exit is a core ops failure. v0.1 guarantees restore on:

1. **Normal exit** — `TerminalGuard::drop` runs.
2. **Panic** — panic hook installed *before* `TerminalGuard::enter()`; hook calls the restore routine, then the default hook. The guard's `drop` is idempotent so a second restore is a no-op.
3. **SIGINT / SIGTERM** — `signal-hook` sets an `AtomicBool` shutdown flag; main loop observes it next tick and exits cleanly, invoking the guard's drop.
4. **SIGTSTP (Ctrl-Z) / SIGCONT** — on SIGTSTP: restore terminal, re-raise SIGTSTP to actually suspend. On SIGCONT: re-enter raw/alt/hide, force a full redraw. Implemented via a small handler in `signals.rs`; Windows (WSL) path is the unix path since we require WSL.
5. **Setup failure partway through** — `TerminalGuard::enter()` is ordered (raw → alt → hide) and on any intermediate error, reverses the steps already taken before returning the error. Tested.
6. **Terminal resize** — `crossterm::Event::Resize` is handled in the main loop; if the new size is below the minimum (`44×24` chars), the game auto-pauses and shows a "terminal too small" overlay; resuming redraws fully.
7. **Slow stdout / backpressure** — writes are buffered via `BufWriter<Stdout>`; on write failure, the loop exits cleanly via the same guard path.

**Pre-TTY actions.** CLI actions that need user interaction or can fail loudly run **before** `TerminalGuard::enter()`: `--reset-scores` (with stdin y/N prompt, line-buffered), `--version`, `--help`, and any path validation for `--config`. This avoids the well-known TUI failure mode of prompting while the terminal is in raw mode.

## 4. Implementation details

**Coordinate system & top-out (fully specified).**

- Board is a `10 cols × 40 rows` grid. Origin `(0,0)` is **top-left**. `x` increases rightward (0..10). `y` increases downward (0..40).
- Rows `0..20` are the **hidden buffer** (used for spawn + above-visible locks). Rows `20..40` are the **visible playfield**. (This is the inverse of the Tetris Guideline's convention of indexing from the bottom, but consistent within our codebase — we pick top-origin because ratatui addresses rows top-down and this eliminates one coordinate flip.)
- **Spawn position.** Each piece spawns with its bounding-box top at row `18` and horizontally centered on cols `3..7` (or `3..6` for O). If *any* cell of the spawned piece overlaps an occupied cell → **block-out**, game over immediately.
- **Lock-out.** After a piece locks, if *every* cell of the just-locked piece has `y < 20` (entirely in the hidden buffer, above the visible area) → **lock-out**, game over. (This replaces the ambiguous v0.1-draft wording.)
- **Partial lock.** If some cells are in the hidden buffer and some visible, the game continues.
- Test cases for each of {block-out, lock-out, partial lock, normal lock} are committed as part of §5.

**Rotation system — SRS.**

- 4 rotation states per piece (0, R, 2, L).
- JLSTZ share one wall-kick table (5 offsets per transition, 8 transitions).
- I piece has its own kick table.
- O piece does not kick.
- Rotation attempt: test base rotation; if blocked, test kicks in order; first non-colliding offset wins; if all fail, rotation is rejected.
- Kick tables are `const` arrays in `srs.rs` and unit-tested against the published SRS reference values.

**7-bag randomizer.** Sequence of 7 tetrominoes, shuffled (Fisher–Yates using the injected `Rng`), drained one at a time; when empty, refill + reshuffle. Seedable via `--seed`.

**Gravity & level curve.** `gravity_seconds_per_cell = (0.8 − (level−1) × 0.007) ^ (level−1)`, clamped at level 20. Soft drop = 20× gravity (+1 pt/cell). Hard drop = instant (+2 pt/cell).

**Lock delay.** 500 ms extended placement; resets on successful move/rotate up to a cap of 15 resets; piece locks when timer expires or after 15 resets while still grounded.

**Line clears.** After lock: detect full rows, play a brief clear animation (2 frames of inverted cells, ~100 ms), then collapse. Scoring (Guideline-style, no T-spin in v0.1):

| Clear | Points |
| --- | --- |
| Single | 100 × level |
| Double | 300 × level |
| Triple | 500 × level |
| Tetris | 800 × level |
| Back-to-back Tetris | ×1.5 |

Soft-drop: +1/cell. Hard-drop: +2/cell.

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

**Input handling (DAS/ARR).**

- DAS (default 170 ms): hold left/right → wait DAS → auto-shift.
- ARR (default 50 ms): interval between auto-shifts while held.
- Soft-drop repeat: default 30 ms.
- Configurable in `config.toml`.

**Persistence (atomic, hardened).**

- File: `{data_dir}/high_scores.json`, where `data_dir = ProjectDirs::from("", "", "tetris-tui").data_dir()`.
- Format: JSON array of `{score, lines, level, date}`, top 10 kept.
- **Atomic write procedure:**
 1. Create a named temp file via `tempfile::NamedTempFile::new_in(data_dir)` — guarantees same-filesystem so `rename` is atomic.
 2. Write JSON; call `file.sync_all()` (fsync the data file).
 3. `NamedTempFile::persist(target)` — atomic rename over any existing file. On Unix this refuses to cross filesystems.
 4. Open the parent directory (`File::open(data_dir)`) and call `sync_all()` on it so the rename is durable (unix only; no-op on windows).
- **Permissions:** on unix, set mode `0o600` on the temp file before rename (via `std::os::unix::fs::PermissionsExt`). Score file is user-private.
- **Symlink/race behavior:** we `persist` over the target path; if the target is a symlink, the rename replaces the link itself (not the link target). If the target exists and is a directory, `persist` fails loudly — we do not fall back.
- **Corrupt file recovery:** on parse failure, move the bad file to `high_scores.json.bak` (also atomic rename), log to stderr, start with an empty list. If `.bak` already exists, it is overwritten — we keep only the most recent corrupt copy.
- Config file (optional): `{config_dir}/config.toml`. Missing file = defaults; parse errors are logged and defaults are used (we don't fail startup).

**SSH responsiveness (measurable targets).**

- **Bytes/frame budget:** steady-state idle frame writes ≤ 16 bytes (cursor reposition + no cell changes). Mid-game frame with piece moving one cell writes ≤ 128 bytes. Measured by a test that captures the `crossterm` write stream into a `Vec<u8>`.
- **Input-to-visible-change latency (local):** < 20 ms p99 measured with fake clock, from `Input` enqueue to corresponding `Buffer` diff emission.
- **Event poll timeout:** 8 ms, so input latency ≤ 1 poll + 1 step; loop wakes promptly on keypress via `crossterm::event::poll` returning true.
- **Resize:** next tick after `Event::Resize`, renderer forces a full redraw; no panic on shrink below minimum (we enter the "too small" overlay state).
- **Slow stdout / backpressure:** `BufWriter<Stdout>` with an 8 KiB buffer; explicit `flush` once per frame. On `ErrorKind::BrokenPipe`, exit cleanly via guard.

**CLI flags.**

- `--seed <u64>` — deterministic bag.
- `--reset-scores` — prompts (y/N) **before** entering raw mode; writes empty score file atomically, then exits 0.
- `--config <path>` — override config file location.
- `--no-color` — force monochrome (also honors `NO_COLOR` env var).
- `--version`, `--help` — standard clap output, no TTY required.

**v0.1 scope decisions (resolved from interview).**

- **scope-and-modes:** single-player Marathon only.
- **rotation-and-rules:** full SRS kicks; Guideline-style scoring (no T-spins); lock delay with 15-reset cap; 7-bag.
- **rendering-stack:** Rust + `ratatui` over `crossterm`; double-wide cells; dirty-region diffing.
- **input-and-timing:** single-threaded main loop; `crossterm::event::poll` with 8 ms timeout; DAS/ARR in-sim with dt.
- **persistence-and-config:** JSON high-scores via `directories` + `tempfile` atomic rename + parent-dir fsync; optional TOML config; `NO_COLOR` respected.

**v0.1 "nice-to-have" cut line.** IN: 7-bag, next-piece preview, ghost piece. OUT (v0.2): hold piece, T-spin detection, combo, color themes.

## 5. Tests plan

**Red/green TDD call-outs.** Built **test-first** — failing test committed before implementation:

- `srs.rs` wall-kick tables and rotation resolution.
- `bag.rs` 7-bag **aligned-bag** invariants (see correction below).
- `rules.rs` scoring for single/double/triple/tetris and back-to-back multiplier.
- `rules.rs` gravity formula values at levels 1, 5, 10, 15, 20.
- `rules.rs` lock-delay 15-reset cap.
- `board.rs` line detection + row collapse.
- `board.rs` top-out: block-out, lock-out, partial-lock, normal-lock cases.
- `persistence.rs` atomic write + corrupt-file recovery.
- `terminal.rs` setup rollback on intermediate failure.

UI / rendering code is NOT TDD'd — developed against an in-memory `Buffer`, then covered by snapshot tests.

**CI tests (must pass on every PR):**

1. **Unit tests** — `cargo test` across all modules. Fast, deterministic, seeded RNG.
2. **SRS conformance suite** — golden test of every kick from every rotation state, asserted against an embedded table derived from the published SRS spec.
3. **Bag invariant property test (corrected).** Using `proptest` over 1000 seeds, draw 700 pieces each and assert: (a) pieces `[7k .. 7k+7)` are a permutation of `IOTSZJL` for every `k` (the correct aligned-bag invariant — *not* arbitrary sliding windows); (b) maximum gap between two occurrences of the same piece is ≤ 12 (the standard 7-bag worst-case); (c) minimum gap is ≥ 1 (no immediate repeats within a bag). Sliding-window uniqueness is **not** a property of the standard 7-bag and is explicitly not asserted.
4. **Scoring table test** — parameterized over 4 clear types × 5 levels × with/without B2B.
5. **Snapshot render tests** — render `GameState` fixtures into a ratatui `Buffer`; compare to committed `.snap` files via `insta`. Covers: empty board, mid-game, game-over overlay, pause overlay, no-color fallback, terminal-too-small overlay.
6. **Integration test: headless game loop** — `GameState` with fake `Clock` + fake `Rng`, scripted input sequence, assert final score/lines/level.
7. **Persistence round-trip + failure modes** — write, read back, assert equality; corrupt the file, assert `.bak` created and fresh list returned; pre-existing symlink target handled per spec; parent-dir fsync path exercised (unix only, gated by `#[cfg(unix)]`).
8. **Top-out scenarios test** — scripted boards that trigger block-out, lock-out (piece entirely in `y < 20`), partial lock (survives), and normal lock.
9. **Terminal lifecycle tests.**
 - Panic-safety: force a panic mid-game in a subprocess (via `assert_cmd` + a hidden `--panic-now` debug flag compiled under `cfg(debug_assertions)`); parent process checks stdin/stdout tty state post-exit (raw mode off, cursor visible, alt screen exited) by running `stty -a` and grepping for `-icanon` absence.
 - SIGINT: send SIGINT; assert clean exit + terminal restored.
 - SIGTSTP/SIGCONT (unix only): suspend + resume; assert terminal restored on suspend and redrawn on resume.
 - `--reset-scores` prompt runs in cooked mode: test via `assert_cmd` feeding stdin `n\n` and asserting file is unchanged + no ANSI escape sequences in captured stdout.
10. **SSH responsiveness budgets** — capture renderer byte output per frame for (idle, mid-move, line-clear) fixtures; assert against the budgets in §4.
11. **Cross-platform CI** — GitHub Actions matrix: `ubuntu-latest` + `macos-latest`, stable toolchain + `1.75` MSRV check; `cargo fmt --check`; `cargo clippy -D warnings`; `cargo test`; `cargo build --release` artifact upload.
12. **Dependency hygiene** — `cargo deny check` (licenses + advisories) and a check that `Cargo.lock` is committed and unchanged by `cargo check`.

**Manual test plan (mirrors user stories).** Each user story in §2 has a one-paragraph walkthrough in `docs/manual-test.md` — run before tagging a release. Story 4 includes an explicit SSH walkthrough (`mosh`/`ssh` into a Linux box, play a game, confirm responsiveness subjectively).

## 6. Team

v0.1 is scoped to **2 engineers** (reduced from the initial 4-engineer draft — the scope does not justify four veterans, and coordination cost on a single-binary Rust project with a clean pure-core/renderer seam is low enough for two). A third QA-focused engineer is added only for sprint 3 to own the test hardening pass.

- **Veteran Rust systems + TUI engineer (1)** — owns build, CI matrix, MSRV check, dependency pinning, `terminal.rs` + signal handling, `persistence.rs` with atomic semantics, `config.rs`, CLI, `render/` module, double-wide cells, color/glyph fallback, snapshot tests. (Merges the former systems and TUI specialist roles; the ratatui/crossterm stack and terminal lifecycle are tightly coupled.)
- **Veteran game-logic engineer w/ Tetris domain knowledge (1)** — owns `game/` entirely: `srs.rs`, `bag.rs`, `rules.rs`, `board.rs`, `piece.rs`, scoring, lock-delay, level curve, top-out. This is the hardest-to-get-right role and stays dedicated.
- **Veteran QA / test engineer (1, sprint 3 only)** — property tests, SRS conformance suite, terminal-lifecycle tests, snapshot fixtures, manual test doc. The other two engineers TDD their own code; QA provides an independent hardening pass before release.

Total: 2 engineers for sprints 1–2 and 4, +1 QA for sprint 3. This keeps the parallelization benefit (pure-core vs. I/O-and-render) while matching staffing to the scope.

## 7. Implementation plan

Four sprints of ~1 week each.

### Sprint 1 — Foundation (serial handoff, then parallel)

- **Systems/TUI eng:** repo scaffold, `cargo` workspace, CI matrix (Ubuntu + macOS, stable + 1.75), `rustfmt`/`clippy`/`cargo deny` gates, `TerminalGuard` w/ ordered setup + rollback, panic hook, SIGINT/SIGTERM handler, SIGTSTP/SIGCONT handler (unix), CLI skeleton w/ `clap`, `--reset-scores` pre-TTY prompt wired. Ships a binary that opens + closes cleanly under normal exit, panic, Ctrl-C, and Ctrl-Z/fg.
- **Game-logic eng:** `board.rs` (incl. coord system + top-out cases), `piece.rs` shape tables, collision primitive, first failing SRS test (red). TDD.

Exit criteria: `cargo test` passes; binary opens/closes cleanly on both OSes under all four exit paths; empty playfield renders; top-out test cases red-to-green.

### Sprint 2 — Core mechanics (parallel)

- **Game-logic eng:** implement `srs.rs` (turn red tests green), `bag.rs` with *aligned-bag* invariant tests, `rules.rs` gravity + lock delay + scoring. All TDD.
- **Systems/TUI eng:** `persistence.rs` with `tempfile`, `sync_all`, parent-dir fsync (unix), `0o600` perms, corrupt-file recovery; `config.rs` TOML loader; `clock.rs` abstraction; wire `Renderer` to real `GameState`; ghost piece + next preview; `NO_COLOR` + glyph fallback.

Exit criteria: a human can play a full game end-to-end; all core TDD tests green; persistence tests green including failure modes.

### Sprint 3 — Polish, hardening, QA pass (parallel, +QA eng)

- **Game-logic eng:** soft-drop/hard-drop scoring, B2B multiplier, level progression wiring, top-out wiring into state machine.
- **Systems/TUI eng:** line-clear animation, pause overlay, game-over overlay w/ new-best highlight, title screen w/ top-5, DAS/ARR model, resize + too-small overlay, `--seed`/`--reset-scores`/`--config` flags.
- **QA eng:** SRS conformance golden suite; bag property test (aligned-bag + max-gap); scoring parameterized test; snapshot fixtures incl. no-color + too-small; integration headless game loop test; terminal-lifecycle tests (panic, SIGINT, SIGTSTP/SIGCONT, reset-scores in cooked mode); SSH byte-budget tests; manual test doc drafted.

Exit criteria: feature-complete for v0.1 scope; CI matrix green on both OSes; manual test doc complete; all lifecycle + persistence failure-mode tests green.

### Sprint 4 — Release (converging)

- All: bug bash against manual test plan, fix regressions, finalize README (incl. honest Guideline-inspired scoping, keybinds, install), tag v0.1.0.
- **Systems/TUI eng:** release profile tuning (`lto = "thin"`, `codegen-units = 1`), binary size check, release artifacts for macOS (arm64 + x86_64) and Linux (x86_64).
- **QA eng:** re-run snapshot suite on release build; sign off on user-story walkthroughs.

**Parallelization map.** After sprint 1, game-logic ↔ systems/TUI are independent because `GameState` is pure and `Renderer` only reads it — interface nailed down in sprint 1. QA in sprint 3 follows both by ~1 day per area.

## 8. Embedded Changelog

- **v0.2 (current)** — Tighten correctness + ops: fix 7-bag test invariant to aligned bags + max-gap (reject sliding-window false claim); specify coordinate system, block-out vs lock-out, partial-lock; expand terminal lifecycle to cover SIGINT/SIGTERM/SIGTSTP/SIGCONT/resize/backpressure/setup-rollback; sequence `--reset-scores` prompt before raw mode; pin all crate versions + add missing `toml`/`tempfile`/`proptest`/`insta`/`signal-hook`/`assert_cmd` with MSRV 1.75 CI check; harden persistence with `tempfile`, `sync_all`, parent-dir fsync, `0o600` perms, symlink behavior; downgrade claim to "Guideline-inspired" (hold + T-spins remain v0.2 work); add measurable SSH responsiveness budgets; fill in architecture diagram and ownership rules; reduce team to 2 engineers + 1 sprint-3 QA.
- **v0.1** — Initial spec. Scope: Rust + ratatui/crossterm; Guideline SRS + 7-bag + ghost + next-preview; Marathon mode; JSON high-score persistence; macOS + Linux CI. Deferred to v0.2: hold piece, T-spin detection, color themes.
