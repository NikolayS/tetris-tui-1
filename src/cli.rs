/// CLI argument definitions and pre-TTY handlers.
///
/// `Args` is parsed by clap (derive) before any terminal manipulation.
/// Flags that must run before `TerminalGuard::enter()` (e.g. `--reset-scores`)
/// are handled here in cooked mode so that no raw-mode / alternate-screen
/// bytes leak into stdout.
use std::io::{self, BufRead, Write};

use clap::Parser;

/// blocktxt — terminal falling-block puzzle game.
///
/// A falling-block puzzle game for macOS and Linux.
/// Single-player, keyboard-controlled, no network, no sound.
/// See <https://github.com/NikolayS/blocktxt-1> for details.
#[derive(Parser, Debug)]
#[command(
    name = "blocktxt",
    version,
    about = "A terminal falling-block puzzle game.",
    long_about = "blocktxt — a terminal falling-block puzzle game. \
                  Single-player, keyboard-controlled, no network, no sound. \
                  See https://github.com/NikolayS/blocktxt-1 for details."
)]
pub struct Args {
    /// Seed the ChaCha RNG for reproducible piece sequences.
    ///
    /// Accepts any u64 value. Used by Sprint 2 piece-bag generation.
    #[arg(long)]
    pub seed: Option<u64>,

    /// Force-disable color output (supplements the NO_COLOR env variable).
    #[arg(long)]
    pub no_color: bool,

    /// Prompt in cooked mode to delete the high-score file, then exit.
    ///
    /// Runs before raw mode / alternate screen so no ANSI escape bytes
    /// appear in stdout.
    #[arg(long)]
    pub reset_scores: bool,

    /// Color palette selection.
    #[arg(
        long,
        default_value = "tokyo-night",
        help = "Color palette: tokyo-night|catppuccin-mocha|gruvbox-dark|nord|dracula"
    )]
    pub theme: String,

    /// Trigger a panic after TerminalGuard::enter() for PTY signal tests.
    ///
    /// Hidden from help output. Replaces the ad-hoc args() check from PR #8.
    #[arg(long, hide = true)]
    pub crash_for_test: bool,
}

/// Parse command-line arguments.
pub fn parse() -> Args {
    Args::parse()
}

/// Handle `--reset-scores` in cooked mode before the TUI guard is entered.
///
/// Prompts on stdout/stdin (no ANSI sequences). If the user answers `y`
/// or `yes` (case-insensitive), deletes the high-score file and any
/// `.corrupt.*` siblings. Exits before `TerminalGuard::enter()`.
///
/// # Cooked-mode contract (SPEC §1a / §5)
///
/// This function must not write any ANSI escape bytes (`\x1b`) to stdout.
/// Tests assert this by scanning captured stdout bytes.
pub fn handle_reset_scores(_args: &Args) -> anyhow::Result<()> {
    use blocktxt::persistence;

    let stdout = io::stdout();
    let mut out = stdout.lock();

    // Resolve the data directory — failure is not fatal here.
    let dir = match persistence::init_data_dir() {
        Ok(d) => d,
        Err(_) => {
            writeln!(out, "No data directory found; nothing to reset.")?;
            return Ok(());
        }
    };

    // Write prompt to stdout — plain text only, no ANSI sequences.
    write!(out, "Reset high scores? [y/N] ")?;
    out.flush()?;

    // Read one line from stdin in cooked mode.
    let stdin = io::stdin();
    let mut line = String::new();
    stdin.lock().read_line(&mut line)?;

    let answer = line.trim().to_lowercase();
    if answer == "y" || answer == "yes" {
        let scores_file = persistence::scores_path(&dir);

        // Delete main scores file if it exists.
        if scores_file.exists() {
            std::fs::remove_file(&scores_file).ok();
        }

        // Delete any .corrupt.* siblings.
        let file_name = scores_file
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("scores.json");
        let prefix = format!("{file_name}.corrupt.");
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with(&prefix) {
                        std::fs::remove_file(&p).ok();
                    }
                }
            }
        }

        writeln!(out, "High scores reset.")?;
    } else {
        writeln!(out, "Cancelled.")?;
    }

    Ok(())
}
