use std::path::Path;
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub use anvil_kernel_types::diagnostics::ControlDecision;
use anvil_kernel_types::{
    Notification, NotificationClass, NotificationContext, NotificationPriority,
};
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

use crate::enforcement::EnforcementDecision;

pub const NOTIFICATION_SCHEMA: &str = "anvil.notification.v1";
pub const INTERCEPT_SOURCE: &str = "intercept";
pub const INTERCEPT_DRIVER_ID: &str = "intercept-daemon-v1";

/// MLP2-025b: reason string emitted on the notification envelope AND
/// in `tracing::warn!` calls when the daemon control-lane detects an
/// out-of-lineage env-tag forgery and blocks the write.
///
/// Defined as a `pub const` (not an enum) so a future migration to a
/// typed degraded-mode enum has a single find-target. See
/// `plans/specs/2026-05-16-mlp2-025-spoof-cross-check-control-lane.md`
/// §8.
pub const DEGRADED_SPOOFED_ATTRIBUTION: &str = "degraded:spoofed-attribution";

/// MLP2-026: reason string emitted when the daemon's fence rate
/// window detects 5 fence fires within 60 seconds for the same
/// worktree, engaging `degraded:fence-cascade` mode. Emitted via
/// both the notification envelope (ActiveToFenced) AND
/// `tracing::warn!`. Defined as a `pub const` so a future migration
/// to a typed degraded-mode enum has a single find-target. See
/// `plans/specs/2026-05-16-mlp2-026-fence-cascade-control-lane.md`
/// §8.
pub const DEGRADED_FENCE_CASCADE: &str = "degraded:fence-cascade";

/// MLP2-026: paired clear reason emitted when an operator clears
/// a `degraded:fence-cascade` via `anvil intercept unblock
/// --acknowledge-cascade`. Emitted via the notification envelope
/// (FencedToActive) AND `tracing::info!` — mirrors the priority
/// asymmetry from the existing `FenceTransition` mapping
/// (Critical on engage, Normal on clear).
pub const DEGRADED_FENCE_CASCADE_CLEAR: &str = "degraded:fence-cascade-clear";

