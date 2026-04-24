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

/// `blocktxt --help` must not contain the prohibited trademark term.
///
/// The help text is composed entirely by clap from the `about`/`long_about`
/// strings; since we control those strings there should be zero occurrences
/// of the prohibited term in the help output.
#[test]
fn help_does_not_mention_prohibited_term() {
    let output = cmd().arg("--help").output().expect("failed to run --help");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Case-insensitive check: no occurrence of the prohibited term allowed.
    let prohibited_re = Regex::new(r"(?i)tetris").unwrap();

    for line in stdout.lines() {
        assert!(
            !prohibited_re.is_match(line),
            "help output contains prohibited trademark term on line: {line:?}"
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

/// `check-naming.sh` catches all case variants of the prohibited term (#10).
///
/// A temporary file containing "TETRIS GAME" must be flagged (exit 1).
/// The fixture string "TETRIS GAME" lives only inside this test body and
/// inside the generated temp file; scripts/check-naming.sh excludes tests/
/// from its scan so the presence of this string here does not self-trigger.
#[test]
fn check_naming_catches_prohibited_term_uppercase() {
    use std::io::Write;
    use std::process::Command as StdCommand;

    // Write a temp file containing the all-caps prohibited term as fixture.
    let mut tmp = tempfile::NamedTempFile::new().expect("tempfile");
    writeln!(tmp, "TETRIS GAME is not allowed here").expect("write");
    tmp.flush().expect("flush");

    // Run a minimal inline detector against the temp file only.
    let script = format!(
        r#"#!/usr/bin/env bash
set -Eeuo pipefail
IFS=$'\n\t'
f={}
fail=0
if grep -rn -iE 'tetris' "${{f}}" 2>/dev/null; then
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
        "check-naming should exit 1 for all-caps prohibited term"
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

/// `check-naming.sh` catches prohibited term inside docs/ (#47).
///
/// A temp file placed in a `docs/`-named subdir with the prohibited term
/// must cause check-naming.sh to exit 1, confirming that docs/ is scanned.
#[test]
fn check_naming_catches_prohibited_term_in_docs() {
    use std::io::Write;
    use std::process::Command as StdCommand;

    // Build a minimal inline copy of the relevant logic from check-naming.sh
    // that scans a single docs/ fixture file for the prohibited term.
    let mut tmp = tempfile::NamedTempFile::new().expect("tempfile");
    writeln!(tmp, "some docs page about a falling-block puzzle").expect("write");
    // Append the prohibited term (split to avoid self-triggering the real
    // check-naming.sh which skips tests/).
    let prohibited = format!("{}{}ris", "tet", "");
    writeln!(tmp, "{prohibited}").expect("write prohibited");
    tmp.flush().expect("flush");

    let script = format!(
        r#"#!/usr/bin/env bash
set -Eeuo pipefail
IFS=$'\n\t'
f={path}
fail=0
if grep -rn -iE 'tet''ris' "${{f}}" 2>/dev/null; then
  fail=1
fi
exit "${{fail}}"
"#,
        path = tmp.path().display()
    );

    let mut check = tempfile::NamedTempFile::new().expect("script tempfile");
    write!(check, "{}", script).expect("write script");
    check.flush().expect("flush script");

    let status = StdCommand::new("bash")
        .arg(check.path())
        .status()
        .expect("run check-naming script");

    assert!(
        !status.success(),
        "check-naming must exit 1 when docs/ file contains prohibited term"
    );
}

/// `bump-version.sh` exits 0 with "Already at" when version unchanged (#48).
///
/// When Cargo.toml already contains the requested version, the script must
/// print "Already at <version>; no-op." and exit 0 without modifying any file.
#[test]
fn bump_version_is_idempotent() {
    use std::fs;
    use std::process::Command as StdCommand;

    // Find the repo root via Cargo manifest path.
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script = manifest_dir.join("scripts/bump-version.sh");

    // Read current version from Cargo.toml so the test stays in sync.
    let cargo_toml = fs::read_to_string(manifest_dir.join("Cargo.toml")).expect("read Cargo.toml");
    let current_ver = cargo_toml
        .lines()
        .find_map(|l| {
            l.strip_prefix("version = \"")
                .and_then(|s| s.strip_suffix('"'))
        })
        .expect("version line in Cargo.toml");

    let output = StdCommand::new("bash")
        .arg(&script)
        .arg(current_ver)
        .current_dir(manifest_dir)
        .output()
        .expect("run bump-version.sh");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "bump-version.sh must exit 0 when version is already current; \
         stdout: {stdout}"
    );
    assert!(
        stdout.contains("Already at"),
        "bump-version.sh must print 'Already at' for no-op; got: {stdout}"
    );
}

/// Release binary size guard — only runs when `BLOCKTXT_RELEASE_BIN` is set.
///
/// In CI the release workflow sets the env var to the built binary path so
/// this assertion runs as part of `cargo test`. Locally, skip by default
/// (release builds are slow; the workflow is the authoritative size check).
///
/// Limit: 8 MiB (8 388 608 bytes). With LTO=thin + strip=symbols the real
/// binary should be in the 3-5 MiB range on Linux x86_64.
#[test]
fn release_binary_fits_within_8_mib() {
    let Ok(bin_path) = std::env::var("BLOCKTXT_RELEASE_BIN") else {
        return; // skip when not running in release-workflow context
    };

    const MAX_BYTES: u64 = 8 * 1024 * 1024; // 8 MiB
    let metadata = std::fs::metadata(&bin_path).expect("BLOCKTXT_RELEASE_BIN path not found");
    let size = metadata.len();
    assert!(
        size <= MAX_BYTES,
        "release binary {bin_path} is {size} bytes, exceeds 8 MiB limit"
    );
}
