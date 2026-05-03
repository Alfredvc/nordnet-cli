//! On-disk session persistence for authenticated `nordnet auth` commands.
//!
//! After `nordnet auth login`, the resolved [`nordnet_model::auth::Session`]
//! is serialized to TOML at the [default path](`default_session_path`)
//! (`<config_dir>/nordnet/session.toml`, e.g. `~/.config/nordnet/session.toml`
//! on Linux). Subsequent authenticated commands read this file to attach
//! the `Authorization` header without re-running the challenge flow.
//!
//! # File permissions
//!
//! On Unix the file is written with mode `0600` (owner read/write only).
//! On other platforms no perm hardening is applied — the OS file ACL is
//! used as-is. The path lives next to `credentials.toml` because both
//! files contain secrets that should be treated identically.
//!
//! # Override order
//!
//! The CLI resolves the session in this order (first hit wins):
//!
//! 1. `--session-key <KEY>` global flag (highest priority; one-off use).
//! 2. `NORDNET_SESSION_KEY` environment variable.
//! 3. The session file written by the last `nordnet auth login`.
//!
//! Authenticated commands fail with [`SessionError::Missing`] if none of
//! the three is available.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;
use time::OffsetDateTime;

/// Persisted session record. Wire form is TOML; fields mirror the
/// `nordnet_model::auth::Session` plus the local acquisition timestamp,
/// which lets `nordnet auth status` compute remaining time without a
/// network call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredSession {
    /// `session_key` returned by `POST /login/verify`.
    pub session_key: String,
    /// Server-reported lifetime in seconds, captured from the
    /// `expires_in` field of the original `POST /login/verify` response.
    /// `PUT /login` (refresh) does not return a new `expires_in`, so this
    /// value is treated as the canonical session lifetime; refreshes
    /// reset [`StoredSession::acquired_at`] but leave this field
    /// unchanged. See `cmd::auth::Cmd::Refresh` for the assumption this
    /// rests on.
    pub expires_in: i64,
    /// Wall-clock UTC time when the session was acquired.
    /// Used by `nordnet auth status` to compute remaining time locally.
    #[serde(with = "time::serde::rfc3339")]
    pub acquired_at: OffsetDateTime,
}

impl StoredSession {
    /// Compute the projected expiry time (`acquired_at + expires_in`)
    /// without consulting the server.
    pub fn expires_at(&self) -> OffsetDateTime {
        self.acquired_at + time::Duration::seconds(self.expires_in)
    }

    /// Convert to the `nordnet_model` session shape the HTTP client
    /// expects.
    pub fn to_api_session(&self) -> nordnet_model::auth::Session {
        nordnet_model::auth::Session {
            session_key: self.session_key.clone(),
            expires_in: self.expires_in,
        }
    }
}

/// Resolve the default on-disk session path. Returns `None` if no
/// config home can be determined (extremely rare; usually only in
/// stripped CI containers without `$HOME`).
pub fn default_session_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("nordnet").join("session.toml"))
}

/// Read the session at the default path, returning `Ok(None)` if the
/// file does not exist.
pub fn load() -> Result<Option<StoredSession>, SessionError> {
    let path = default_session_path().ok_or(SessionError::NoConfigHome)?;
    load_from(&path)
}

/// Read the session at an explicit path, returning `Ok(None)` if the
/// file does not exist.
pub fn load_from(path: &Path) -> Result<Option<StoredSession>, SessionError> {
    if !path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(path).map_err(|e| SessionError::Io(path.to_path_buf(), e))?;
    let session: StoredSession =
        toml::from_str(&raw).map_err(|e| SessionError::Toml(path.to_path_buf(), e.to_string()))?;
    Ok(Some(session))
}

/// Write the session to the default path. Creates parent directories as
/// needed. On Unix, sets file mode to `0600`.
pub fn save(session: &StoredSession) -> Result<PathBuf, SessionError> {
    let path = default_session_path().ok_or(SessionError::NoConfigHome)?;
    save_to(&path, session)?;
    Ok(path)
}

/// Write the session to an explicit path. Creates parent directories as
/// needed. On Unix, the file is created with mode `0600` *before* any
/// secret bytes are written, then atomically renamed over the destination
/// — there is no window during which another local user can read a
/// world-readable copy of the session key.
pub fn save_to(path: &Path, session: &StoredSession) -> Result<(), SessionError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| SessionError::Io(parent.to_path_buf(), e))?;
    }
    let serialized = toml::to_string(session)
        .map_err(|e| SessionError::Encode(path.to_path_buf(), e.to_string()))?;
    write_atomic_owner_only(path, serialized.as_bytes())
}

