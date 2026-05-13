use std::collections::HashSet;

use crate::verdict::ErrorClass;

/// Deduplication key for [`SuppressionLog`].
///
/// ADR-038 §D-1: "Same class+detail won't re-emit in the same
/// session." A `daemon-down` message should fire once per session,
/// not 82 times during a sub-agent burst. The key is `(ErrorClass,
/// detail)` so two different daemon-down events (e.g. different
/// reasons) can each fire once, but a single class repeats stay
/// silent after the first.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SuppressionKey {
    pub class: ErrorClass,
    /// Free-form detail string. Use `""` when the class itself is
    /// the only differentiator.
    pub detail: String,
}

impl SuppressionKey {
    pub fn new(class: ErrorClass, detail: impl Into<String>) -> Self {
        Self {
            class,
            detail: detail.into(),
        }
    }

    /// Convenience for the common case where the class is the only
    /// differentiator (e.g. a single `DaemonUnreachable` message
    /// per session regardless of which RPC failed).
    pub fn from_class(class: ErrorClass) -> Self {
        Self::new(class, "")
    }
}

/// Per-session suppression record. Constructed once at hook-process
/// startup; consulted before emitting any [`crate::Verdict::InternalError`]
/// stderr line.
///
/// Not thread-safe by design — hook processes are single-threaded
/// and short-lived; if you need cross-process suppression, that
/// belongs in the daemon, not here.
#[derive(Debug, Default)]
pub struct SuppressionLog {
    seen: HashSet<SuppressionKey>,
}

impl SuppressionLog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true the first time `key` is seen and false on every
    /// subsequent call with the same key. Use as:
    ///
    /// ```ignore
    /// if log.should_emit(&key) {
    ///     eprintln!("{}", rendered.stderr_line);
    /// }
    /// ```
    pub fn should_emit(&mut self, key: &SuppressionKey) -> bool {
        // `HashSet::insert` always clones via the owning `K` argument,
        // even when the key is already present. Probe with `contains`
        // first so the clone only happens on the (rare) first-emit
        // path; on the suppressed path we do zero allocations.
        if self.seen.contains(key) {
            false
        } else {
            self.seen.insert(key.clone());
            true
        }
    }

    /// True when the key has been seen at least once.
    pub fn has_seen(&self, key: &SuppressionKey) -> bool {
        self.seen.contains(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_emit_returns_true() {
        let mut log = SuppressionLog::new();
        let key = SuppressionKey::from_class(ErrorClass::DaemonUnreachable);
        assert!(log.should_emit(&key));
    }

    #[test]
    fn repeat_emit_returns_false() {
        let mut log = SuppressionLog::new();
        let key = SuppressionKey::from_class(ErrorClass::DaemonUnreachable);
        assert!(log.should_emit(&key));
        assert!(!log.should_emit(&key));
        assert!(!log.should_emit(&key));
    }

    #[test]
    fn different_classes_are_independent() {
        let mut log = SuppressionLog::new();
        let a = SuppressionKey::from_class(ErrorClass::DaemonUnreachable);
        let b = SuppressionKey::from_class(ErrorClass::Panic);
        assert!(log.should_emit(&a));
        assert!(log.should_emit(&b));
        assert!(!log.should_emit(&a));
        assert!(!log.should_emit(&b));
    }

    #[test]
    fn different_details_within_same_class_are_independent() {
        let mut log = SuppressionLog::new();
        let a = SuppressionKey::new(ErrorClass::DaemonUnreachable, "rpc-timeout");
        let b = SuppressionKey::new(ErrorClass::DaemonUnreachable, "rpc-refused");
        assert!(log.should_emit(&a));
        assert!(log.should_emit(&b));
        assert!(!log.should_emit(&a));
    }

    #[test]
    fn has_seen_does_not_record() {
        let log = SuppressionLog::new();
        let key = SuppressionKey::from_class(ErrorClass::Panic);
        // `has_seen` is read-only; should_emit on next call must
        // still return true because nothing has been recorded.
        assert!(!log.has_seen(&key));
        let mut log = log;
        assert!(log.should_emit(&key));
    }

    #[test]
    fn burst_of_eighty_two_daemon_down_messages_emits_once() {
        // ADR-038: "Daemon-down message fires once per session, not
        // 82 times during a sub-agent burst."
        let mut log = SuppressionLog::new();
        let key = SuppressionKey::from_class(ErrorClass::DaemonUnreachable);
        let mut emit_count = 0;
        for _ in 0..82 {
            if log.should_emit(&key) {
                emit_count += 1;
            }
        }
        assert_eq!(emit_count, 1, "burst must collapse to a single emit");
    }
}
