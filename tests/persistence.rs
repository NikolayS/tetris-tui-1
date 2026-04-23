//! Integration tests for `persistence.rs`.
//!
//! Security-critical paths (symlink, world-writable, mode) are gated
//! with `#[cfg(unix)]` or `#[cfg_attr(not(unix), ignore)]`.

use blocktxt::persistence::{
    self, check_dir_safety, create_dir_mode_0700, HighScore, HighScoreStore, PersistenceError,
};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use tempfile::TempDir;

// ── helpers ───────────────────────────────────────────────────────────────

fn make_score(name: &str, score: u32) -> HighScore {
    HighScore {
        name: name.to_owned(),
        score,
        level: 1,
        lines: 10,
        ts: 0,
    }
}

fn data_dir(tmp: &TempDir) -> PathBuf {
    tmp.path().join("data")
}

// ── directory hardening ───────────────────────────────────────────────────

/// `create_dir_mode_0700` must produce a directory with mode 0o700 on Unix.
#[test]
#[cfg(unix)]
fn init_data_dir_creates_with_0o700() {
    use std::os::unix::fs::MetadataExt;

    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("newdir");

    create_dir_mode_0700(&dir).expect("should create dir");

    let meta = fs::metadata(&dir).unwrap();
    let mode = meta.mode() & 0o777;
    assert_eq!(mode, 0o700, "expected mode 0o700, got 0o{mode:o}");
}

/// `check_dir_safety` must refuse a symlink.
#[test]
#[cfg(unix)]
fn init_data_dir_refuses_symlink() {
    use std::os::unix::fs as unix_fs;

    let tmp = tempfile::tempdir().unwrap();
    let real_dir = tmp.path().join("real");
    let link = tmp.path().join("link");
    fs::create_dir(&real_dir).unwrap();
    unix_fs::symlink(&real_dir, &link).unwrap();

    let err = check_dir_safety(&link).unwrap_err();
    assert!(
        matches!(err, PersistenceError::UnsafeSymlink),
        "unexpected error: {err}"
    );
}

/// `check_dir_safety` must refuse a world-writable directory (0o777).
#[test]
#[cfg(unix)]
fn init_data_dir_refuses_world_writable() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("public");
    fs::create_dir(&dir).unwrap();
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o777)).unwrap();

    let err = check_dir_safety(&dir).unwrap_err();
    assert!(
        matches!(err, PersistenceError::UnsafeGroupOrOther),
        "unexpected error: {err}"
    );
}

/// `check_dir_safety` must refuse a dir at 0o755 (group-readable).
///
/// SPEC §4: `mode & 0o077 == 0` must hold. 0o755 has group+execute (0o055
/// > 0) so it must be rejected even though it is not world-writable.
#[test]
#[cfg(unix)]
fn init_data_dir_refuses_group_readable() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("grpread");
    fs::create_dir(&dir).unwrap();
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();

    let err = check_dir_safety(&dir).unwrap_err();
    assert!(
        matches!(err, PersistenceError::UnsafeGroupOrOther),
        "0o755 dir should be rejected; got: {err}"
    );
}

// ── save / load roundtrip ─────────────────────────────────────────────────

#[test]
fn save_then_load_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = data_dir(&tmp);
    create_dir_mode_0700(&dir).unwrap();

    let mut store = HighScoreStore::new();
    store.insert(make_score("Alice", 1000));
    store.insert(make_score("Bob", 500));

    persistence::save(&store, &dir).expect("save should succeed");

    let loaded = persistence::load(&dir).expect("load should succeed");
    let top = loaded.top(10);
    assert_eq!(top.len(), 2);
    assert_eq!(top[0].score, 1000);
    assert_eq!(top[0].name, "Alice");
    assert_eq!(top[1].score, 500);
}

/// Saved file must have mode 0o600 on Unix.
#[test]
#[cfg(unix)]
fn save_writes_0o600() {
    use std::os::unix::fs::MetadataExt;

    let tmp = tempfile::tempdir().unwrap();
    let dir = data_dir(&tmp);
    create_dir_mode_0700(&dir).unwrap();

    let store = HighScoreStore::new();
    persistence::save(&store, &dir).unwrap();

    let path = persistence::scores_path(&dir);
    let meta = fs::metadata(&path).unwrap();
    let mode = meta.mode() & 0o777;
    assert_eq!(mode, 0o600, "expected 0o600, got 0o{mode:o}");
}

// ── corrupt-file recovery ─────────────────────────────────────────────────

