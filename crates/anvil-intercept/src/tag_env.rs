//! MLP2-025: env reader for `ANVIL_AGENT_TAG`.
//!
//! This module is the **first in-tree reader** of the
//! `ANVIL_AGENT_TAG_ENV` constant declared in
//! [`anvil_intercept_proto::session::ANVIL_AGENT_TAG_ENV`]. It is
//! separate from `crates/anvil-intercept/src/auth.rs` because that
//! module is the DRVR-007 driver-allowlist trust boundary and must not
//! absorb unrelated surfaces (per Planning Council 2026-05-15).
//!
//! Contract for [`env_agent_tag`]:
//!
//! - Returns `None` when the env var is unset, empty, or holds a value
//!   that does not parse as an [`AgentTag`]. Malformed values are
//!   **treated as missing**, mirroring MLP2-025's spoof-cross-check
//!   rule: a tag that the daemon can't validate is not honoured.
//! - Returns `Some(tag)` when the env var carries a well-formed
//!   `AgentTag` JSON value (as produced by
//!   [`anvil_attribution::env::agent_tag_to_env_value`]).
//!
//! The pure helper [`agent_tag_from_env`] takes an `Option<&str>` so
//! tests don't need to mutate the process env (which is global state
//! and flaky under `cargo test`'s parallel runner).

use anvil_attribution::env::agent_tag_from_env_value;
use anvil_intercept_proto::session::{ANVIL_AGENT_TAG_ENV, AgentTag};

/// Read `ANVIL_AGENT_TAG` from the process environment and decode it
/// into an [`AgentTag`]. See module docs for the contract.
#[must_use]
pub fn env_agent_tag() -> Option<AgentTag> {
    let raw = std::env::var(ANVIL_AGENT_TAG_ENV).ok();
    agent_tag_from_env(raw.as_deref())
}

/// Pure helper: decode the given env value (or absence) into an
/// [`AgentTag`]. Folds the three "absent / empty / malformed" cases
/// into a single `None`, leaving `Some` only for fully-parsed tags.
///
/// Exposed for tests; production callers should prefer
/// [`env_agent_tag`].
#[must_use]
pub fn agent_tag_from_env(raw: Option<&str>) -> Option<AgentTag> {
    let value = raw?;
    agent_tag_from_env_value(value).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_tag() -> AgentTag {
        AgentTag::new("anvil-run", "claude-code-2", 1_700_000_042)
    }

    /// A well-formed env value parses into the expected `AgentTag`.
    /// Mirrors the round-trip exercised in
    /// `anvil_attribution::env::tests::round_trip_through_env_value`,
    /// but at the intercept-side reader.
    #[test]
    fn env_agent_tag_parses_valid_value() {
        let encoded = anvil_attribution::env::agent_tag_to_env_value(&sample_tag());
        let decoded = agent_tag_from_env(Some(&encoded)).expect("valid value yields Some");
        assert_eq!(decoded, sample_tag());
    }

    /// A malformed env value is treated as missing, not as an error.
    /// MLP2-025 contract: tags the daemon can't validate are not
    /// honoured, so the spoof-cross-check sees them as `Untagged`.
    #[test]
    fn env_agent_tag_rejects_malformed_value() {
        assert!(agent_tag_from_env(Some("not-json")).is_none());
        assert!(agent_tag_from_env(Some("{\"incomplete\":")).is_none());
    }

    /// An empty env value is treated as missing.
    #[test]
    fn env_agent_tag_treats_empty_as_missing() {
        assert!(agent_tag_from_env(Some("")).is_none());
    }

    /// An absent env var (i.e. the production `std::env::var` would
    /// return `Err(NotPresent)`) is treated as missing.
    #[test]
    fn env_agent_tag_treats_absent_as_missing() {
        assert!(agent_tag_from_env(None).is_none());
    }
}
