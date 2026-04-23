//! Hardened high-score persistence.
//!
//! Security model (SPEC §4 round-2):
//! - Directory must be owned by current user, not world-writable,
//!   and must not be a symlink.
//! - Files are written atomically via a same-directory temp file
//!   (same mount point guarantees rename(2) atomicity).
//! - Corrupt files are renamed to time-stamped backups; at most 5
//!   backups are kept.

use std::{
    fs::{self, File},
    io,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ── error type ────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum PersistenceError {
    #[error("cannot determine home directory")]
    NoHome,
    #[error("data directory is a symlink — refusing (possible attack)")]
    UnsafeSymlink,
    #[error("data directory has unsafe group/other permissions — refusing")]
    UnsafeGroupOrOther,
    #[error("data directory is owned by another user")]
    WrongOwner,
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

// ── data types ────────────────────────────────────────────────────────────

/// A single high-score entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighScore {
    pub name: String,
    pub score: u32,
    pub level: u8,
    pub lines: u32,
    /// Unix timestamp (seconds) when the score was recorded.
    pub ts: u64,
}

/// In-memory store of high scores, ordered descending by `score`.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct HighScoreStore {
    scores: Vec<HighScore>,
}

impl HighScoreStore {
    /// Creates an empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the top `n` scores (already sorted descending).
    pub fn top(&self, n: usize) -> &[HighScore] {
        let end = n.min(self.scores.len());
        &self.scores[..end]
    }

    /// Inserts a score in sorted position.
    ///
    /// Returns `true` if this is a new personal best for the name
    /// (i.e. it is the highest score that name has ever recorded in
    /// this store).
    pub fn insert(&mut self, score: HighScore) -> bool {
        let name = score.name.clone();
        let new_score = score.score;

        // Find previous best for this name.
        let prev_best = self
            .scores
            .iter()
            .filter(|s| s.name == name)
            .map(|s| s.score)
            .max()
            .unwrap_or(0);

        // Insert in descending order.
        let pos = self.scores.partition_point(|s| s.score >= score.score);
        self.scores.insert(pos, score);

        new_score > prev_best
    }
}

// ── directory hardening ───────────────────────────────────────────────────

/// Returns (creating if necessary) the hardened data directory.
///
/// Errors on symlink, world-writable dir, or wrong owner.
pub fn init_data_dir() -> Result<PathBuf, PersistenceError> {
    let dirs = ProjectDirs::from("", "", "blocktxt").ok_or(PersistenceError::NoHome)?;
    let data_dir = dirs.data_local_dir();

    if !data_dir.exists() {
        create_dir_mode_0700(data_dir)?;
        // Re-check after creation to close the TOCTOU window between
        // exists() and create_dir_all — a racing attacker could place a
        // symlink in that gap.
        check_dir_safety(data_dir)?;
        return Ok(data_dir.canonicalize()?);
    }

    // Directory exists — harden-check it.
    check_dir_safety(data_dir)?;
    Ok(data_dir.canonicalize()?)
}

/// Creates a directory (and all parents) with mode 0o700 on Unix.
pub fn create_dir_mode_0700(path: &Path) -> Result<(), PersistenceError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(path)
            .map_err(PersistenceError::Io)
    }
    #[cfg(not(unix))]
    {
        fs::create_dir_all(path).map_err(PersistenceError::Io)
    }
}

/// Checks that `path` is safe to use as a data directory.
///
/// Returns `Ok(())` if and only if:
/// - It is not a symlink.
/// - It is not world-writable (Unix only).
/// - It is owned by the current user (Unix only).
pub fn check_dir_safety(path: &Path) -> Result<(), PersistenceError> {
    // 1. Symlink check — use symlink_metadata so we do NOT follow links.
    let meta = path.symlink_metadata().map_err(PersistenceError::Io)?;

    if meta.file_type().is_symlink() {
        return Err(PersistenceError::UnsafeSymlink);
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;

        // 2. Group+other permissions check (SPEC §4: mode & 0o077 == 0).
        if meta.mode() & 0o077 != 0 {
            return Err(PersistenceError::UnsafeGroupOrOther);
        }

        // 3. Owner check.
        let uid = nix::unistd::getuid().as_raw();
        if meta.uid() != uid {
            return Err(PersistenceError::WrongOwner);
        }
    }

    Ok(())
}

// ── save ──────────────────────────────────────────────────────────────────

/// The canonical file name inside the data directory.
const SCORES_FILE: &str = "scores.json";

/// Returns the canonical path of the scores file given the data dir.
pub fn scores_path(dir: &Path) -> PathBuf {
    dir.join(SCORES_FILE)
}

