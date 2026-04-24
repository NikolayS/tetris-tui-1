# blocktxt

A terminal falling-block puzzle game. Modern TUI, keyboard-only,
zero dependencies beyond a single static binary.

![CI](https://github.com/NikolayS/blocktxt-1/actions/workflows/ci.yml/badge.svg)
![Release](https://img.shields.io/github/v/release/NikolayS/blocktxt-1)
![MSRV](https://img.shields.io/badge/MSRV-1.75-orange)
![License](https://img.shields.io/badge/license-Apache--2.0-blue)

![title screen](docs/qa/v011-title.gif)

> _Hero GIF will render once the QA-evidence PR merges._

A correct, lightweight, keyboard-only falling-block puzzle game that
lives in a terminal tab. Faithful Guideline-inspired mechanics (SRS
rotation with kicks, 7-bag randomizer, ghost piece, 15-reset lock
delay, Back-to-Back 4-line scoring) wrapped in a modern TUI with a
curated color palette, rounded borders, and subtle animations. Single
static binary for macOS + Linux; zero dependencies.

## Features

- **Guideline-inspired mechanics** — SRS rotation with JLSTZ + I kick
  tables, 7-bag randomizer, ghost piece, 15-move reset lock delay,
  Back-to-Back 4-line scoring at 1.5×.
- **Modern TUI** — Tokyo Night palette by default (Catppuccin Mocha
  via `--theme catppuccin-mocha`), rounded Unicode borders, solid
  `██` double-wide cells, real piece shapes in the Next queue.
- **Game feel** — spawn fade-in, line-clear flash pop, score rollup,
  Game Over zoom-in. All animations are deterministic and
  FakeClock-driven.
- **ASCII title screen** with top-5 leaderboard and reset-scores
  prompt.
- **Persistent high scores** — atomic writes with fsync + parent-dir
  fsync, 0o600 file mode, 0o700 directory, symlink + ownership
  hardening, corrupted files recovered via timestamped backups
  (capped at 5).
- **Async-signal-safe signal handling** —
  SIGINT/SIGTERM/SIGTSTP/SIGCONT/SIGWINCH observed via atomic flags;
  all terminal I/O is done from the main loop.
- **Accessibility** — `NO_COLOR` and `--no-color` give glyph-based
  mono rendering with 7 distinct piece letters.
- **DAS / ARR input** — 160 ms delayed auto-shift, 30 ms auto-repeat
  rate; kitty keyboard protocol probe with heuristic fallback for
  terminals without press/release events.
- **Minimum terminal size** — 44 × 24. A clear "too small" overlay
  renders below that.
- **Single binary** — no runtime deps.

## Install

### From a release artifact (recommended)

Download the right tarball from the
[latest release](https://github.com/NikolayS/blocktxt-1/releases/latest):

- `blocktxt-<version>-aarch64-apple-darwin.tar.gz` — macOS Apple Silicon
- `blocktxt-<version>-x86_64-apple-darwin.tar.gz` — macOS Intel
- `blocktxt-<version>-x86_64-unknown-linux-gnu.tar.gz` — Linux x86_64

Extract and drop `blocktxt` somewhere on your `$PATH`.

### From source (Rust stable, MSRV 1.75)

```bash
cargo install --locked --git https://github.com/NikolayS/blocktxt-1 --tag v0.1.1
```

### Quick dev run

```bash
git clone https://github.com/NikolayS/blocktxt-1
cd blocktxt-1
just run              # or: cargo run --release
```

## Play

```bash
blocktxt                            # default settings
blocktxt --seed 42                  # reproducible piece sequence
blocktxt --theme catppuccin-mocha   # alt palette
blocktxt --no-color                 # monochrome / NO_COLOR mode
blocktxt --reset-scores             # prompt + delete high scores
blocktxt --help
```

| Key(s)              | Action                                               |
|---------------------|------------------------------------------------------|
| `←` / `a` / `h`    | move left                                            |
| `→` / `d` / `l`    | move right                                           |
| `↓` / `s` / `j`    | soft drop                                            |
| Space               | hard drop                                            |
| `z`                 | rotate CCW                                           |
| `x`                 | rotate CW                                            |
| `p`                 | pause                                                |
| `r`                 | restart (on Game Over) · reset scores (on Title)     |
| `q` / Ctrl-C        | quit                                                 |

## Development

Uses [`just`](https://github.com/casey/just) for common tasks:

```bash
just --list           # see all recipes
just build            # cargo build
just release          # cargo build --release
just test             # cargo test
just run -- --seed 42
just ci               # local CI gate: fmt + clippy + test + deny + naming
just bench-size       # assert release binary under 8 MiB
```

Tests cover core mechanics with property tests (`proptest`), rendering
with snapshot tests (`insta`), and terminal lifecycle with PTY tests
(`rexpect` — Linux only due to macOS PTY termios semantics; see
`tests/terminal_lifecycle.rs`).

## High scores

Scores are persisted to a platform-standard config directory:

- macOS: `~/Library/Application Support/blocktxt/scores.json`
- Linux: `$XDG_DATA_HOME/blocktxt/scores.json` (defaults to
  `~/.local/share/blocktxt/scores.json`)

Scores are written atomically with a 0o600 file mode in a 0o700
directory. Corrupted score files are moved aside to
`scores.json.corrupt.<timestamp>` (capped at 5) and the game
continues with in-memory scores.

## Credits

The rotation system (SRS), 7-piece bag randomizer, and
Guideline-style scoring used in this game are derived independently
from their public mathematical specifications. Piece names
(I, O, T, S, Z, J, L) refer to their shapes.

License: Apache-2.0.