static PRODUCER_INSTANCE_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TelemetryCorrelation {
    pub session_id: Option<String>,
    pub worktree: Option<String>,
    pub run_id: Option<String>,
    pub control_correlation_id: Option<String>,
    /// INTD-015: identifier of the session that produced this event.
    /// When set, the fanout (`crates/anvil-intercept/src/fanout.rs`)
    /// uses this — NOT `session_id` — to enforce cross-session
    /// redaction. `session_id` describes "the session this envelope
    /// is *about* in the operator-visible telemetry sense";
    /// `originating_session_id` is the **load-bearing scoping key**
    /// that the fanout reads to decide redaction. They are equal in
    /// the common case; carrying both lets future producers (e.g. a
    /// driver that proxies an event for another session) keep the
    /// scoping invariant explicit.
    pub originating_session_id: Option<String>,
    /// INTD-015: stable identity of the driver that produced this
    /// event, in the daemon's view. The daemon mints this from the
    /// connection's socket-peer credentials (UID + binary path /
    /// install-time token) — **not** from a driver-supplied
    /// `driverName`. The fanout uses this for telemetry rate-limiting
    /// and quarantine; subscribers see it for diagnostic purposes.
    pub originating_driver_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TelemetryEmitter {
    producer_instance_id: String,
    next_seq: u64,
    timestamp_override: Option<String>,
}

impl TelemetryEmitter {
    #[must_use]
    pub fn new() -> Self {
        Self {
            producer_instance_id: generate_producer_instance_id(),
            next_seq: 1,
            timestamp_override: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn for_tests(
        producer_instance_id: impl Into<String>,
        timestamp: impl Into<String>,
    ) -> Self {
        Self {
            producer_instance_id: producer_instance_id.into(),
            next_seq: 1,
            timestamp_override: Some(timestamp.into()),
        }
    }

    #[must_use]
    pub fn producer_instance_id(&self) -> &str {
        &self.producer_instance_id
    }

    /// Convenience builder used by callers that have a single
    /// session+driver pair and want the `originating_*` fields
    /// populated without naming each one. The fanout (INTD-015)
    /// reads `originating_session_id` regardless of whether the
    /// caller used this helper or set the field directly.
    pub fn delivered_envelope_for_decision_from(
        &mut self,
        session_id: impl Into<String>,
        driver_id: impl Into<String>,
        decision: &EnforcementDecision,
    ) -> NotificationEnvelope {
        let session_id = session_id.into();
        let correlation = TelemetryCorrelation {
            session_id: Some(session_id.clone()),
            originating_session_id: Some(session_id),
            originating_driver_id: Some(driver_id.into()),
            ..TelemetryCorrelation::default()
        };
        self.delivered_envelope_for_decision(correlation, decision)
    }

    pub fn delivered_envelope_for_decision(
        &mut self,
        correlation: TelemetryCorrelation,
        decision: &EnforcementDecision,
    ) -> NotificationEnvelope {
        let context = self.next_context(correlation);
        delivered_envelope_for_decision(&context, decision)
    }

    pub fn failed_send_health_envelope(
        &mut self,
        mut correlation: TelemetryCorrelation,
        attempted_decision: ControlDecision,
        control_correlation_id: impl Into<String>,
        message: impl Into<String>,
    ) -> NotificationEnvelope {
        correlation.control_correlation_id = Some(control_correlation_id.into());
        let context = self.next_context(correlation);
        let notification = Notification::new(
            NotificationClass::Health,
            NotificationPriority::High,
            "control-lane delivery failed",
            message,
        )
        .with_context(notification_context(None));

        envelope(&context, Some(attempted_decision), notification, None)
    }

    pub fn envelope_for_fence_transition(
        &mut self,
        mut correlation: TelemetryCorrelation,
        worktree: &Path,
        transition: FenceTransition,
    ) -> NotificationEnvelope {
        correlation.worktree = Some(worktree.display().to_string());
        let context = self.next_context(correlation);
        envelope_for_fence_transition(&context, worktree, transition)
    }

    fn next_context(&mut self, correlation: TelemetryCorrelation) -> TelemetryContext {
        let seq = self.next_seq;
        self.next_seq = self.next_seq.saturating_add(1);
        // INTD-015: when only one of `session_id` / `originating_session_id`
        // is set, mirror the value across both so subscribers and the
        // fanout always see a consistent originator. This preserves
        // backwards-compat with pre-INTD-015 callers (which only set
        // `session_id`) without weakening the scoping check — the
        // mirroring goes one direction only, and only when no
        // explicit originator is provided.
        let originating_session_id = correlation
            .originating_session_id
            .clone()
            .or_else(|| correlation.session_id.clone());
        TelemetryContext {
            producer_instance_id: self.producer_instance_id.clone(),
            seq,
            timestamp: self.timestamp(),
            session_id: correlation.session_id,
            worktree: correlation.worktree,
            run_id: correlation.run_id,
            control_correlation_id: correlation.control_correlation_id,
            originating_session_id,
            originating_driver_id: correlation.originating_driver_id,
        }
    }

    fn timestamp(&self) -> String {
        if let Some(timestamp) = &self.timestamp_override {
            return timestamp.clone();
        }
        Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
    }
}

impl Default for TelemetryEmitter {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TelemetryContext {
    producer_instance_id: String,
    seq: u64,
    timestamp: String,
    session_id: Option<String>,
    worktree: Option<String>,
    run_id: Option<String>,
    control_correlation_id: Option<String>,
    originating_session_id: Option<String>,
    originating_driver_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationEnvelope {
    pub schema: String,
    pub producer_instance_id: String,
    pub seq: u64,
    pub timestamp: String,
    pub correlation: NotificationCorrelation,
    pub notification: Notification,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grouping: Option<NotificationGrouping>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mirror: Option<NotificationMirror>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationCorrelation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    pub source: String,
    /// INTD-015: identifier of the session that originated this
    /// event. The fanout (`fanout.rs`) reads this to enforce
    /// cross-session subscription redaction; subscribers MUST treat
    /// unknown / absent values as "not authorised" (default deny).
    /// Distinct from [`session_id`] (operator-visible context) —
    /// both fields exist so future producers that proxy events for
    /// another session keep the scoping invariant explicit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub originating_session_id: Option<String>,
    /// INTD-015: stable identity of the driver that produced the
    /// event. Minted by the daemon from socket-peer credentials —
    /// **not** from a driver-supplied `driverName` — so a hostile
    /// same-UID peer cannot impersonate another driver by self-
    /// declaring a name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub originating_driver_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationGrouping {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transition: Option<NotificationTransition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationTransition {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationMirror {
    pub decision: ControlDecision,
    pub driver: String,
    pub ack_required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_correlation_id: Option<String>,
}

#[must_use]
pub fn notification_mapping(
    decision: ControlDecision,
) -> (NotificationClass, NotificationPriority) {
    match decision {
        ControlDecision::Allow => (NotificationClass::Info, NotificationPriority::Low),
        ControlDecision::Warn => (NotificationClass::Warning, NotificationPriority::High),
        ControlDecision::Block => (NotificationClass::Block, NotificationPriority::Critical),
        ControlDecision::Interrupt => {
            (NotificationClass::Interrupt, NotificationPriority::Critical)
        }
    }
}

fn ack_required(decision: ControlDecision) -> bool {
    matches!(
        decision,
        ControlDecision::Block | ControlDecision::Interrupt
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FenceTransition {
    ActiveToFenced,
    FencedToActive,
}

#[must_use]
pub(crate) fn delivered_envelope_for_decision(
    context: &TelemetryContext,
    decision: &EnforcementDecision,
) -> NotificationEnvelope {
    let control = control_decision_for(decision);
    let (class, priority) = notification_mapping(control);
    let affected_path = first_affected_path(decision);
    let notification = match decision {
        EnforcementDecision::Allow { .. } => Notification::new(
            class,
            priority,
            "allowed",
            "intercept enforcement allowed the change",
        ),
        EnforcementDecision::Interrupt(interrupt) => Notification::new(
            class,
            priority,
            interrupt.rule_id.clone(),
            interrupt.message.clone(),
        ),
    }
    .with_context(notification_context(affected_path.as_ref()));

    envelope(
        context,
        Some(control),
        notification,
        grouping_for_path(affected_path.as_ref()),
    )
}

#[must_use]
pub(crate) fn envelope_for_fence_transition(
    context: &TelemetryContext,
    worktree: &Path,
    transition: FenceTransition,
) -> NotificationEnvelope {
    let (from, to, priority, message) = match transition {
        FenceTransition::ActiveToFenced => (
            "active",
            "fenced",
            NotificationPriority::Critical,
            "intercept fenced the worktree",
        ),
        FenceTransition::FencedToActive => (
            "fenced",
            "active",
            NotificationPriority::Normal,
            "intercept unblocked the worktree",
        ),
    };
    let notification = Notification::new(
        NotificationClass::FenceState,
        priority,
        "worktree fence state changed",
        message,
    )
    .with_context(notification_context(None));
    let grouping = NotificationGrouping {
        key: Some(format!("intercept:fence:{}", worktree.display())),
        transition: Some(NotificationTransition {
            from: from.to_string(),
            to: to.to_string(),
        }),
    };

    envelope(context, None, notification, Some(grouping))
}

fn envelope(
    context: &TelemetryContext,
    decision: Option<ControlDecision>,
    notification: Notification,
    grouping: Option<NotificationGrouping>,
) -> NotificationEnvelope {
    let mirror = decision.map(|decision| NotificationMirror {
        decision,
        driver: INTERCEPT_DRIVER_ID.to_string(),
        ack_required: ack_required(decision),
        control_correlation_id: context.control_correlation_id.clone(),
    });

    NotificationEnvelope {
        schema: NOTIFICATION_SCHEMA.to_string(),
        producer_instance_id: context.producer_instance_id.clone(),
        seq: context.seq,
        timestamp: context.timestamp.clone(),
        correlation: NotificationCorrelation {
            session_id: context.session_id.clone(),
            worktree: context.worktree.clone(),
            run_id: context.run_id.clone(),
            source: INTERCEPT_SOURCE.to_string(),
            originating_session_id: context.originating_session_id.clone(),
            originating_driver_id: context.originating_driver_id.clone(),
        },
        notification,
        grouping,
        mirror,
    }
}

fn control_decision_for(decision: &EnforcementDecision) -> ControlDecision {
    match decision {
        EnforcementDecision::Allow { .. } => ControlDecision::Allow,
        EnforcementDecision::Interrupt(_) => ControlDecision::Interrupt,
    }
}

fn first_affected_path(decision: &EnforcementDecision) -> Option<String> {
    let paths = match decision {
        EnforcementDecision::Allow { affected_paths } => affected_paths,
        EnforcementDecision::Interrupt(interrupt) => &interrupt.affected_paths,
    };
    paths.first().map(|path| path.display().to_string())
}

fn notification_context(file: Option<&String>) -> NotificationContext {
    NotificationContext {
        file: file.cloned(),
        source: Some(INTERCEPT_SOURCE.to_string()),
    }
}

fn grouping_for_path(path: Option<&String>) -> Option<NotificationGrouping> {
    path.map(|path| NotificationGrouping {
        key: Some(format!("intercept:decision:{path}")),
        transition: None,
    })
}

fn generate_producer_instance_id() -> String {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map_or_else(
        |error| error.duration().as_nanos(),
        |duration| duration.as_nanos(),
    );
    let counter = PRODUCER_INSTANCE_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("pi_{:x}_{nanos:x}_{counter:x}", process::id())
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use anvil_kernel_types::{NotificationClass, NotificationPriority};

    use crate::enforcement::{EnforcementDecision, InterruptDecision};
    use crate::telemetry::{
        ControlDecision, DEGRADED_FENCE_CASCADE, DEGRADED_FENCE_CASCADE_CLEAR,
        DEGRADED_SPOOFED_ATTRIBUTION, FenceTransition, TelemetryContext, TelemetryCorrelation,
        TelemetryEmitter, delivered_envelope_for_decision, envelope_for_fence_transition,
        notification_mapping,
    };

    /// MLP2-025b: pin the reason-string value. A future enum migration
    /// is the only legitimate reason to touch this constant.
    #[test]
    fn degraded_spoofed_attribution_constant_matches_spec() {
        assert_eq!(DEGRADED_SPOOFED_ATTRIBUTION, "degraded:spoofed-attribution");
    }

    /// MLP2-026: pin the engage + clear reason-string values.
    /// Same enum-migration-target rationale as the spoof const.
    #[test]
    fn degraded_fence_cascade_constants_match_spec() {
        assert_eq!(DEGRADED_FENCE_CASCADE, "degraded:fence-cascade");
        assert_eq!(DEGRADED_FENCE_CASCADE_CLEAR, "degraded:fence-cascade-clear");
    }

    fn context() -> TelemetryContext {
        TelemetryContext {
            producer_instance_id: "producer-1".to_string(),
            seq: 7,
            timestamp: "2026-04-29T15:40:00Z".to_string(),
            session_id: Some("sess-1".to_string()),
            worktree: Some("feat/intd".to_string()),
            run_id: Some("run-1".to_string()),
            control_correlation_id: Some("ctrl-1".to_string()),
            originating_session_id: Some("sess-1".to_string()),
            originating_driver_id: Some("intercept-daemon-v1".to_string()),
        }
    }

    #[test]
    fn interrupt_decision_becomes_canonical_notification_envelope() {
        let decision = EnforcementDecision::Interrupt(InterruptDecision {
            rule_id: "anvil.reasoning.ai-001".to_string(),
            message: "appeal to authority detected".to_string(),
            line: Some(42),
            affected_paths: vec![PathBuf::from("src/lib.rs")],
        });

        let envelope = delivered_envelope_for_decision(&context(), &decision);

        assert_eq!(envelope.schema, "anvil.notification.v1");
        let notification_context = envelope.notification.context.as_ref().expect("context");

        assert_eq!(envelope.correlation.source, "intercept");
        assert_eq!(notification_context.source.as_deref(), Some("intercept"));
        assert_eq!(envelope.notification.class, NotificationClass::Interrupt);
        assert_eq!(
            envelope.notification.priority,
            NotificationPriority::Critical
        );
        assert_eq!(envelope.notification.title, "anvil.reasoning.ai-001");
        assert_eq!(notification_context.file.as_deref(), Some("src/lib.rs"));
        let mirror = envelope.mirror.as_ref().expect("mirror");
        assert_eq!(mirror.decision, ControlDecision::Interrupt);
        assert_eq!(mirror.driver, "intercept-daemon-v1");
        assert!(mirror.ack_required);
        assert_eq!(mirror.control_correlation_id.as_deref(), Some("ctrl-1"));
    }

    #[test]
    fn allow_decision_uses_low_priority_info_mapping() {
        let decision = EnforcementDecision::Allow {
            affected_paths: vec![PathBuf::from("src/ok.rs")],
        };

        let envelope = delivered_envelope_for_decision(&context(), &decision);

        assert_eq!(envelope.notification.class, NotificationClass::Info);
        assert_eq!(envelope.notification.priority, NotificationPriority::Low);
        let mirror = envelope.mirror.expect("mirror");
        assert_eq!(mirror.decision, ControlDecision::Allow);
        assert!(!mirror.ack_required);
    }

    #[test]
    fn fixed_mapping_table_matches_intercept_notification_model() {
        assert_eq!(
            notification_mapping(ControlDecision::Allow),
            (NotificationClass::Info, NotificationPriority::Low),
        );
        assert_eq!(
            notification_mapping(ControlDecision::Warn),
            (NotificationClass::Warning, NotificationPriority::High),
        );
        assert_eq!(
            notification_mapping(ControlDecision::Block),
            (NotificationClass::Block, NotificationPriority::Critical),
        );
        assert_eq!(
            notification_mapping(ControlDecision::Interrupt),
            (NotificationClass::Interrupt, NotificationPriority::Critical),
        );
    }

    #[test]
    fn fence_transition_populates_grouping_transition() {
        let envelope = envelope_for_fence_transition(
            &context(),
            Path::new("/worktrees/demo"),
            FenceTransition::ActiveToFenced,
        );
        let grouping = envelope.grouping.expect("grouping");
        let transition = grouping.transition.expect("transition");

        assert_eq!(envelope.notification.class, NotificationClass::FenceState);
        assert_eq!(
            envelope.notification.priority,
            NotificationPriority::Critical
        );
        assert!(envelope.mirror.is_none());
        assert_eq!(
            grouping.key.as_deref(),
            Some("intercept:fence:/worktrees/demo")
        );
        assert_eq!(transition.from, "active");
        assert_eq!(transition.to, "fenced");
    }

    #[test]
    fn unblock_transition_uses_fenced_to_active_grouping() {
        let envelope = envelope_for_fence_transition(
            &context(),
            Path::new("/worktrees/demo"),
            FenceTransition::FencedToActive,
        );
        let transition = envelope
            .grouping
            .expect("grouping")
            .transition
            .expect("transition");

        assert!(envelope.mirror.is_none());
        assert_eq!(envelope.notification.priority, NotificationPriority::Normal);
        assert_eq!(transition.from, "fenced");
        assert_eq!(transition.to, "active");
    }

    #[test]
    fn emitter_owns_sequence_and_derives_fence_worktree_correlation() {
        let mut emitter = TelemetryEmitter::for_tests("producer-1", "2026-04-29T15:40:00Z");
        let decision = EnforcementDecision::Allow {
            affected_paths: vec![PathBuf::from("src/ok.rs")],
        };

        let first =
            emitter.delivered_envelope_for_decision(TelemetryCorrelation::default(), &decision);
        let second = emitter.envelope_for_fence_transition(
            TelemetryCorrelation {
                worktree: Some("wrong-worktree".to_string()),
                ..TelemetryCorrelation::default()
            },
            Path::new("/worktrees/demo"),
            FenceTransition::ActiveToFenced,
        );

        assert_eq!(first.producer_instance_id, "producer-1");
        assert_eq!(first.seq, 1);
        assert_eq!(second.seq, 2);
        assert_eq!(
            second.correlation.worktree.as_deref(),
            Some("/worktrees/demo")
        );
    }

    #[test]
    fn envelope_serialises_to_notification_stream_wire_shape() {
        let decision = EnforcementDecision::Interrupt(InterruptDecision {
            rule_id: "secret-detection".to_string(),
            message: "secret detected".to_string(),
            line: None,
            affected_paths: vec![PathBuf::from("src/lib.rs")],
        });

        let value = serde_json::to_value(delivered_envelope_for_decision(&context(), &decision))
            .expect("serialise envelope");

        assert_eq!(value["schema"], "anvil.notification.v1");
        assert_eq!(value["correlation"]["source"], "intercept");
        assert_eq!(value["notification"]["class"], "interrupt");
        assert_eq!(value["notification"]["priority"], "critical");
        assert_eq!(value["notification"]["context"]["source"], "intercept");
        assert_eq!(value["mirror"]["decision"], "interrupt");
        assert_eq!(value["mirror"]["driver"], "intercept-daemon-v1");
    }

    #[test]
    fn failed_control_send_emits_health_with_attempted_decision_mirror() {
        let mut emitter = TelemetryEmitter::for_tests("producer-1", "2026-04-29T15:40:00Z");

        let envelope = emitter.failed_send_health_envelope(
            TelemetryCorrelation::default(),
            ControlDecision::Interrupt,
            "ctrl-1",
            "failed to deliver control decision",
        );

        assert_eq!(envelope.notification.class, NotificationClass::Health);
        assert_eq!(envelope.notification.priority, NotificationPriority::High);
        let mirror = envelope.mirror.expect("mirror");
        assert_eq!(mirror.decision, ControlDecision::Interrupt);
        assert_eq!(mirror.driver, "intercept-daemon-v1");
        assert!(mirror.ack_required);
        assert_eq!(mirror.control_correlation_id.as_deref(), Some("ctrl-1"));
    }
}