/// Write `bytes` to `dest` such that, on Unix, no intermediate state on
/// disk is readable by anyone other than the owner: a sibling tempfile is
/// created with mode `0600` via `OpenOptions::mode`, the bytes are
/// written and `fsync`'d, then the tempfile is `rename`'d over `dest`
/// (atomic on POSIX). On non-Unix the call falls back to `std::fs::write`
/// — perm hardening on those platforms is the OS's responsibility.
#[cfg(unix)]
fn write_atomic_owner_only(dest: &Path, bytes: &[u8]) -> Result<(), SessionError> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    use std::time::{SystemTime, UNIX_EPOCH};

    let parent = dest
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| Path::new(".").to_path_buf());
    let stem = dest
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("session");
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp = parent.join(format!(
        ".{stem}.tmp.{pid}.{tid:?}.{nanos}",
        pid = std::process::id(),
        tid = std::thread::current().id(),
    ));

    let result = (|| -> Result<(), SessionError> {
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&tmp)
            .map_err(|e| SessionError::Io(tmp.clone(), e))?;
        f.write_all(bytes)
            .map_err(|e| SessionError::Io(tmp.clone(), e))?;
        f.sync_all().map_err(|e| SessionError::Io(tmp.clone(), e))?;
        std::fs::rename(&tmp, dest).map_err(|e| SessionError::Io(dest.to_path_buf(), e))?;
        Ok(())
    })();

    if result.is_err() {
        // Best-effort cleanup; ignore the result because the original
        // error is already what the caller needs to see.
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

#[cfg(not(unix))]
fn write_atomic_owner_only(dest: &Path, bytes: &[u8]) -> Result<(), SessionError> {
    std::fs::write(dest, bytes).map_err(|e| SessionError::Io(dest.to_path_buf(), e))
}

/// Delete the session file at the default path. Returns `Ok(false)` if
/// the file did not exist.
pub fn delete() -> Result<bool, SessionError> {
    let path = default_session_path().ok_or(SessionError::NoConfigHome)?;
    delete_at(&path)
}

/// Delete the session file at an explicit path. Returns `Ok(false)` if
/// the file did not exist.
pub fn delete_at(path: &Path) -> Result<bool, SessionError> {
    if !path.exists() {
        return Ok(false);
    }
    std::fs::remove_file(path).map_err(|e| SessionError::Io(path.to_path_buf(), e))?;
    Ok(true)
}

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("no configuration home directory available (HOME / XDG_CONFIG_HOME unset)")]
    NoConfigHome,
    #[error("session not available — run `nordnet auth login`, set NORDNET_SESSION_KEY, or pass --session-key")]
    Missing,
    #[error("could not access {0}: {1}")]
    Io(PathBuf, std::io::Error),
    #[error("could not parse session file {0} as TOML: {1}")]
    Toml(PathBuf, String),
    #[error("could not encode session file {0} as TOML: {1}")]
    Encode(PathBuf, String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_path(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "nordnet-cli-session-test-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    #[test]
    fn save_load_roundtrip() {
        let path = tmp_path("session_roundtrip.toml");
        let session = StoredSession {
            session_key: "sk-abc".into(),
            expires_in: 600,
            acquired_at: OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap(),
        };
        save_to(&path, &session).unwrap();
        let loaded = load_from(&path).unwrap().unwrap();
        assert_eq!(loaded, session);
    }

    #[test]
    fn load_missing_file_returns_none() {
        let path = tmp_path("does_not_exist.toml");
        if path.exists() {
            std::fs::remove_file(&path).unwrap();
        }
        let r = load_from(&path).unwrap();
        assert!(r.is_none());
    }

    #[test]
    fn delete_missing_file_returns_false() {
        let path = tmp_path("delete_missing.toml");
        if path.exists() {
            std::fs::remove_file(&path).unwrap();
        }
        assert!(!delete_at(&path).unwrap());
    }

    #[test]
    fn delete_existing_file_returns_true_then_removes() {
        let path = tmp_path("delete_existing.toml");
        let session = StoredSession {
            session_key: "x".into(),
            expires_in: 60,
            acquired_at: OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap(),
        };
        save_to(&path, &session).unwrap();
        assert!(delete_at(&path).unwrap());
        assert!(!path.exists());
    }

    #[cfg(unix)]
    #[test]
    fn save_sets_owner_only_mode_on_unix() {
        use std::os::unix::fs::PermissionsExt;
        let path = tmp_path("session_perms.toml");
        let session = StoredSession {
            session_key: "sk".into(),
            expires_in: 1,
            acquired_at: OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap(),
        };
        save_to(&path, &session).unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600, "expected mode 0600, got {mode:o}");
    }

    /// Pre-existing world-readable file gets atomically replaced with a
    /// new 0600 file. Proves the rename pattern brings the new file's mode
    /// rather than mutating the old file's perms in place (which would
    /// leave a race window).
    #[cfg(unix)]
    #[test]
    fn save_overwrites_loose_perms_atomically() {
        use std::os::unix::fs::PermissionsExt;
        let path = tmp_path("session_perms_atomic.toml");
        // Pre-create with world-readable perms.
        std::fs::write(&path, "stale").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o644
        );

        let session = StoredSession {
            session_key: "sk".into(),
            expires_in: 1,
            acquired_at: OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap(),
        };
        save_to(&path, &session).unwrap();

        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(
            mode, 0o600,
            "expected mode 0600 after overwrite, got {mode:o}"
        );
        // Verify the new content actually landed (not the "stale" string).
        let loaded = load_from(&path).unwrap().unwrap();
        assert_eq!(loaded.session_key, "sk");
    }

    /// No `.tmp.*` sidecar file should remain in the destination directory
    /// after a successful save — proves the rename completed and cleanup
    /// is unnecessary on the happy path.
    #[cfg(unix)]
    #[test]
    fn save_leaves_no_tempfile_behind() {
        let path = tmp_path("session_no_tempfile.toml");
        let dir = path.parent().unwrap().to_path_buf();
        let session = StoredSession {
            session_key: "sk".into(),
            expires_in: 1,
            acquired_at: OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap(),
        };
        save_to(&path, &session).unwrap();
        let leftovers: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name())
            .filter(|n| n.to_string_lossy().contains(".tmp."))
            .collect();
        assert!(leftovers.is_empty(), "tempfiles left behind: {leftovers:?}");
    }

    #[test]
    fn expires_at_adds_expires_in_seconds() {
        let acquired = OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap();
        let s = StoredSession {
            session_key: "x".into(),
            expires_in: 90,
            acquired_at: acquired,
        };
        assert_eq!(s.expires_at(), acquired + time::Duration::seconds(90));
    }
}