/// Writing garbage JSON then calling `load` must:
/// - return an empty store, and
/// - create a `.corrupt.<ts>` backup file.
#[test]
fn load_corrupt_renames_and_returns_empty() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = data_dir(&tmp);
    create_dir_mode_0700(&dir).unwrap();

    let scores_file = persistence::scores_path(&dir);
    fs::write(&scores_file, b"not json at all").unwrap();

    let store = persistence::load(&dir).expect("load should not propagate error");

    // Store must be empty.
    assert_eq!(store.top(10).len(), 0);

    // Original file must be gone.
    assert!(
        !scores_file.exists(),
        "corrupt file should have been renamed away"
    );

    // A backup with the `.corrupt.` prefix must exist.
    let backups = find_corrupt_backups(&dir, "scores.json");
    assert!(!backups.is_empty(), "expected a .corrupt.* backup file");
}

/// After 6 pre-existing `.corrupt.*` files a new corruption must delete
/// the oldest, leaving exactly 5 backups.
#[test]
fn corrupt_backup_cap_at_5() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = data_dir(&tmp);
    create_dir_mode_0700(&dir).unwrap();

    // Create 6 pre-existing backups with timestamps 100000000..100000005.
    // The oldest has ts=100000000.
    for i in 0u64..=5 {
        let p = dir.join(format!("scores.json.corrupt.{}", 100_000_000 + i));
        fs::write(&p, b"old").unwrap();
        // Stagger mtimes so the oldest is unambiguous.
        // We rely on creation order; on most OSes that's sufficient.
    }

    // Force a fresh corruption.
    let scores_file = persistence::scores_path(&dir);
    fs::write(&scores_file, b"garbage").unwrap();
    persistence::load(&dir).unwrap();

    let backups = find_corrupt_backups(&dir, "scores.json");
    assert_eq!(
        backups.len(),
        5,
        "expected exactly 5 backups, got {}: {backups:?}",
        backups.len()
    );
}

/// Collision loop: planting `<secs>` AND `<secs>-1` forces the counter to 2.
///
/// Verifies the loop-based dedup in `unique_corrupt_path` (closes #18).
#[test]
fn no_overwrite_corrupt_counter_loops_past_one() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = data_dir(&tmp);
    create_dir_mode_0700(&dir).unwrap();

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Plant collisions at counter 0 (bare ts) and counter 1.
    let base = dir.join("scores.json");
    let c0 = dir.join(format!("scores.json.corrupt.{ts}"));
    let c1 = dir.join(format!("scores.json.corrupt.{ts}-1"));
    fs::write(&c0, b"old0").unwrap();
    fs::write(&c1, b"old1").unwrap();

    let result = persistence::unique_corrupt_path(&base);
    // Must differ from both planted paths.
    assert_ne!(result, c0, "must not reuse bare-ts path");
    assert_ne!(result, c1, "must not reuse counter-1 path");
    assert!(!result.exists(), "returned path must not already exist");
}

/// Two corruptions that occur within the same second must produce
/// distinct backup file names (no silent overwrite).
#[test]
fn no_overwrite_existing_corrupt_same_second() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = data_dir(&tmp);
    create_dir_mode_0700(&dir).unwrap();

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Plant a backup that collides with the current second.
    let collider = dir.join(format!("scores.json.corrupt.{ts}"));
    fs::write(&collider, b"old").unwrap();

    // Now trigger a new corruption.
    let scores_file = persistence::scores_path(&dir);
    fs::write(&scores_file, b"also garbage").unwrap();
    persistence::load(&dir).unwrap();

    let backups = find_corrupt_backups(&dir, "scores.json");
    // We should have at least 2 distinct backups.
    assert!(
        backups.len() >= 2,
        "expected ≥2 distinct backups, got {}: {backups:?}",
        backups.len()
    );
}

// ── HighScoreStore logic ──────────────────────────────────────────────────

#[test]
fn insert_returns_true_for_personal_best() {
    let mut store = HighScoreStore::new();
    // First score for Alice is always a personal best.
    assert!(store.insert(make_score("Alice", 100)));
    // 50 < 100 — not a personal best.
    assert!(!store.insert(make_score("Alice", 50)));
    // 200 > 100 — new personal best.
    assert!(store.insert(make_score("Alice", 200)));
}

#[test]
fn top_n_is_sorted_descending() {
    let mut store = HighScoreStore::new();
    for &s in &[300u32, 100, 200, 500, 400] {
        store.insert(make_score("Player", s));
    }
    let top = store.top(3);
    assert_eq!(top.len(), 3);
    assert_eq!(top[0].score, 500);
    assert_eq!(top[1].score, 400);
    assert_eq!(top[2].score, 300);
}

// ── helper ────────────────────────────────────────────────────────────────

fn find_corrupt_backups(dir: &Path, base_name: &str) -> Vec<PathBuf> {
    let prefix = format!("{base_name}.corrupt.");
    fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            let name = p.file_name()?.to_str()?.to_owned();
            if name.starts_with(&prefix) {
                Some(p)
            } else {
                None
            }
        })
        .collect()
}
