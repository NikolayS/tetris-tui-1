/// Async-signal-safe signal flags for the main loop.
///
/// Each field is an `Arc<AtomicBool>` that a signal handler sets to `true`
/// using `Ordering::SeqCst`.  All terminal I/O that must happen in response
/// to a signal is done by the main loop observing the flag on its next tick;
/// handlers never touch the terminal directly.
///
/// This design follows the SPEC §3 "app.rs ↔ signals.rs" seam: handlers
/// only do `AtomicBool::store(true, SeqCst)` — no other syscalls, no
/// allocations, no locks.
#[cfg(unix)]
pub mod unix {
    use std::io;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    use signal_hook::consts::signal::{SIGCONT, SIGINT, SIGTERM, SIGTSTP, SIGWINCH};

    /// All signal flags observed by the main loop each tick.
    #[derive(Clone)]
    pub struct Flags {
        /// SIGINT or SIGTERM: clean shutdown requested.
        pub shutdown: Arc<AtomicBool>,
        /// SIGTSTP (Ctrl-Z): restore terminal then raise SIGSTOP.
        pub tstp_pending: Arc<AtomicBool>,
        /// SIGCONT: re-enter raw/alt mode + force redraw.
        pub cont_pending: Arc<AtomicBool>,
        /// SIGWINCH: terminal resized, query new size + redraw.
        pub winch_pending: Arc<AtomicBool>,
    }

    /// Register all 5 signal handlers and return the flag set.
    ///
    /// Must be called before `TerminalGuard::enter()` so that Ctrl-C is
    /// handled even if guard setup fails partway through.
    pub fn install() -> Result<Flags, io::Error> {
        let flags = Flags {
            shutdown: Arc::new(AtomicBool::new(false)),
            tstp_pending: Arc::new(AtomicBool::new(false)),
            cont_pending: Arc::new(AtomicBool::new(false)),
            winch_pending: Arc::new(AtomicBool::new(false)),
        };

        signal_hook::flag::register(SIGINT, Arc::clone(&flags.shutdown))?;
        signal_hook::flag::register(SIGTERM, Arc::clone(&flags.shutdown))?;
        signal_hook::flag::register(SIGTSTP, Arc::clone(&flags.tstp_pending))?;
        signal_hook::flag::register(SIGCONT, Arc::clone(&flags.cont_pending))?;
        signal_hook::flag::register(SIGWINCH, Arc::clone(&flags.winch_pending))?;

        Ok(flags)
    }
}
