//! Rate-limited "update available" hint state (DISTRIB-002).
//!
//! Both `anvil status` and the watch TUI may surface a one-line
//! "update available" hint when an update is detected, but the hint
//! must not nag — the spec caps it at once per 24h. This module owns
//! the small JSON state file that records when the hint last fired, so
//! both surfaces share a single source of truth.
//!
//! State lives at `<state-dir>/anvil/update-hint.json`, matching the
//! convention used by `commands::hook` for the panic log. The file is
//! intentionally tiny and human-readable so an operator can `cat` it
//! while debugging a "why am I not seeing the hint?" report.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// Default rate-limit window for the update-available hint.
///
/// The spec says "rate-limited to once per 24h". Encoded explicitly so
/// tests can substitute a shorter window without going around the
/// production constant.
pub const DEFAULT_HINT_TTL: Duration = Duration::from_secs(24 * 60 * 60);

/// On-disk shape of the hint state. `version` records which release
/// the hint last advertised — when a fresh version appears, the gate
/// fires immediately (the prior `last_shown_at` does not gate a hint
/// for a *different* version).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HintState {
    /// Unix seconds since epoch when the hint last fired. `None` for
    /// "never shown". Stored as an integer to keep the file
    /// human-eyeballable.
    pub last_shown_at: Option<u64>,
    /// The advertised latest version at last firing. `None` when the
    /// hint has never fired or the file was first created without a
    /// version.
    pub last_advertised_version: Option<String>,
}

impl HintState {
    /// Returns true when the hint should fire for `latest_version`
    /// given `now` and the rate-limit `ttl`. Always fires if the
    /// advertised version differs from the last advertised one (so a
    /// fresh release immediately notifies the user without waiting for
    /// the 24h window to expire).
    pub fn should_show(&self, latest_version: &str, now: SystemTime, ttl: Duration) -> bool {
        // Different version → always show, regardless of the timer.
        if self
            .last_advertised_version
            .as_deref()
            .is_none_or(|prev| prev != latest_version)
        {
            return true;
        }
        // Same version → respect the rate limit.
        let Some(last) = self.last_shown_at else {
            return true;
        };
        let now_secs = match now.duration_since(UNIX_EPOCH) {
            Ok(d) => d.as_secs(),
            // Pre-epoch system clock — fire to be safe.
            Err(_) => return true,
        };
        let elapsed = now_secs.saturating_sub(last);
        elapsed >= ttl.as_secs()
    }

    /// Record that the hint fired for `latest_version` at `now`. The
    /// caller is expected to write the result back to disk via
    /// [`write_to`] (typically through [`record_shown_at`]).
    pub fn after_shown(&self, latest_version: &str, now: SystemTime) -> Self {
        let secs = now
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or_default();
        HintState {
            last_shown_at: Some(secs),
            last_advertised_version: Some(latest_version.to_string()),
        }
    }
}

/// Resolve the on-disk state-file path. Returns `None` on the rare
/// platforms where neither `dirs::state_dir` nor `dirs::data_local_dir`
/// is available (mirrors `commands::hook::panic_log_path`).
pub fn state_file_path() -> Option<PathBuf> {
    let base = dirs::state_dir().or_else(dirs::data_local_dir)?;
    Some(base.join("anvil").join("update-hint.json"))
}

/// Read the hint state from `path`. Missing or unreadable files yield
/// `Ok(HintState::default())` — a brand-new install behaves as "never
/// shown". Only true parse errors propagate, so a corrupted file does
/// not silently mask itself as a freshly-installed system.
pub fn read_from(path: &Path) -> std::io::Result<HintState> {
    match std::fs::read_to_string(path) {
        Ok(text) => {
            let state: HintState = serde_json::from_str(&text)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            Ok(state)
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(HintState::default()),
        Err(err) => Err(err),
    }
}

/// Write `state` to `path`, creating the parent directory if needed.
pub fn write_to(path: &Path, state: &HintState) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let body = serde_json::to_string_pretty(state)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(path, body)
}

