# Changelog

All notable changes to this project will be documented in this file.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

---

## [0.2.0] — 2026-04-24

**Originality pass for trade-dress safety.** *Tetris Holding LLC v.
Xio Interactive* (2012) named seven specific elements of falling-block
games as protected look-and-feel: **playfield dimensions, piece-color
associations, cell glyph style, line-clear animation, next-piece
preview, ghost-piece style, and locked-piece color persistence.** v0.2.0
makes blocktxt's implementation of EACH of those seven distinct from
the protected commercial implementation, while preserving SRS / 7-bag /
Guideline-style scoring (which are mathematical and not protectable).

This is a **breaking visual change**. High-score files load forward,
but scores set on the v0.1.x 10×20 playfield are not directly
comparable to v0.2.x 12×24 scores.

### Changed (originality)

- **Playfield: 10×20 → 12×24** (#63). Wider + taller; spawn positions
  recentered: O at cols 5+6, JLSTZ/I 4-wide bbox at cols 4..8.
  `MIN_WIDTH` 44 → 52; `MIN_HEIGHT` 24 → 28.
- **Piece-color permutation** (#63). Non-Guideline assignments,
  applied identically across all 5 palettes via `PaletteSlot`
  indirection: I=orange, O=pink, T=green, S=blue, Z=yellow,
  J=purple, L=cyan. (Guideline canon: I=cyan, O=yellow, T=purple,
  S=green, Z=red, J=blue, L=orange.)
- **Cell glyph: `██` → `▰▰`** (#63). Black Parallelogram (U+25B0)
  reads as a deliberate dingbat rather than a generic block. Flash
  glyph during line-clear: `▣▣` (White Square Containing Black Small
  Square, U+25A3).
- **Line-clear animation: WipeOutward** (#63). Was Flash + Dim +
  Collapse; now Flash + WipeOutward (200 ms total, center cells clear
  first then expand outward).
- **Next preview: single letter, not miniature board** (#63). Was a
  5-piece queue rendered as full piece shapes; now a SINGLE next
  piece shown as its uppercase letter (`I`/`O`/`T`/`S`/`Z`/`J`/`L`)
  centered in the next box, colored per piece accent.
- **Ghost piece: floor line, not piece outline** (#63). Was the piece
  shape rendered with `░░` glyphs at the landing position; now a
  single horizontal line (`▔▔` per landing column) at 40% intensity.
  Suppressed when the piece is already at the floor (would overlap
  its own body).
- **Locked-piece color: dimmed** (#63). Was full piece accent (same
  as active); now `dim_color(accent, 0.6)`. Active piece pops above
  the muted stack.

### Tests

- 223 tests total (up from 214 in v0.1.2). 7 new originality tests
  pinning each protected-element distinction (`piece_color_mapping_consistent_across_palettes`,
  `next_preview_shows_only_one_piece`, `ghost_renders_as_floor_line`,
  `ghost_does_not_render_when_piece_is_already_on_floor`,
  `locked_pieces_render_dimmer_than_active`,
  `snapshot_line_clear_wipe_midframe`, etc.). All 17 existing render
  snapshots intentionally regenerated for the new visual baseline.

### Trade-offs

- The single-piece next preview reduces multi-piece-lookahead
  strategic depth versus the v0.1.x 5-piece queue. Skilled players
  used the queue to plan 4-line clears 3–5 pieces ahead.
- The 12-wide playfield makes I-piece 4-line clears slightly easier
  (2 extra columns of lateral planning room).
- Both trade-offs are deliberate per the trade-dress directive.

### Documentation

- SPEC §1a "Branding" extended to document each of the 7 distinctions
  with rationale + case citation.
- README "Originality" section added after Credits.

### Known limitations (unchanged)

- macOS `SIGTSTP`/`SIGCONT` round-trip untested via PTY (Linux-only
  lifecycle tests).
- Windows: WSL only.

---

## [0.1.2] — 2026-04-24

Gameplay + visual expansion. Two SPEC §1a v0.2-deferred items land:
hold piece and more theme choices.

### Added

- **Hold piece (`c` key)** (#62). Guideline-style hold: swap the
  active piece into a hold slot (or draw from bag on first hold);
  subsequent holds within a cycle swap active ↔ hold. Locked once
  per piece cycle (prevents infinite-hold stalling); unlocks on
  lock_piece, so hard-drop resets cleanly. Hold resets lock-delay
  on the new piece. Spawn animation fires on the held piece. Hold
  box renders above the next-queue in the HUD; `Modifier::DIM` when
  locked for the cycle. Block-out transition if swapping would
  spawn into occupied cells. Hold no-op in Title / Paused /
  GameOver / during line-clear animation.
- **Three new color palettes** (#61): **Gruvbox Dark** (morhetz
  canonical hex), **Nord** (nordtheme.com reference), **Dracula**
  (draculatheme.com reference). All 5 palettes now selectable via
  `--theme`:
  - `tokyo-night` / `tn` (default)
  - `catppuccin-mocha` / `catppuccin` / `cm`
  - `gruvbox-dark` / `gruvbox` / `gv`
  - `nord` / `nord-dark`
  - `dracula` / `dr`

### Tests

- 214 tests total (up from 193 in v0.1.1). 8 new hold-piece tests,
  7 new palette-parsing + RGB-spec tests, 6 new render snapshots
  (3 hold-box variants + 3 per-palette board_view baselines).

### Known limitations (unchanged)

- macOS SIGTSTP/SIGCONT round-trip untested via PTY — Linux-only
  lifecycle tests; the flag-based implementation is exercised on
  both platforms at compile + unit-test level.
- Windows: WSL only.

---

## [0.1.1] — 2026-04-24

Polish release. No behavior-breaking changes to gameplay; all the
mechanics from v0.1.0 are unchanged. Visual + UX level-up so the
game actually looks like a modern TUI product.

### Added

- **Title screen** (#57). ASCII-art `BLOCKTXT` logo, tagline, top-5
  leaderboard, controls cheat-sheet, and a "press any key to start"
  footer. New `Phase::Title` is the initial state; `Input::StartGame`
  on any action key transitions to Playing. New `Phase::ConfirmResetScores`
  reachable by `r` on Title — Y clears the high-score store in place
  and persists; N cancels. Game Over `r` now returns to Title (not
  straight into a new game).
- **`--theme` CLI flag** (#56). Tokyo Night is the new default palette
  (higher saturation + brighter text than the v0.1.0 Catppuccin
  Mocha). Catppuccin Mocha still available via
  `--theme catppuccin-mocha`. Aliases: `tokyo-night` / `tn`,
  `catppuccin-mocha` / `catppuccin` / `cm`. `NO_COLOR` still wins over
  any palette.
- **Juice animations** (#54). Piece spawn fade-in (80 ms at 60 / 80 /
  100% intensity). Line-clear flash is now a bright
  `Rgb(255, 255, 255) + BOLD` pop over the prior dim phase. Score
  rollup interpolates over 250 ms so big swings read visually
  instead of snapping. Game Over overlay zooms in from 0.5× to 1.0×
  over 200 ms.

### Changed

- **Release CI matrix drops `macos-13`** (#55, closes #46).
  `macos-latest` (ARM) cross-compiles both `aarch64-apple-darwin` and
  `x86_64-apple-darwin` via `rustup target add`; `macos-13`'s
  upcoming deprecation is moot.
- **`scripts/check-naming.sh` now scans `docs/`** (#55, closes #47)
  with binary-file skipping + text-extension allowlist so committed
  GIFs/PNGs don't false-positive.
- **`scripts/bump-version.sh` is idempotent** (#55, closes #48) —
  re-runs at the same version exit 0 with "Already at X; no-op."

### Docs

- **README rewrite** (#58). Hero GIF, badges (CI / release / MSRV /
  license), 9-feature pitch, 3 install paths (release artifact,
  `cargo install --git`, `just run`), keybind table, Development
  section pointing at the Justfile.
- **QA evidence** (#59, closes #50). `docs/qa/v011-title-native.png` +
  `v011-gameplay-native.png` native macOS Terminal screenshots
  captured via AppleScript + `screencapture -R`; `v011-title.gif` +
  `v011-gameplay.gif` asciinema + agg for animated context;
  `v011-summary.md` before/after matrix.

### Tests

- 193 tests across the suite (up from 147 in v0.1.0). New coverage
  for the title state machine, animations under FakeClock, palette
  selection via `Palette` enum + `--theme` flag, and
  bump-version idempotency.

### Known limitations (unchanged from v0.1.0)

- macOS `SIGTSTP`/`SIGCONT` termios round-trip remains untested via
  PTY — Linux-only lifecycle tests; the flag-based implementation is
  exercised on both platforms at compile + unit-test level.
- Windows support is still WSL only.

---

## [0.1.0] — 2026-04-23

First public release. A single-player terminal falling-block puzzle
game. Single static binary for macOS (Apple Silicon + Intel) and Linux
(x86_64). Zero deps beyond what ships in the binary. No network, no
sound, no GUI.

### Added

- **Core gameplay**: SRS rotation with JLSTZ + I kick tables (derived
  from public mathematical spec), 7-piece bag randomizer with correct
  aligned-window invariant and max-gap ≤ 12, ghost piece, next-piece
  preview (up to 5), Guideline-style scoring
  (100/300/500/800 × level), Back-to-Back 4-line clears at ×1.5,
  gravity curve to level 20, lock delay with 500 ms timer + 15-move
  reset cap, soft-drop cap (`max(natural/20, 30ms/cell)`), hard-drop
  bypass, top-out (block-out + lock-out) state machine, pause, restart
  from Game Over.
- **Input**: arrow keys / WASD / vim-style. DAS 160 ms / ARR 30 ms
  with kitty keyboard protocol probe + 160 ms release-inference
  fallback for terminals without press/release events.
- **Rendering**: `ratatui` + `crossterm` TUI at an 8 ms poll /
  16 ms frame cadence; line-clear animation (flash / dim / collapse
  over 200 ms); pause overlay; Game Over overlay with **NEW BEST!**
  highlight when the score beats the stored top score.
- **Persistence**: top-5 high-score store at the platform-standard
  config directory, atomic write via `tempfile::NamedTempFile::persist`
  with fsync + parent-dir fsync + 0o600 file mode in a 0o700 directory
  (symlink-refusing, world-writable-refusing, ownership-verified).
  Corrupted score files are moved aside to
  `scores.json.corrupt.<unix_secs>` (cap 5), and the game continues
  with in-memory scores.
- **CLI flags**: `--seed <u64>` for reproducible piece sequences,
  `--no-color` for monochrome/glyph fallback (supplements `NO_COLOR`
  env), `--reset-scores` to prompt (in cooked mode) and delete the
  high-score file, `--version`, `--help`.
- **Accessibility**: `NO_COLOR`-aware theme with distinct glyphs for
  16-color and monochrome terminals; minimum terminal size 44 × 24
  with a "too small" overlay while smaller.
- **Terminal lifecycle**: `TerminalGuard` with ordered setup +
  idempotent Drop-based restore; panic hook writes ANSI reset bytes
  directly to fd 2 (async-signal-safe). Flag-based signal handlers
  (via `signal-hook`) for SIGINT/SIGTERM/SIGTSTP/SIGCONT/SIGWINCH —
  handlers only `AtomicBool::store`; the main loop performs all
  terminal I/O per SPEC §4 async-signal-safe design.
- **Docs**: README with install/play/flags/high-scores/accessibility,
  trademark-free; `docs/manual-test-plan.md` — 26-item checklist
  across 7 sections for release bug bash.
- **Release workflow**: `.github/workflows/release.yml` matrix builds
  for `x86_64-unknown-linux-gnu` / `aarch64-apple-darwin` /
  `x86_64-apple-darwin` on tag; 8 MiB binary-size guard; tarballs
  uploaded to the GitHub release.

### Changed

- **Runtime RNG is `StdRng` (ChaCha12)**, not `ChaCha8`. `rand_chacha`
  is a dev-dep only; runtime code uses `rand::rngs::StdRng`. Seeds
  produce different bag sequences than prior dev versions — no prior
  public release carried the earlier RNG.

### Tests

- 147 tests across the suite: unit + property-based (`proptest`) for
  SRS rotation conformance, 7-bag invariant, scoring/level progression,
  lock-delay state transitions; snapshot tests via `insta` for HUD /
  board / ghost / pause / Game Over / line-clear / too-small overlay
  variants; PTY-based lifecycle tests via `rexpect` (Linux-only where
  macOS PTY master/slave termios semantics diverge); 5000-tick seeded
  e2e stress test with no-panic + state-consistency assertions each
  tick; DAS/ARR boundary sweep 0–500 ms; persistence failure-mode
  suite (symlink, ownership, world-writable parent, corrupt +
  timestamped backups capped at 5).

### Known limitations

- macOS `SIGTSTP`/`SIGCONT` termios round-trip is not tested via PTY
  (reliable cross-platform PTY termios observation is a research
  project); the flag-based handler implementation is exercised on
  Linux CI.
- Windows support is via WSL only; no native Win32 target in this
  release.
