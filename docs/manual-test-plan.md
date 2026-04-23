# Manual test plan — blocktxt v0.1.0

Run through this checklist against a release build on each supported
platform (macOS arm64, macOS x86_64, Linux x86_64). Tick as you go;
any broken item blocks release.

## Launch

- [ ] `blocktxt --version` prints `blocktxt <version>`.
- [ ] `blocktxt --help` prints help; no prohibited trademark term
      anywhere.
- [ ] `blocktxt` launches into a playable board in iTerm2, Ghostty,
      and Alacritty on macOS; `xterm` / `gnome-terminal` on Linux.
- [ ] Under 200 ms from launch to playable board.

## Core gameplay

- [ ] Arrow keys move left/right; piece falls; hard drop locks on Space.
- [ ] Z / X rotate correctly including near walls (SRS kicks).
- [ ] Soft drop accelerates piece while held.
- [ ] Line clears show flash → dim → collapse (~200 ms total budget).
- [ ] Scoring: single=100×L, double=300×L, triple=500×L, quad=800×L.
- [ ] Back-to-Back quads score ×1.5 (=1200×L at level 1).
- [ ] Non-quad clear breaks B2B; empty lock does NOT break B2B.
- [ ] Level bumps every 10 lines.
- [ ] Ghost piece shows correct landing position.
- [ ] Next-piece preview shows upcoming pieces.

## Signals + terminal lifecycle

- [ ] Ctrl-C exits cleanly; cursor + cooked mode restored.
- [ ] Ctrl-Z suspends; `fg` resumes; display is correct after resume.
- [ ] Terminal resize: smaller than 44×24 shows "too small" overlay;
      resize back restores gameplay.
- [ ] Panic path (via `--crash-for-test`): terminal is cooked after
      child dies.

## Persistence

- [ ] First game: score is saved; next launch shows it.
- [ ] New personal best: "NEW BEST!" banner on Game Over.
- [ ] `blocktxt --reset-scores` with `y` deletes file; `n` preserves.
- [ ] Corrupt score file: game launches, prints stderr warning, and
      plays with in-memory score.

## Accessibility

- [ ] `NO_COLOR=1 blocktxt` renders in monochrome with distinct glyphs.
- [ ] `blocktxt --no-color` does the same without the env var.
- [ ] Glyphs are distinguishable in Terminal.app's default (dim) color.

## Performance

- [ ] No visible flicker at level 1.
- [ ] SSH session over 30–50 ms RTT: input feels responsive.
- [ ] Binary size < 8 MiB.

## Naming-check

- [ ] `bash scripts/check-naming.sh` exits 0 against the release tarball.
