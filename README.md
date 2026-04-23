# blocktxt

A terminal falling-block puzzle game. Single-player, keyboard-controlled,
no network, no sound. Single static binary for macOS and Linux.

Zero setup. Launch it, play it, quit it — all without leaving your terminal.

## Install

### From a release artifact

Download the binary for your platform from
[Releases](https://github.com/NikolayS/blocktxt-1/releases/latest):

- `blocktxt-<version>-aarch64-apple-darwin.tar.gz` — macOS Apple Silicon
- `blocktxt-<version>-x86_64-apple-darwin.tar.gz` — macOS Intel
- `blocktxt-<version>-x86_64-unknown-linux-gnu.tar.gz` — Linux x86_64

Unpack, move `blocktxt` somewhere on your `$PATH`, and run `blocktxt`.

### From source

Requires Rust stable (MSRV 1.75).

```bash
cargo install --locked --git https://github.com/NikolayS/blocktxt-1
```

## Play

Run `blocktxt`. Keybinds:

| Key(s)         | Action                    |
|----------------|---------------------------|
| ← / `a` / `h` | move left                 |
| → / `d` / `l` | move right                |
| ↓ / `s` / `j` | soft drop (hold)          |
| Space          | hard drop                 |
| `z`            | rotate CCW                |
| `x`            | rotate CW                 |
| `p`            | pause                     |
| `r`            | restart (from Game Over)  |
| `q` / Ctrl-C   | quit                      |

## Flags

- `--seed <u64>` — reproducible piece sequence (any 64-bit integer).
- `--no-color` — force monochrome + glyph rendering. Also honored via
  the `NO_COLOR` env var.
- `--reset-scores` — prompt (in cooked mode) to delete the high-score
  file, then exit.

## High scores

Scores are persisted to a platform-standard config directory:

- macOS: `~/Library/Application Support/blocktxt/scores.json`
- Linux: `$XDG_DATA_HOME/blocktxt/scores.json` (defaults to
  `~/.local/share/blocktxt/scores.json`)

Scores are written atomically with a 0o600 file mode in a 0o700
directory. Corrupted score files are moved aside to
`scores.json.corrupt.<timestamp>` (capped at 5) and the game continues.

## Minimum terminal size

44 × 24 cells. A "terminal too small" overlay is drawn if smaller; the
game resumes when the terminal is resized.

## Accessibility

`NO_COLOR` env var (or `--no-color` flag) falls back to a glyph-based
piece palette that remains distinct on 16-color and monochrome terminals.

## What's in v0.1 / what's coming

v0.1 is Guideline-inspired, not fully Guideline-compliant. It ships:
SRS rotation with wall kicks, 7-bag randomizer, ghost piece, next-piece
preview, Guideline-style scoring for singles/doubles/triples/4-line clears,
and Back-to-Back 4-line-clear bonus.

Not in v0.1 (planned for v0.2): hold piece, T-spin detection and scoring,
combo counter, configurable timings.

## Credits

The rotation system (SRS), 7-piece bag randomizer, and Guideline-style
scoring used here are derived independently from their public mathematical
specifications. Piece names (I, O, T, S, Z, J, L) refer to their shapes.

## License

Apache-2.0.
