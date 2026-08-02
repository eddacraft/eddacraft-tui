//! KFIT-006 disabled consumption seam for the embedded kindling runtime.
//!
//! This module deliberately prepares configuration only. It does not call
//! [`kindling_runtime::Runtime::start`], so enabling the compile-time feature
//! and rollout flag still cannot start or attach to a daemon in this slice.

use std::path::PathBuf;

use ::kindling_runtime::{RuntimeConfig, SpawnStrategy};

/// Prepare the embedded runtime configuration when the rollout gate allows it.
///
/// Returning `None` is the normal production result while the gate remains
/// default-off. A returned configuration is inert until the later KFIT-006
/// activation slice explicitly passes it to `Runtime::start`.
pub(crate) fn prepare(
    project_root: String,
    kindling_home: PathBuf,
    spool_path: PathBuf,
) -> Option<RuntimeConfig> {
    if !crate::feature_flags::kindling_embedded_runtime_enabled() {
        return None;
    }

    let mut config = RuntimeConfig::with_home(kindling_home, project_root, SpawnStrategy::Embedded);
    config.spool_path = Some(spool_path);
    Some(config)
}

// Keep the disabled seam compile-checked in non-test feature builds without
// adding a runtime caller before KFIT-005 publishes the approved dependency.
const _: fn(String, PathBuf, PathBuf) -> Option<RuntimeConfig> = prepare;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_off_preparation_does_not_create_runtime_state() {
        let base = tempfile::tempdir().expect("temp runtime root");
        let home = base.path().join("kindling");
        let spool = base.path().join("spool.ndjson");

        temp_env::with_var(
            crate::feature_flags::KINDLING_EMBEDDED_RUNTIME_ENV_VAR,
            None::<&str>,
            || {
                assert!(prepare("/repo/anvil".to_owned(), home.clone(), spool).is_none());
            },
        );

        assert!(
            !home.exists(),
            "preparing the default-off seam must not create kindling state"
        );
    }

    #[test]
    fn explicit_opt_in_only_prepares_embedded_configuration() {
        let base = tempfile::tempdir().expect("temp runtime root");
        let home = base.path().join("kindling");
        let spool = base.path().join("spool.ndjson");

        let config = temp_env::with_var(
            crate::feature_flags::KINDLING_EMBEDDED_RUNTIME_ENV_VAR,
            Some("1"),
            || prepare("/repo/anvil".to_owned(), home.clone(), spool.clone()),
        )
        .expect("explicit opt-in prepares a config");

        assert_eq!(config.project_root, "/repo/anvil");
        assert_eq!(config.kindling_home, home);
        assert_eq!(config.spool_path.as_deref(), Some(spool.as_path()));
        assert_eq!(config.spawn, SpawnStrategy::Embedded);
        assert!(
            !config.kindling_home.exists(),
            "configuration preparation must not start the daemon"
        );
    }
}