/// Convenience: load the state file, check if a hint should fire for
/// `latest_version`, and if so record the firing in the file before
/// returning `true`. Used by `anvil status` and the watch TUI to keep
/// rate-limit accounting consistent across surfaces.
///
/// Returns `false` (no hint) on any I/O failure rather than nagging
/// the user — the hint is convenience, not contract.
pub fn record_if_due(path: &Path, latest_version: &str, now: SystemTime, ttl: Duration) -> bool {
    let state = read_from(path).unwrap_or_default();
    if !state.should_show(latest_version, now, ttl) {
        return false;
    }
    let next = state.after_shown(latest_version, now);
    let _ = write_to(path, &next);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(secs: u64) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(secs)
    }

    #[test]
    fn fires_when_never_shown() {
        let state = HintState::default();
        assert!(state.should_show("0.7.0-beta", at(0), DEFAULT_HINT_TTL));
    }

    #[test]
    fn rate_limit_blocks_same_version_within_window() {
        let state = HintState {
            last_shown_at: Some(1_000_000),
            last_advertised_version: Some("0.7.0-beta".into()),
        };
        // 23 hours later — within the 24h window.
        let now = at(1_000_000 + 23 * 60 * 60);
        assert!(!state.should_show("0.7.0-beta", now, DEFAULT_HINT_TTL));
    }

    #[test]
    fn rate_limit_releases_at_24_hours() {
        let state = HintState {
            last_shown_at: Some(1_000_000),
            last_advertised_version: Some("0.7.0-beta".into()),
        };
        let now = at(1_000_000 + 24 * 60 * 60);
        assert!(state.should_show("0.7.0-beta", now, DEFAULT_HINT_TTL));
    }

    #[test]
    fn new_version_bypasses_rate_limit() {
        // A fresh release within the 24h window must surface
        // immediately — users would otherwise miss a hotfix because
        // they saw an older "update available" 23 hours ago.
        let state = HintState {
            last_shown_at: Some(1_000_000),
            last_advertised_version: Some("0.7.0-beta".into()),
        };
        let now = at(1_000_000 + 60); // one minute later
        assert!(state.should_show("0.7.1-beta", now, DEFAULT_HINT_TTL));
    }

    #[test]
    fn record_if_due_writes_state_and_returns_true_first_time() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("update-hint.json");
        let now = at(2_000_000);
        let fired = record_if_due(&path, "0.7.0-beta", now, DEFAULT_HINT_TTL);
        assert!(fired);
        let state = read_from(&path).unwrap();
        assert_eq!(state.last_advertised_version.as_deref(), Some("0.7.0-beta"));
        assert_eq!(state.last_shown_at, Some(2_000_000));
    }

    #[test]
    fn record_if_due_is_idempotent_within_window() {
        // The DISTRIB-002 spec validation test
        // (`watch::tests::update_hint_rate_limited`) lives in the TUI
        // crate; here we assert the underlying primitive: two calls
        // within the window only fire once.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("update-hint.json");
        let now = at(2_000_000);
        assert!(record_if_due(&path, "0.7.0-beta", now, DEFAULT_HINT_TTL));
        let later = at(2_000_000 + 60 * 60); // 1h later
        assert!(!record_if_due(&path, "0.7.0-beta", later, DEFAULT_HINT_TTL));
    }

    #[test]
    fn record_if_due_fires_again_after_ttl() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("update-hint.json");
        let now = at(2_000_000);
        assert!(record_if_due(&path, "0.7.0-beta", now, DEFAULT_HINT_TTL));
        let later = at(2_000_000 + 24 * 60 * 60);
        assert!(record_if_due(&path, "0.7.0-beta", later, DEFAULT_HINT_TTL));
    }

    #[test]
    fn read_from_missing_file_is_default_not_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("does-not-exist.json");
        let state = read_from(&path).unwrap();
        assert_eq!(state, HintState::default());
    }

    #[test]
    fn corrupted_file_propagates_as_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("update-hint.json");
        std::fs::write(&path, "{not json").unwrap();
        let err = read_from(&path).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    }
}
