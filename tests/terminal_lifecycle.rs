/// PTY-based terminal lifecycle tests.
///
/// These tests spawn `blocktxt` under a PTY allocated by rexpect, send
/// signals, and verify that the terminal is left in the correct mode.
///
/// The `sigtstp_restores_cooked_then_sigcont_restores_raw` test is gated
/// to Linux only because macOS PTY semantics for SIGTSTP/SIGCONT when the
/// controlling terminal is the master side behave differently: the slave
/// tty does not necessarily reflect the stopped state in a way that can be
/// reliably queried from the master side within the process-group boundary
/// that rexpect sets up.  The other three tests run on both platforms.
#[cfg(unix)]
mod tests {
    use rexpect::process::PtyProcess;
    use std::process::Command;
    use std::time::Duration;

    // Locate the debug binary built by `cargo test`.
    fn binary_path() -> std::path::PathBuf {
        // CARGO_BIN_EXE_blocktxt is set by cargo test for integration tests.
        // Fall back to a relative path for manual runs.
        if let Ok(p) = std::env::var("CARGO_BIN_EXE_blocktxt") {
            return p.into();
        }
        let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("target/debug/blocktxt");
        p
    }

    fn spawn_blocktxt(args: &[&str]) -> PtyProcess {
        let mut cmd = Command::new(binary_path());
        for arg in args {
            cmd.arg(arg);
        }
        PtyProcess::new(cmd).expect("failed to spawn blocktxt PTY process")
    }

    /// Helper: check that a PTY slave fd is in raw mode by querying termios
    /// via the master side. Returns true when ICANON is NOT set (raw mode).
    fn is_raw(proc: &PtyProcess) -> bool {
        use nix::sys::termios::{tcgetattr, LocalFlags};
        use std::os::fd::BorrowedFd;
        use std::os::unix::io::AsRawFd;
        // Safety: proc.pty is live for the duration of this call.
        let bfd = unsafe { BorrowedFd::borrow_raw(proc.pty.as_raw_fd()) };
        let flags = tcgetattr(bfd).expect("tcgetattr failed on PTY master");
        !flags.local_flags.contains(LocalFlags::ICANON)
    }

    fn sleep_ms(ms: u64) {
        std::thread::sleep(Duration::from_millis(ms));
    }

    /// After the binary starts and enters the TUI, the PTY should be in raw mode.
    #[test]
    fn termios_raw_after_enter() {
        let proc = spawn_blocktxt(&[]);
        // Give the binary time to enter raw mode (TerminalGuard::enter).
        sleep_ms(300);
        assert!(is_raw(&proc), "terminal should be raw after enter");
    }

    /// After sending SIGINT the binary should exit and the PTY should be
    /// back in cooked (non-raw) mode.
    #[test]
    fn termios_restored_after_sigint() {
        use nix::sys::signal::{kill, Signal};

        let proc = spawn_blocktxt(&[]);
        sleep_ms(300);
        assert!(is_raw(&proc), "should be raw before signal");

        kill(proc.child_pid, Signal::SIGINT).expect("kill SIGINT failed");
        sleep_ms(400);

        // After exit the terminal should be cooked (ICANON restored).
        assert!(
            !is_raw(&proc),
            "terminal should be cooked after SIGINT exit"
        );
    }

    /// Send SIGTSTP → PTY should go cooked; send SIGCONT → PTY should go raw
    /// again.  Linux only — see module doc comment.
    #[cfg(target_os = "linux")]
    #[test]
    fn sigtstp_restores_cooked_then_sigcont_restores_raw() {
        use nix::sys::signal::{kill, Signal};

        let proc = spawn_blocktxt(&[]);
        sleep_ms(300);
        assert!(is_raw(&proc), "should be raw before SIGTSTP");

        kill(proc.child_pid, Signal::SIGTSTP).expect("kill SIGTSTP failed");
        sleep_ms(400);
        assert!(!is_raw(&proc), "should be cooked after SIGTSTP");

        kill(proc.child_pid, Signal::SIGCONT).expect("kill SIGCONT failed");
        sleep_ms(400);
        assert!(is_raw(&proc), "should be raw again after SIGCONT");
    }

    /// `--crash-for-test` causes a panic after TerminalGuard::enter(); the
    /// panic hook must restore the terminal before the process dies.
    #[test]
    fn panic_restores_terminal() {
        let proc = spawn_blocktxt(&["--crash-for-test"]);
        // Allow time for guard entry and the panic to fire.
        sleep_ms(500);
        // Process should have exited.
        assert!(
            proc.status().is_some(),
            "process should have exited after panic"
        );
        // Terminal should be cooked.
        assert!(!is_raw(&proc), "terminal should be cooked after panic exit");
    }
}
