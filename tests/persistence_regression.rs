//! Persistence failure-mode regression suite (SPEC §5 / Sprint 4 Track D).
//!
//! Covers:
//!   - Symlink attack: check_dir_safety returns UnsafeSymlink.
//!   - World-writable parent: init still works (only data dir hardened).
//!   - Corrupt + load cycle: garbage JSON → rename to .corrupt.<ts>;
//!     second corrupt → distinct suffix; 6 total → oldest pruned to cap 5.
//!   - Disk-full simulation: skipped (documented below).
//!   - Wrong owner: gated cfg(unix) + ignore if not root.

use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

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

// ── symlink attack ────────────────────────────────────────────────────────

/// `check_dir_safety` on a path that IS a symlink must return UnsafeSymlink.
#[test]
#[cfg(unix)]
fn symlink_attack_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let real_dir = tmp.path().join("real");
    fs::create_dir(&real_dir).unwrap();

    let link_path = tmp.path().join("link");
    std::os::unix::fs::symlink(&real_dir, &link_path).unwrap();

    let result = check_dir_safety(&link_path);
    assert!(
        matches!(result, Err(PersistenceError::UnsafeSymlink)),
        "expected UnsafeSymlink, got {result:?}"
    );
}

/// On non-Unix, just assert the function succeeds (no symlink check).
#[test]
#[cfg(not(unix))]
fn symlink_not_checked_on_non_unix() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("data");
    create_dir_mode_0700(&dir).unwrap();
    // Should be Ok on non-Unix regardless.
    assert!(check_dir_safety(&dir).is_ok());
}

// ── world-writable parent ─────────────────────────────────────────────────

/// A world-writable parent directory must NOT prevent the data dir itself
/// from being created with mode 0o700 and passing safety checks.
///
/// On macOS the sticky bit (0o1777 on /tmp) is normally set; using a
/// controlled tempdir avoids that.  We chmod the *parent* to 0o777 and
/// verify `create_dir_mode_0700` + `check_dir_safety` on the child still work.
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

/// Write garbage JSON → load returns empty store and renames to .corrupt.<ts>.
#[test]
fn corrupt_file_renamed_on_load() {
    let (_tmp, dir) = make_tmp_dir();
    let path = scores_path(&dir);

    fs::write(&path, b"this is not json {{{{").unwrap();
    assert!(path.exists(), "corrupt file must exist before load");

    let store = load(&dir).expect("load should succeed with fallback");
    assert_eq!(
        store.top(10).len(),
        0,
        "load of corrupt file should return empty store"
    );
    assert!(!path.exists(), "corrupt file should be renamed away");

    // A .corrupt.<ts> sibling must now exist.
    let prefix = format!("{}.corrupt.", path.display());
    let siblings: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .filter(|e| {
            e.path()
                .to_str()
                .map(|s| s.starts_with(&prefix))
                .unwrap_or(false)
        })
        .collect();
    assert!(
        !siblings.is_empty(),
        "at least one .corrupt.<ts> backup must exist"
    );
}

/// Two corrupt loads in quick succession must use distinct backup paths.
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

/// Six corrupt loads → only 5 backups kept (oldest pruned).
///
/// We force distinct timestamps by writing the backup files directly with
/// `unique_corrupt_path` and synthetic mtime ordering, then load once more
/// (the 7th action) and verify pruning.
///
/// Because `unique_corrupt_path` uses `SystemTime::now()` which can repeat
/// within a second on fast machines, we use the counter-suffix variant.
/// We create 5 pre-existing backups, write a 6th corrupt file, load, and
/// assert that exactly 5 backups remain after pruning.
#[test]
fn corrupt_cap_five_backups() {
    let (_tmp, dir) = make_tmp_dir();
    let path = scores_path(&dir);

    // Plant 5 fake backups with earlier timestamps (mtime set implicitly
    // by creation order — all within the same second, so unique_corrupt_path
    // will use counter suffixes for collisions).
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Use timestamps well in the past to guarantee oldest-first ordering.
    for i in 0u64..5 {
        let fake_backup = PathBuf::from(format!(
            "{}.corrupt.{}",
            path.display(),
            now_secs - 100 + i
        ));
        fs::write(&fake_backup, b"old backup").unwrap();
    }

    // Now write the 6th corrupt file and load.
    fs::write(&path, b"not json at all").unwrap();
    let store = load(&dir).unwrap();
    assert_eq!(store.top(10).len(), 0, "fallback must return empty store");

    // Count .corrupt.* siblings.
    let base_str = path.to_str().unwrap();
    let prefix = format!("{base_str}.corrupt.");
    let siblings: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.to_str().map(|s| s.starts_with(&prefix)).unwrap_or(false))
        .collect();

    assert_eq!(
        siblings.len(),
        5,
        "after 6 corrupt loads, exactly 5 backups must remain; got {}: {siblings:?}",
        siblings.len()
    );
}

/// `unique_corrupt_path` must return distinct paths on repeated calls.
#[test]
fn unique_corrupt_path_no_collision() {
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path().join("scores.json");

    let p1 = unique_corrupt_path(&base);
    // Create p1 so p2 must pick a different name.
    fs::write(&p1, b"x").unwrap();

    let p2 = unique_corrupt_path(&base);
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
