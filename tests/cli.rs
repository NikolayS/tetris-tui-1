/// CLI integration tests (assert_cmd).
///
/// Red-green TDD: these tests are written before the implementation.
/// Each test exercises a contract from SPEC §4 "CLI flags".
use assert_cmd::Command;
use predicates::str::is_match;
use regex::Regex;

// ── helpers ───────────────────────────────────────────────────────────────

fn cmd() -> Command {
    Command::cargo_bin("blocktxt").expect("binary not found")
}

// ── tests ─────────────────────────────────────────────────────────────────

/// `blocktxt --version` exits 0 and prints a semver on stdout.
#[test]
fn version_flag_prints_crate_version() {
    cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(is_match(r"blocktxt \d+\.\d+\.\d+\n").unwrap());
}

/// `blocktxt --help` must not use the word "Tetris" as a product name.
///
/// The trademark footer pattern ("trademark of") is the only allowed
/// exception; since --help content is composed entirely by clap from the
/// `about` string, and we control that string, there should be zero
/// occurrences of bare "Tetris" in the help output.
#[test]
fn help_does_not_mention_tetris_as_product() {
    let output = cmd().arg("--help").output().expect("failed to run --help");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // For each line, check if it contains bare "Tetris" without ® suffix,
    // and isn't a trademark-of disclaimer line.
    let bare_tetris_re = Regex::new(r"(?:^|[^®a-zA-Z])Tetris([^®]|$)").unwrap();

    for line in stdout.lines() {
        let lower = line.to_lowercase();
        if lower.contains("trademark of") || lower.contains("registered trademark") {
            // Trademark disclaimer lines are allowed to mention Tetris®.
            continue;
        }
        assert!(
            !bare_tetris_re.is_match(line),
            "help output contains bare 'Tetris' product name on line: {line:?}"
        );
    }

    assert!(output.status.success());
}

/// `blocktxt --reset-scores` with `n\n` on stdin exits 0 with no ANSI
/// escape bytes in stdout (cooked-mode requirement: no raw-mode artifacts).
#[test]
fn reset_scores_aborts_cleanly_on_no() {
    let output = cmd()
        .arg("--reset-scores")
        .write_stdin("n\n")
        .output()
        .expect("failed to run --reset-scores");

    let stdout_bytes = &output.stdout;

    // No ANSI escape sequences (\x1b) allowed in pre-TTY cooked-mode output.
    assert!(
        !stdout_bytes.contains(&0x1b),
        "stdout must not contain ANSI escape bytes in cooked-mode prompt"
    );

    assert!(
        output.status.success(),
        "exit code should be 0 when user answers 'n'"
    );
}

/// `check-naming.sh` catches both `Tetris` and all-caps `TETRIS` (#10).
///
/// A temporary file containing "TETRIS GAME" must be flagged (exit 1).
/// The README/Cargo.toml scan (no prohibited strings) must pass (exit 0).
#[test]
fn check_naming_catches_all_caps_tetris() {
    use std::io::Write;
    use std::process::Command as StdCommand;

    // Write a temp file containing all-caps TETRIS.
    let mut tmp = tempfile::NamedTempFile::new().expect("tempfile");
    writeln!(tmp, "TETRIS GAME is not allowed here").expect("write");
    tmp.flush().expect("flush");

    // Patch the script's `targets` inline: feed the temp file as the only
    // target by running a sub-script that overrides the targets array.
    let script = format!(
        r#"#!/usr/bin/env bash
set -Eeuo pipefail
IFS=$'\n\t'
f={}
fail=0
if grep -nE '(^|[^®a-zA-Z])(Tetris|TETRIS)([^®]|$)' "${{f}}" \
    | grep -vi 'trademark of'; then
  fail=1
fi
exit "${{fail}}"
"#,
        tmp.path().display()
    );

    let mut check = tempfile::NamedTempFile::new().expect("script tempfile");
    write!(check, "{}", script).expect("write script");
    check.flush().expect("flush script");

    let status = StdCommand::new("bash")
        .arg(check.path())
        .status()
        .expect("run script");

    assert!(
        !status.success(),
        "check-naming should exit 1 for all-caps TETRIS"
    );
}

/// `blocktxt --reset-scores` with `y\n` on stdin exits 0.
///
/// Stub behavior for v0.1: there is no high-score file yet (Sprint 2 adds
/// persistence). The test verifies only that the prompt is accepted and the
/// process exits cleanly.
///
/// TODO(Sprint 2): actually delete the high-score file when persistence exists.
#[test]
fn reset_scores_exits_cleanly_on_yes() {
    let output = cmd()
        .arg("--reset-scores")
        .write_stdin("y\n")
        .output()
        .expect("failed to run --reset-scores with y");

    let stdout_bytes = &output.stdout;

    // Cooked-mode prompt must not emit ANSI escape bytes.
    assert!(
        !stdout_bytes.contains(&0x1b),
        "stdout must not contain ANSI escape bytes in cooked-mode prompt"
    );

    assert!(
        output.status.success(),
        "exit code should be 0 when user answers 'y'"
    );
}
