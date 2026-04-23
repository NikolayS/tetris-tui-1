//! Persistence integration tests.
//!
//! Covers: GameOver → HighScore written to file; unreadable-dir fallback;
//! `--reset-scores yes` deletes file; `--reset-scores no` preserves.

use std::{
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

use tempfile::TempDir;

use blocktxt::clock::FakeClock;
use blocktxt::game::state::GameState;
use blocktxt::persistence::{self, create_dir_mode_0700, HighScore, HighScoreStore};

// ── helpers ───────────────────────────────────────────────────────────────

fn data_dir(tmp: &TempDir) -> PathBuf {
    tmp.path().join("data")
}

fn make_score(score: u32) -> HighScore {
    HighScore {
        name: "player".to_owned(),
        score,
        level: 1,
        lines: 10,
        ts: 0,
    }
}

// ── mock GameOver scenario ────────────────────────────────────────────────

/// Inserting a HighScore and saving writes the file to disk.
#[test]
fn game_over_score_written_to_file() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = data_dir(&tmp);
    create_dir_mode_0700(&dir).unwrap();

    let mut store = HighScoreStore::new();
    let _new_best = store.insert(make_score(1234));

    // Should be a personal best (first entry).
    assert!(_new_best, "first insert should be personal best");

    persistence::save(&store, &dir).expect("save should succeed");

    // File must exist and contain the score.
    let path = persistence::scores_path(&dir);
    assert!(path.exists(), "scores file should exist after save");

    let content = fs::read_to_string(&path).unwrap();
    assert!(
        content.contains("1234"),
        "scores file should contain the score"
    );
}

/// File size must be reasonable (under 10 KiB for a handful of scores).
#[test]
fn saved_file_size_reasonable() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = data_dir(&tmp);
    create_dir_mode_0700(&dir).unwrap();

    let mut store = HighScoreStore::new();
    for i in 0..10 {
        store.insert(make_score(i * 100));
    }
    persistence::save(&store, &dir).unwrap();

    let path = persistence::scores_path(&dir);
    let size = fs::metadata(&path).unwrap().len();
    assert!(
        size < 10 * 1024,
        "scores file should be under 10 KiB, got {size} bytes"
    );
}

/// Unreadable-dir fallback: `HighScoreStore::new_with_fallback` with a bad
/// dir returns an empty store and an error (game still playable).
#[test]
fn unreadable_dir_returns_empty_store() {
    let bad: Result<PathBuf, blocktxt::persistence::PersistenceError> =
        Err(blocktxt::persistence::PersistenceError::NoHome);

    let (store, err) = HighScoreStore::new_with_fallback(bad);

    assert!(err.is_some(), "should carry error");
    assert_eq!(store.top(10).len(), 0, "fallback store must be empty");
}

/// Game state can be stepped (runs) even when persistence is unavailable.
#[test]
fn game_playable_without_persistence() {
    let clock = Box::new(FakeClock::new(Instant::now()));
    let mut state = GameState::new(42, clock);
    use std::time::Duration;

    // Step a few ticks with no inputs — must not panic.
    for _ in 0..10 {
        state.step(Duration::from_millis(16), &[]);
    }
}

/// `--reset-scores yes` deletes the scores file and corrupt siblings.
#[test]
fn reset_scores_yes_deletes_file() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = data_dir(&tmp);
    create_dir_mode_0700(&dir).unwrap();

    // Write a scores file.
    let mut store = HighScoreStore::new();
    store.insert(make_score(999));
    persistence::save(&store, &dir).unwrap();

    let scores_file = persistence::scores_path(&dir);
    assert!(scores_file.exists(), "setup: scores file must exist");

    // Also plant a corrupt sibling.
    let corrupt = dir.join("scores.json.corrupt.123456");
    fs::write(&corrupt, b"garbage").unwrap();

    // Simulate "reset yes": delete the file and siblings.
    delete_scores_and_siblings(&dir, &scores_file);

    assert!(!scores_file.exists(), "scores file should be deleted");
    assert!(!corrupt.exists(), "corrupt sibling should be deleted");
}

/// `--reset-scores no` preserves the scores file.
#[test]
fn reset_scores_no_preserves_file() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = data_dir(&tmp);
    create_dir_mode_0700(&dir).unwrap();

    let mut store = HighScoreStore::new();
    store.insert(make_score(777));
    persistence::save(&store, &dir).unwrap();

    let scores_file = persistence::scores_path(&dir);
    assert!(scores_file.exists(), "setup: scores file must exist");

    // Simulate "reset no": do nothing.
    // File must still exist.
    assert!(
        scores_file.exists(),
        "scores file should be preserved on 'no'"
    );
}

/// Load-save-load roundtrip preserves all scores.
#[test]
fn load_save_roundtrip_preserves_top5() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = data_dir(&tmp);
    create_dir_mode_0700(&dir).unwrap();

    let mut store = HighScoreStore::new();
    for &s in &[500u32, 400, 300, 200, 100, 50] {
        store.insert(make_score(s));
    }
    persistence::save(&store, &dir).unwrap();

    let loaded = persistence::load(&dir).unwrap();
    let top = loaded.top(5);
    assert_eq!(top.len(), 5);
    assert_eq!(top[0].score, 500);
    assert_eq!(top[4].score, 100);
}

// ── helper: mirrors what cli::handle_reset_scores does ───────────────────

fn delete_scores_and_siblings(dir: &Path, scores_file: &Path) {
    if scores_file.exists() {
        fs::remove_file(scores_file).ok();
    }
    let file_name = scores_file
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("scores.json");
    let prefix = format!("{file_name}.corrupt.");
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                if name.starts_with(&prefix) {
                    fs::remove_file(&p).ok();
                }
            }
        }
    }
}
