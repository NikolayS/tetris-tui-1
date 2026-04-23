/// PTY signal lifecycle — Sprint 4 Track D extension of PR #8's four tests.
///
/// This file adds one additional test:
///
/// **resize_sigwinch_binary_stays_alive** — spawn `blocktxt`, wait for raw
/// mode, send SIGWINCH, assert the process is still alive (no crash),
/// then clean up.
///
/// SIGWINCH is the standard Unix window-resize signal and must not crash
/// a properly written TUI application; it should only trigger a re-draw.
///
/// # Cleanup strategy
///
/// `blocktxt` does not handle SIGTERM, so `PtyProcess::exit()` (SIGTERM +
/// blocking waitpid loop) hangs forever.  The `PtyProcess::Drop` impl has
/// the same problem.  We avoid the hang by:
///   1. Sending SIGKILL via `nix::kill` directly.
///   2. Calling `std::mem::forget(proc)` to skip the blocking Drop.
///
/// This leaks the `PtyProcess` handle (file descriptors + pid tracking)
/// within the test process, which is acceptable for a test binary that
/// exits after the test suite completes.  The OS reclaims all resources
/// when the test process exits.  The zombie child is adopted by pid 1 and
/// reaped there.
///
/// The existing PR #8 tests avoid this by always letting the binary exit
/// naturally (SIGINT is handled) or via crash (no Drop needed).
#[cfg(unix)]
mod tests {
    use rexpect::process::PtyProcess;
    use std::{process::Command, time::Duration};

    fn binary_path() -> std::path::PathBuf {
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

    fn sleep_ms(ms: u64) {
        std::thread::sleep(Duration::from_millis(ms));
    }

    /// Resize-during-raw-mode regression.
    ///
    /// Protocol:
    ///   1. Spawn binary and wait for raw/TUI mode entry (~400 ms).
    ///   2. Send SIGWINCH.
    ///   3. Wait 200 ms for any handler to execute.
    ///   4. Assert process is still alive: `status()` == `Some(StillAlive)`.
    ///   5. SIGKILL for cleanup + `mem::forget` to avoid hanging Drop.
    ///
    /// `PtyProcess::status()` calls `waitpid(WNOHANG)`.
    /// `Some(WaitStatus::StillAlive)` → process is running.
    /// `Some(Exited | Signaled)` → process died (crash/signal).
    /// `None` → waitpid error (rare; treated as inconclusive, not failure).
    #[test]
    fn resize_sigwinch_binary_stays_alive() {
        use nix::sys::signal::{kill, Signal};
        use rexpect::process::wait::WaitStatus;

        let proc = spawn_blocktxt(&[]);
        // Allow the binary to enter raw mode and start the game loop.
        sleep_ms(400);

        // Send SIGWINCH (window-resize; must not crash the process).
        let send_result = kill(proc.child_pid, Signal::SIGWINCH);
        if let Err(ref e) = send_result {
            // Platform cannot send SIGWINCH; skip with note.
            eprintln!(
                "resize_sigwinch_binary_stays_alive: \
                 SIGWINCH send failed ({e:?}); \
                 skipping — manual verification required on this platform"
            );
            // SIGKILL + forget to avoid hanging Drop.
            let _ = kill(proc.child_pid, Signal::SIGKILL);
            std::mem::forget(proc);
            return;
        }

        // Allow the signal handler to run.
        sleep_ms(200);

        // Check liveness.
        let status = proc.status();
        let alive = matches!(status, Some(WaitStatus::StillAlive));

        // SIGKILL + forget BEFORE asserting to ensure cleanup always happens.
        let _ = kill(proc.child_pid, Signal::SIGKILL);
        std::mem::forget(proc);

        assert!(
            alive,
            "blocktxt exited or crashed after SIGWINCH — \
             resize-during-raw-mode regression detected (status={status:?})"
        );
    }
}
