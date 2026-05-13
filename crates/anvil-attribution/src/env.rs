//! Env-var encoding for [`AgentTag`] + [`ANVIL_TASK_ID_ENV`].
//!
//! The on-wire encoding is the same compact JSON the proto crate's
//! `AgentTag` already serialises to (sorted-key-stable via `serde_json`
//! deterministic key ordering for plain structs). Keeping it as JSON
//! rather than a custom field-separated format means the launcher can
//! emit it with `serde_json::to_string` and the child can parse it
//! with `serde_json::from_str` — no parser to keep in sync.
//!
//! Linux env vars are byte strings without embedded NULs. JSON
//! serialisation of [`AgentTag`] never emits NULs (`driver_id` and
//! `claimed_agent_id` round-trip through `to_string` regardless of
//! content) — but the decoder still defends against malformed input
//! by returning a typed error rather than panicking.

use std::process::Command;

use anvil_intercept_proto::session::{ANVIL_AGENT_TAG_ENV, ANVIL_TASK_ID_ENV, AgentTag};
use thiserror::Error;

/// Failures from [`agent_tag_from_env_value`]. Distinct from raw
/// `serde_json::Error` so callers can render a stable diagnostic
/// without depending on the serde-json error type's `Display`.
#[derive(Debug, Error)]
pub enum ParseAgentTagError {
    /// Value was empty — treat as if the env var was unset.
    #[error("ANVIL_AGENT_TAG was empty")]
    Empty,

    /// JSON did not decode into an `AgentTag`. The wrapped message is
    /// the underlying serde-json `Display` text.
    #[error("ANVIL_AGENT_TAG could not be parsed as AgentTag: {0}")]
    Malformed(String),
}

/// Encode an [`AgentTag`] for the `ANVIL_AGENT_TAG` env var.
///
/// Always succeeds — the proto type's `Serialize` impl is total.
#[must_use]
pub fn agent_tag_to_env_value(tag: &AgentTag) -> String {
    // `serde_json::to_string` can only fail on Serialize impls that
    // return errors. `AgentTag` is a plain struct of `String` + `u64`,
    // so the call is infallible in practice; we unwrap rather than
    // propagate a phantom error.
    serde_json::to_string(tag).expect("AgentTag Serialize is infallible")
}

/// Decode an `ANVIL_AGENT_TAG` env-var payload back into an
/// [`AgentTag`]. Returns [`ParseAgentTagError::Empty`] for empty input
/// so callers can treat an empty env var the same as an unset env var.
pub fn agent_tag_from_env_value(raw: &str) -> Result<AgentTag, ParseAgentTagError> {
    if raw.is_empty() {
        return Err(ParseAgentTagError::Empty);
    }
    serde_json::from_str::<AgentTag>(raw).map_err(|e| ParseAgentTagError::Malformed(e.to_string()))
}

/// Set `ANVIL_AGENT_TAG` + `ANVIL_TASK_ID` on `cmd` so the child
/// process inherits attribution.
///
/// Callers are free to set additional env vars before or after; this
/// helper does not clear the existing environment.
pub fn set_attribution_env(cmd: &mut Command, tag: &AgentTag, task_id: &str) {
    cmd.env(ANVIL_AGENT_TAG_ENV, agent_tag_to_env_value(tag));
    cmd.env(ANVIL_TASK_ID_ENV, task_id);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_tag() -> AgentTag {
        AgentTag::new("anvil-run", "claude-code-7", 1_700_000_042)
    }

    #[test]
    fn round_trip_through_env_value() {
        let tag = sample_tag();
        let encoded = agent_tag_to_env_value(&tag);
        let decoded = agent_tag_from_env_value(&encoded).expect("decode");
        assert_eq!(decoded, tag);
    }

    #[test]
    fn empty_value_returns_empty_variant() {
        match agent_tag_from_env_value("") {
            Err(ParseAgentTagError::Empty) => {}
            other => panic!("expected Empty, got {other:?}"),
        }
    }

    #[test]
    fn malformed_value_returns_malformed_variant() {
        match agent_tag_from_env_value("not-json") {
            Err(ParseAgentTagError::Malformed(_)) => {}
            other => panic!("expected Malformed, got {other:?}"),
        }
    }

    #[test]
    fn malformed_value_carries_serde_message() {
        let err = agent_tag_from_env_value("not-json").unwrap_err();
        assert!(
            err.to_string().contains("could not be parsed"),
            "diagnostic should mention parse failure, got: {err}"
        );
    }

    #[test]
    fn set_attribution_env_populates_both_vars() {
        let mut cmd = Command::new("true");
        set_attribution_env(&mut cmd, &sample_tag(), "task-42");

        let envs: std::collections::HashMap<_, _> = cmd
            .get_envs()
            .filter_map(|(k, v)| v.map(|v| (k.to_owned(), v.to_owned())))
            .collect();

        let tag_val = envs
            .get(std::ffi::OsStr::new(ANVIL_AGENT_TAG_ENV))
            .expect("ANVIL_AGENT_TAG set");
        let task_val = envs
            .get(std::ffi::OsStr::new(ANVIL_TASK_ID_ENV))
            .expect("ANVIL_TASK_ID set");

        assert_eq!(task_val.to_string_lossy(), "task-42");
        let decoded = agent_tag_from_env_value(&tag_val.to_string_lossy()).expect("decode");
        assert_eq!(decoded, sample_tag());
    }

    #[test]
    fn driver_id_with_spaces_round_trips() {
        let tag = AgentTag::new("anvil run (debug)", "claude code 9", 42);
        let encoded = agent_tag_to_env_value(&tag);
        let decoded = agent_tag_from_env_value(&encoded).expect("decode");
        assert_eq!(decoded, tag);
    }
}
