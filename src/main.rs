mod cli;
mod signals;
mod terminal;

use std::sync::atomic::Ordering;
use std::time::Duration;

use crossterm::event;
use terminal::{install_panic_hook, TerminalGuard};

fn main() -> anyhow::Result<()> {
    // Step 1: Parse CLI args (clap derive).
    let args = cli::parse();

    // Step 2: Handle --reset-scores in cooked mode before any raw mode.
    if args.reset_scores {
        cli::handle_reset_scores(&args)?;
        return Ok(());
    }

    // Step 3: Install signal flags BEFORE guard enter so Ctrl-C works if
    //         guard setup fails partway through.
    // Step 4: Install the panic hook BEFORE guard setup so the terminal is
    //         restored even if setup panics.
    install_panic_hook();

    #[cfg(unix)]
    let flags = signals::unix::install()?;

    // Step 5: Enter TUI (raw mode + alternate screen + hide cursor).
    let mut guard = TerminalGuard::enter()?;

    // Step 6: --crash-for-test panics after guard entry (used by PTY tests).
    //         Replaces the ad-hoc std::env::args() check from PR #8.
    if args.crash_for_test {
        panic!("--crash-for-test: intentional panic after guard entry");
    }

    // Step 7: Run loop (Sprint 1 stub; real game in Sprint 2).
    run_loop(&mut guard, &flags)?;

    Ok(())
}

/// Main event loop skeleton (Sprint 1 stub).
///
/// Observes signal flags each tick (ordered per SPEC §3 data-flow):
///
///   shutdown → exit clean
///   tstp_pending → restore terminal, raise SIGSTOP; on SIGCONT re-enter
///   cont_pending → re-enter raw/alt mode, force redraw
///   winch_pending → (Sprint 2) query size, redraw
///
/// The loop body is a stub for Sprint 2: polls crossterm events with an
/// 8 ms timeout per SPEC §4 frame cadence.
#[cfg(unix)]
fn run_loop(guard: &mut TerminalGuard, flags: &signals::unix::Flags) -> anyhow::Result<()> {
    use nix::sys::signal::{self, Signal};

    loop {
        // --- 1. Check signal flags (ordered) ---

        if flags.shutdown.swap(false, Ordering::Relaxed) {
            break;
        }

        if flags.tstp_pending.swap(false, Ordering::Relaxed) {
            // Restore terminal before stopping so the shell gets a sane tty.
            guard.restore();
            // Raise SIGSTOP to actually pause the process. The kernel will
            // deliver SIGCONT later when the user does `fg`.
            let _ = signal::raise(Signal::SIGSTOP);
            // When we wake from SIGSTOP (SIGCONT delivered), re-enter TUI.
            guard.re_enter()?;
            flags.cont_pending.store(false, Ordering::Relaxed);
            flags.winch_pending.store(true, Ordering::Relaxed);
            continue;
        }

        if flags.cont_pending.swap(false, Ordering::Relaxed) {
            guard.re_enter()?;
            flags.winch_pending.store(true, Ordering::Relaxed);
        }

        if flags.winch_pending.swap(false, Ordering::Relaxed) {
            // Sprint 2 will query terminal size and redraw here.
        }

        // --- 2. Poll for crossterm events (8 ms timeout) ---
        if event::poll(Duration::from_millis(8))? {
            let ev = event::read()?;
            // Sprint 2 will translate events → Inputs here.
            // For now exit cleanly on 'q'.
            if let event::Event::Key(key) = ev {
                use crossterm::event::{KeyCode, KeyEvent};
                if let KeyEvent {
                    code: KeyCode::Char('q'),
                    ..
                } = key
                {
                    break;
                }
            }
        }
    }

    Ok(())
}

// Non-unix stub so the crate still compiles on Windows (WSL takes the unix path).
#[cfg(not(unix))]
fn run_loop(_guard: &mut TerminalGuard, _flags: &()) -> anyhow::Result<()> {
    Ok(())
}
