//! POSBRD-004: attested last intercept action.
//!
//! `query_status` only reports a last-action when this store was written
//! by a real enforcement outcome. Fences listed on the snapshot are not
//! a substitute.

use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use anvil_intercept_proto::status::LastActionV1;

/// Process-local last attested intercept decision.
#[derive(Debug, Default)]
pub struct LastActionStore {
    inner: Mutex<Option<LastActionV1>>,
}

impl LastActionStore {
    #[must_use]
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(None),
        })
    }

    pub fn record(&self, decision: impl Into<String>, stage: Option<&str>) {
        let observed_at_unix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let action = LastActionV1 {
            decision: decision.into(),
            stage: stage.map(str::to_owned),
            observed_at_unix,
        };
        *self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(action);
    }

    #[must_use]
    pub fn snapshot(&self) -> Option<LastActionV1> {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

#[cfg(test)]
mod last_action_tests {
    use super::LastActionStore;

    #[test]
    fn last_action_store_records_interrupt_stage() {
        let store = LastActionStore::new();
        assert!(store.snapshot().is_none());
        store.record("interrupt", Some("sigkill"));
        let action = store.snapshot().expect("recorded");
        assert_eq!(action.decision, "interrupt");
        assert_eq!(action.stage.as_deref(), Some("sigkill"));
        assert!(action.observed_at_unix > 0);
    }

    #[test]
    fn last_action_store_records_fence_without_stage() {
        let store = LastActionStore::new();
        store.record("fence", None);
        let action = store.snapshot().expect("recorded");
        assert_eq!(action.decision, "fence");
        assert!(action.stage.is_none());
    }
}
