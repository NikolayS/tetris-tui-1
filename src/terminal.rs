/// RAII terminal lifecycle guard.
///
/// `TerminalGuard::enter()` enables raw mode, enters the alternate screen,
/// and hides the cursor — in that order.  On any partial failure the steps
/// already taken are reversed before the error is returned.
///
/// `Drop` unconditionally restores the terminal (idempotent — calling it
/// multiple times is safe).
///
/// A panic hook is installed *before* `enter()` is called (from `main`).
/// The hook calls the same byte-level restore routine via a `static`
/// `OnceLock`; it does not touch the guard's state, so the guard's own
/// `Drop` running afterwards is a harmless no-op.
use std::io::{self, Write as _};
use std::sync::OnceLock;

use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::{cursor, execute, style};
use thiserror::Error;

/// Errors that can occur during terminal setup.
#[derive(Debug, Error)]
pub enum TerminalError {
    #[error("failed to enable raw mode: {0}")]
    RawMode(io::Error),
    #[error("failed to enter alternate screen: {0}")]
    AlternateScreen(io::Error),
    #[error("failed to hide cursor: {0}")]
    HideCursor(io::Error),
}

/// Async-signal-safe restore routine: writes known ANSI sequences to
/// stderr's raw fd.  Called from both `Drop` and the panic hook.
///
/// The function writes directly to fd 2 to avoid buffered-writer state
/// that may be corrupted during a panic.
fn restore_raw() {
    // Byte sequences (order matters):
    //   1. show cursor      ESC[?25h
    //   2. reset attributes ESC[0m
    //   3. leave alt screen ESC[?1049l
    //   4. disable raw mode (crossterm syscall — not a write)
    //
    // Steps 1-3 are written as a single raw write so that even if the
    // crossterm API isn't available (e.g. in a signal context) the
    // sequences still reach the terminal.
    const RESET_SEQS: &[u8] = b"\x1b[?25h\x1b[0m\x1b[?1049l";

    // Write to stderr fd directly.
    #[cfg(unix)]
    {
        use std::os::unix::io::FromRawFd;
        // Safety: fd 2 is always open.
        let mut stderr = unsafe { std::fs::File::from_raw_fd(2) };
        let _ = stderr.write_all(RESET_SEQS);
        let _ = stderr.flush();
        // Do NOT drop (close) fd 2.
        std::mem::forget(stderr);
    }
    #[cfg(not(unix))]
    {
        let _ = io::stderr().write_all(RESET_SEQS);
    }

    // Best-effort crossterm cleanup (no-op if already disabled).
    let _ = disable_raw_mode();
}

/// Function pointer type stored in the static.
type RestoreFn = fn();

/// Shared restore function installed once at startup.
/// The panic hook reads this to call `restore_raw()` without holding a
/// reference to the guard.
static RESTORE_FN: OnceLock<RestoreFn> = OnceLock::new();

/// Install the panic hook that restores the terminal.
///
/// Must be called once before `TerminalGuard::enter()`.  Subsequent calls
/// are no-ops (the `OnceLock` is already set).
pub fn install_panic_hook() {
    RESTORE_FN.get_or_init(|| restore_raw as RestoreFn);

    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if let Some(f) = RESTORE_FN.get() {
            f();
        }
        prev(info);
    }));
}

/// RAII guard that owns the raw-mode + alternate-screen state.
///
/// Construction is via `TerminalGuard::enter()`; destruction (including
/// on panic) is via `Drop`.
pub struct TerminalGuard {
    /// Tracks whether we have successfully enabled raw mode so that `Drop`
    /// knows whether to call `disable_raw_mode`.
    raw_enabled: bool,
    /// Tracks whether we entered the alternate screen.
    alt_screen: bool,
    /// Tracks whether we hid the cursor.
    cursor_hidden: bool,
}

impl TerminalGuard {
    /// Enter the TUI environment: raw mode → alternate screen → hide cursor.
    ///
    /// On partial failure the completed steps are reversed before returning
    /// the error so the terminal is always left in a usable state.
    pub fn enter() -> Result<Self, TerminalError> {
        let mut guard = TerminalGuard {
            raw_enabled: false,
            alt_screen: false,
            cursor_hidden: false,
        };

        // Step 1: raw mode.
        enable_raw_mode().map_err(|e| {
            // Nothing to roll back yet.
            TerminalError::RawMode(e)
        })?;
        guard.raw_enabled = true;

        // Step 2: alternate screen.
        execute!(io::stdout(), EnterAlternateScreen).map_err(|e| {
            // Roll back raw mode.
            let _ = disable_raw_mode();
            TerminalError::AlternateScreen(e)
        })?;
        guard.alt_screen = true;

        // Step 3: hide cursor.
        execute!(io::stdout(), cursor::Hide).map_err(|e| {
            // Roll back alternate screen + raw mode.
            let _ = execute!(io::stdout(), LeaveAlternateScreen);
            let _ = disable_raw_mode();
            TerminalError::HideCursor(e)
        })?;
        guard.cursor_hidden = true;

        Ok(guard)
    }

    /// Re-enter raw mode and the alternate screen after a SIGTSTP/SIGCONT
    /// round-trip.  Idempotent — safe to call if already in the TUI.
    pub fn re_enter(&mut self) -> io::Result<()> {
        if !self.raw_enabled {
            enable_raw_mode()?;
            self.raw_enabled = true;
        }
        if !self.alt_screen {
            execute!(io::stdout(), EnterAlternateScreen)?;
            self.alt_screen = true;
        }
        if !self.cursor_hidden {
            execute!(io::stdout(), cursor::Hide)?;
            self.cursor_hidden = true;
        }
        Ok(())
    }

    /// Restore the terminal to its prior state without dropping the guard.
    ///
    /// Used by the SIGTSTP handler: restore terminal, raise SIGSTOP; on
    /// SIGCONT call `re_enter` to come back.
    pub fn restore(&mut self) {
        if self.cursor_hidden {
            let _ = execute!(io::stdout(), cursor::Show);
            self.cursor_hidden = false;
        }
        if self.alt_screen {
            let _ = execute!(io::stdout(), style::ResetColor, LeaveAlternateScreen);
            self.alt_screen = false;
        }
        if self.raw_enabled {
            let _ = disable_raw_mode();
            self.raw_enabled = false;
        }
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        self.restore();
    }
}