/// Atomically writes `store` to `<dir>/scores.json`.
///
/// Steps:
/// 1. Create a temp file in the same directory (same mount point).
/// 2. Write JSON.
/// 3. fsync the temp file.
/// 4. Chmod 0o600 (Unix).
/// 5. Atomic rename via `tempfile::persist`.
/// 6. fsync the parent directory (Unix, best-effort).
pub fn save(store: &HighScoreStore, dir: &Path) -> Result<(), PersistenceError> {
    let dest = scores_path(dir);

    // Step 1 — temp file in the same directory.
    let mut tmp = tempfile::NamedTempFile::new_in(dir).map_err(PersistenceError::Io)?;

    // Step 2 — write JSON.
    serde_json::to_writer_pretty(&mut tmp, store).map_err(PersistenceError::Json)?;

    // Step 3 — fsync temp file.
    tmp.as_file().sync_all().map_err(PersistenceError::Io)?;

    // Step 4 — chmod 0o600 (Unix only).
    #[cfg(unix)]
    set_mode_0600(tmp.path())?;

    // Step 5 — atomic rename.
    tmp.persist(&dest)
        .map_err(|e| PersistenceError::Io(e.error))?;

    // Step 6 — fsync parent directory (best-effort on Unix).
    #[cfg(unix)]
    {
        if let Ok(parent) = File::open(dir) {
            let _ = parent.sync_all();
        }
    }

    Ok(())
}

#[cfg(unix)]
fn set_mode_0600(path: &Path) -> Result<(), PersistenceError> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(PersistenceError::Io)
}

// ── load ──────────────────────────────────────────────────────────────────

/// Loads the high-score store from `<dir>/scores.json`.
///
/// If the file does not exist, returns an empty store.
/// If parsing fails, renames the corrupt file to a timestamped backup,
/// prunes backups beyond 5, and returns an empty store.
pub fn load(dir: &Path) -> Result<HighScoreStore, PersistenceError> {
    let path = scores_path(dir);

    if !path.exists() {
        return Ok(HighScoreStore::new());
    }

    let content = fs::read_to_string(&path).map_err(PersistenceError::Io)?;

    match serde_json::from_str::<HighScoreStore>(&content) {
        Ok(store) => Ok(store),
        Err(_) => {
            // Rename the corrupt file to a timestamped backup.
            let backup = unique_corrupt_path(&path);
            if let Err(e) = fs::rename(&path, &backup) {
                eprintln!(
                    "blocktxt: warning: could not rename corrupt \
                     scores file: {e}"
                );
            } else {
                eprintln!(
                    "blocktxt: warning: scores file was corrupt; \
                     renamed to {}",
                    backup.display()
                );
            }

            // Prune old backups — keep at most 5.
            prune_corrupt_backups(&path);

            Ok(HighScoreStore::new())
        }
    }
}

/// Returns a path like `<base>.corrupt.<ts>` that does not already exist.
///
/// Strategy: try `<stem>.<secs>` first; on collision loop with a counter
/// suffix (`<stem>.<secs>-<n>` for n = 1, 2, …) until a unique path is
/// found.  This eliminates the same-nanosecond overwrite window that the
/// old two-attempt scheme left open.
pub fn unique_corrupt_path(base: &Path) -> PathBuf {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();

    // Build the parent stem: "<base>.corrupt"
    let stem = format!("{}.corrupt", base.display());

    let candidate = PathBuf::from(format!("{stem}.{secs}"));
    if !candidate.exists() {
        return candidate;
    }

    // Collision — loop with an incrementing counter suffix.
    let mut counter: u64 = 1;
    loop {
        let candidate = PathBuf::from(format!("{stem}.{secs}-{counter}"));
        if !candidate.exists() {
            return candidate;
        }
        counter += 1;
    }
}

/// Keeps at most 5 `.corrupt.*` backups, deleting the oldest by mtime.
fn prune_corrupt_backups(base: &Path) {
    let dir = match base.parent() {
        Some(d) => d,
        None => return,
    };
    let file_stem = match base.file_name().and_then(|n| n.to_str()) {
        Some(s) => s.to_owned(),
        None => return,
    };
    let prefix = format!("{file_stem}.corrupt.");

    let mut backups: Vec<(std::time::SystemTime, PathBuf)> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            let name = p.file_name()?.to_str()?.to_owned();
            if !name.starts_with(&prefix) {
                return None;
            }
            let mtime = p.metadata().ok()?.modified().ok()?;
            Some((mtime, p))
        })
        .collect();

    if backups.len() <= 5 {
        return;
    }

    // Sort ascending by mtime — oldest first.
    backups.sort_by_key(|(t, _)| *t);

    let to_delete = backups.len() - 5;
    for (_, path) in backups.iter().take(to_delete) {
        let _ = fs::remove_file(path);
    }
}

// ── fallback constructor ──────────────────────────────────────────────────

impl HighScoreStore {
    /// Attempts to load the store; on any error returns an empty store
    /// plus the error so the caller can log a degraded-mode warning.
    pub fn new_with_fallback(
        dir_result: Result<PathBuf, PersistenceError>,
    ) -> (Self, Option<PersistenceError>) {
        match dir_result {
            Err(e) => (Self::new(), Some(e)),
            Ok(dir) => match load(&dir) {
                Ok(store) => (store, None),
                Err(e) => (Self::new(), Some(e)),
            },
        }
    }
}
