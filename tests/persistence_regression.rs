//! Persistence failure-mode regression suite (SPEC §5 / Sprint 4 Track D).
//!
//! These tests complement the unit-level coverage in `tests/persistence.rs`
//! (added in PR #15).  Duplicates of the symlink/corrupt-rename/cap-5 tests
//! are omitted here to keep `tests/` clean.  Coverage added by this file:
//!   - World-writable *parent* dir: data dir creation and safety-check
//!     still succeed (only the data dir itself needs hardening).
//!   - Wrong-owner rejection: gated `#[cfg(unix)]` + `#[ignore]` — requires
//!     root to exercise (see test doc-comment for the manual plan).
//!   - Two corrupt loads → distinct backup paths (exercises the counter
//!     suffix in `unique_corrupt_path`).
//!   - `unique_corrupt_path` collision avoidance across repeated calls.
//!   - Load-save-load accumulation: multiple sessions add up correctly.
//!   - Disk-full simulation: documented as a manual-test-plan item because
//!     the codebase does not expose a mockable filesystem trait.

use std::{fs, path::PathBuf};

use tempfile::TempDir;

use blocktxt::persistence::{
    check_dir_safety, create_dir_mode_0700, load, save, scores_path, unique_corrupt_path,
    HighScore, HighScoreStore, PersistenceError,
};

// ── helpers ───────────────────────────────────────────────────────────────

fn make_tmp_dir() -> (TempDir, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let data = tmp.path().join("data");
    create_dir_mode_0700(&data).unwrap();
    (tmp, data)
}

fn make_score(score: u32) -> HighScore {
    HighScore {
        name: "tester".to_owned(),
        score,
        level: 1,
        lines: 4,
        ts: 0,
    }
}

// ── world-writable parent ─────────────────────────────────────────────────

/// A world-writable parent directory must NOT prevent the data dir itself
/// from being created with mode 0o700 and passing safety checks.
///
/// Covers a scenario `tests/persistence.rs` does not: the parent is hostile
/// but the data dir still gets hardened correctly.
#[test]
#[cfg(unix)]
fn world_writable_parent_does_not_block_data_dir() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = tempfile::tempdir().unwrap();
    let parent = tmp.path().join("parent_dir");
    fs::create_dir(&parent).unwrap();

    // Make the parent world-writable.
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o777)).unwrap();

    let data_dir = parent.join("data");
    create_dir_mode_0700(&data_dir).expect("create_dir_mode_0700 should succeed");

    // The data dir itself must pass the safety check.
    check_dir_safety(&data_dir).expect("data dir should be safe even with world-writable parent");
}

// ── wrong owner (Unix, root-gated) ────────────────────────────────────────

/// Test for wrong-owner rejection.
///
/// Changing file ownership requires CAP_CHOWN / root; this test is
/// `#[ignore]` by default so it only runs when explicitly selected with
/// `-- --include-ignored` and the test runner has root privileges.
///
/// Manual test plan (if root):
///   1. Create a temp directory.
///   2. `chown 0:0 <dir>` (owned by root).
///   3. Run as non-root: `check_dir_safety(<dir>)` → `WrongOwner`.
#[test]
#[cfg(unix)]
#[ignore = "requires running as root to chown; see doc comment for manual plan"]
fn wrong_owner_rejected() {
    // This body executes only when --include-ignored is passed.
    // In CI (non-root), the test is skipped entirely.
    //
    // If running as root, verify that a dir owned by another uid
    // (e.g., uid=65534 / nobody) triggers WrongOwner.
    use nix::unistd::Uid;
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("foreign_owned");
    fs::create_dir(&dir).unwrap();

    // Attempt to chown to nobody (uid 65534); if this fails we skip.
    let nobody = Uid::from_raw(65534);
    if nix::unistd::chown(&dir, Some(nobody), None).is_err() {
        eprintln!("chown failed — not root; skipping");
        return;
    }

    let result = check_dir_safety(&dir);
    assert!(
        matches!(result, Err(PersistenceError::WrongOwner)),
        "expected WrongOwner, got {result:?}"
    );
}

// ── corrupt + load cycle ──────────────────────────────────────────────────

/// Two corrupt loads in quick succession must use distinct backup paths.
///
/// Complements `tests/persistence.rs::load_corrupt_renames_and_returns_empty`
/// which only exercises a single corruption.  This test verifies the
/// counter-suffix path in `unique_corrupt_path` when two renames happen in
/// the same second.
#[test]
fn two_corrupt_loads_distinct_suffix() {
    let (_tmp, dir) = make_tmp_dir();
    let path = scores_path(&dir);

    // First corrupt.
    fs::write(&path, b"bad json 1").unwrap();
    let _store1 = load(&dir).unwrap();

    // Second corrupt.
    fs::write(&path, b"bad json 2").unwrap();
    let _store2 = load(&dir).unwrap();

    // Count .corrupt.* siblings.
    let base_str = path.to_str().unwrap();
    let prefix = format!("{base_str}.corrupt.");
    let mut siblings: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.to_str().map(|s| s.starts_with(&prefix)).unwrap_or(false))
        .collect();
    siblings.sort();

    assert_eq!(
        siblings.len(),
        2,
        "expected 2 distinct corrupt backups, got {}: {siblings:?}",
        siblings.len()
    );
    assert_ne!(
        siblings[0], siblings[1],
        "two corrupt files must have distinct paths"
    );
}

/// `unique_corrupt_path` must return distinct paths on repeated calls.
#[test]
fn unique_corrupt_path_no_collision() {
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path().join("scores.json");

    let p1 = unique_corrupt_path(&base).unwrap();
    // Create p1 so p2 must pick a different name.
    fs::write(&p1, b"x").unwrap();

    let p2 = unique_corrupt_path(&base).unwrap();
    assert_ne!(p1, p2, "unique_corrupt_path must avoid collisions");
}

/// Load a valid file, then save more scores, reload — store grows correctly.
#[test]
fn load_save_load_accumulates() {
    let (_tmp, dir) = make_tmp_dir();

    // First save.
    let mut store = HighScoreStore::new();
    store.insert(make_score(100));
    save(&store, &dir).unwrap();

    // Load and add more.
    let mut store2 = load(&dir).unwrap();
    store2.insert(make_score(200));
    save(&store2, &dir).unwrap();

    // Final load.
    let store3 = load(&dir).unwrap();
    let top = store3.top(10);
    assert_eq!(top.len(), 2, "should have 2 entries after accumulation");
    assert_eq!(top[0].score, 200, "highest score first");
    assert_eq!(top[1].score, 100, "second score after");
}

// ── disk-full simulation ──────────────────────────────────────────────────

// Disk-full simulation is omitted: the codebase does not expose a
// filesystem abstraction trait that can be mocked.  A manual test plan:
//
//   1. Create a small filesystem image:
//      `dd if=/dev/zero of=/tmp/tiny.img bs=1k count=32`
//      `mkfs.ext2 /tmp/tiny.img`
//   2. Mount it: `sudo mount -o loop /tmp/tiny.img /mnt/tiny`
//   3. Fill it almost full.
//   4. Run: `BLOCKTXT_DATA=/mnt/tiny cargo run -- blocktxt`
//   5. Trigger a game-over → persistence::save() should return Err(Io(_))
//      without panicking.
//   6. The game should continue running in degraded mode.
