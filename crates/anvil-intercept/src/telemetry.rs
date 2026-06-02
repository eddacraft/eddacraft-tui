use std::path::Path;
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub use anvil_kernel_types::diagnostics::ControlDecision;
use anvil_kernel_types::diagnostics::Severity;
use anvil_kernel_types::{
    Diagnostic, Notification, NotificationClass, NotificationContext, NotificationPriority,
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
/// both the notification envelope (`ActiveToFenced`) AND
/// `tracing::warn!`. Defined as a `pub const` so a future migration
/// to a typed degraded-mode enum has a single find-target. See
/// `plans/specs/2026-05-16-mlp2-026-fence-cascade-control-lane.md`
/// §8.
pub const DEGRADED_FENCE_CASCADE: &str = "degraded:fence-cascade";

/// MLP2-026: paired clear reason emitted when an operator clears
/// a `degraded:fence-cascade` via `anvil intercept unblock
/// --acknowledge-cascade`. Emitted via the notification envelope
/// (`FencedToActive`) AND `tracing::info!` — mirrors the priority
/// asymmetry from the existing `FenceTransition` mapping
/// (`Critical` on engage, `Normal` on clear).
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

    /// RTAI-007: build the telemetry-mirror envelope for one mid-edit
    /// (`scan_buffer`, `mode = midEdit`) decision. The outcome class is
    /// derived from the diagnostics via [`midedit_decision_class`]
    /// (allow / warn / block — `interrupt` never applies mid-edit), and
    /// the mirror carries [`MirrorPath::MidEdit`] so subscribers split
    /// in-flight from save-time without parsing the rule id.
    ///
    /// The envelope shares the canonical `anvil.notification.v1` shape
    /// and `correlation.source = "intercept"` with the save-time path,
    /// so an INTD-015 fan-out (`crate::fanout`) redacts it with the
    /// same machinery — the `mirror.path` discriminator survives
    /// redaction unchanged.
    pub fn midedit_envelope_for_decision(
        &mut self,
        correlation: TelemetryCorrelation,
        diagnostics: &[Diagnostic],
    ) -> NotificationEnvelope {
        let context = self.next_context(correlation);
        midedit_envelope(&context, diagnostics)
    }

    /// Convenience builder mirroring
    /// [`Self::delivered_envelope_for_decision_from`]: populate the
    /// INTD-015 `originating_session_id` / `originating_driver_id`
    /// scoping fields from a single session+driver pair so the fan-out
    /// can authorise (or redact) the mid-edit envelope.
    pub fn midedit_envelope_for_decision_from(
        &mut self,
        session_id: impl Into<String>,
        driver_id: impl Into<String>,
        diagnostics: &[Diagnostic],
    ) -> NotificationEnvelope {
        let session_id = session_id.into();
        let correlation = TelemetryCorrelation {
            session_id: Some(session_id.clone()),
            originating_session_id: Some(session_id),
            originating_driver_id: Some(driver_id.into()),
            ..TelemetryCorrelation::default()
        };
        self.midedit_envelope_for_decision(correlation, diagnostics)
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

/// RTAI-007: surface discriminator on the `mirror` block so a
/// subscriber can split in-flight (mid-edit) decisions from save-time
/// decisions **without parsing the rule id**. Save-time envelopes omit
/// the field entirely (`None` → not serialised), so the pre-RTAI-007
/// wire shape is byte-identical; mid-edit envelopes carry
/// [`MirrorPath::MidEdit`] which renders as the canonical `"midEdit"`
/// string shared with `kindling_observation::MIDEDIT_GATE_ID`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MirrorPath {
    /// The decision was produced on the in-flight `scan_buffer`
    /// mid-edit path (RTAI-002), not the save-time enforcement path.
    MidEdit,
    /// Forward-compat catch-all. The discriminator is explicitly
    /// extensible (the whole point is to let subscribers split surfaces
    /// without parsing the rule id), so a `mirror.path` value emitted by
    /// a newer producer that this build does not recognise deserialises
    /// to `Unknown` instead of hard-erroring the entire envelope —
    /// mirroring the `Mode::Unknown` forward-compat pattern in
    /// `anvil-kernel-types`. This build never *produces* `Unknown`, so it
    /// is not part of the wire output.
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationMirror {
    pub decision: ControlDecision,
    pub driver: String,
    pub ack_required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_correlation_id: Option<String>,
    /// RTAI-007: `None` for save-time decisions (field omitted on the
    /// wire); `Some(MirrorPath::MidEdit)` for decisions produced on the
    /// mid-edit `scan_buffer` path. INTD-015 redaction preserves this
    /// field unchanged so cross-session subscribers can still tell
    /// in-flight from save-time without seeing the redacted payload.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<MirrorPath>,
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

/// RTAI-007: map a mid-edit diagnostic batch to its advisory outcome
/// class. Mid-edit is advisory, so the vocabulary is `allow` / `warn` /
/// `block` only — `interrupt` (the refuse-the-write verdict) never
/// applies on this path. The worst severity in the batch wins:
/// any `Error` → `Block`, otherwise any `Warning` → `Warn`, otherwise
/// (`Info`-only or empty) → `Allow`.
#[must_use]
pub fn midedit_decision_class(diagnostics: &[Diagnostic]) -> ControlDecision {
    let mut worst = ControlDecision::Allow;
    for diagnostic in diagnostics {
        let class = match diagnostic.severity {
            Severity::Error => ControlDecision::Block,
            Severity::Warning => ControlDecision::Warn,
            Severity::Info => ControlDecision::Allow,
        };
        if midedit_class_rank(class) > midedit_class_rank(worst) {
            worst = class;
        }
    }
    worst
}

/// Ordering for the mid-edit advisory vocabulary. `Interrupt` is not
/// reachable on this path; it sorts above `Block` only so the function
/// is total.
const fn midedit_class_rank(decision: ControlDecision) -> u8 {
    match decision {
        ControlDecision::Allow => 0,
        ControlDecision::Warn => 1,
        ControlDecision::Block => 2,
        ControlDecision::Interrupt => 3,
    }
}

/// The diagnostic whose severity drives a `warn` / `block` decision —
/// used to title the notification with the offending rule id and carry
/// its summary as the human-readable message. `None` for an `allow`
/// decision (no offending diagnostic), where the notification uses the
/// generic "allowed" copy.
fn midedit_lead_diagnostic(
    diagnostics: &[Diagnostic],
    decision: ControlDecision,
) -> Option<&Diagnostic> {
    let target = match decision {
        ControlDecision::Block => Severity::Error,
        ControlDecision::Warn => Severity::Warning,
        ControlDecision::Allow | ControlDecision::Interrupt => return None,
    };
    diagnostics
        .iter()
        .find(|diagnostic| diagnostic.severity == target)
}

/// RTAI-007: build the `anvil.notification.v1` mirror envelope for one
/// mid-edit decision. Shares `delivered_envelope_for_decision`'s shape
/// and mapping table, but stamps `mirror.path = midEdit` and derives
/// the decision class from diagnostics (advisory allow/warn/block)
/// instead of from an `EnforcementDecision`.
#[must_use]
pub(crate) fn midedit_envelope(
    context: &TelemetryContext,
    diagnostics: &[Diagnostic],
) -> NotificationEnvelope {
    let decision = midedit_decision_class(diagnostics);
    let (class, priority) = notification_mapping(decision);
    let lead = midedit_lead_diagnostic(diagnostics, decision);
    let affected_path = lead.map(|diagnostic| diagnostic.location.file.clone());
    let notification = match lead {
        Some(diagnostic) => Notification::new(
            class,
            priority,
            diagnostic.source.rule_id.clone(),
            diagnostic.summary.clone(),
        ),
        None => Notification::new(
            class,
            priority,
            "allowed",
            "mid-edit validation allowed the change",
        ),
    }
    .with_context(notification_context(affected_path.as_ref()));

    envelope_with_path(
        context,
        Some(decision),
        notification,
        grouping_for_path(affected_path.as_ref()),
        Some(MirrorPath::MidEdit),
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
    // Save-time path: no surface discriminator on the mirror.
    envelope_with_path(context, decision, notification, grouping, None)
}

fn envelope_with_path(
    context: &TelemetryContext,
    decision: Option<ControlDecision>,
    notification: Notification,
    grouping: Option<NotificationGrouping>,
    mirror_path: Option<MirrorPath>,
) -> NotificationEnvelope {
    let mirror = decision.map(|decision| NotificationMirror {
        decision,
        driver: INTERCEPT_DRIVER_ID.to_string(),
        // RTAI-007: the mid-edit path is advisory — there is no ack
        // return channel — so a mid-edit decision never requires an
        // acknowledgement, even when it resolves to `block`. Only the
        // save-time path (`mirror_path == None`) can demand an ack.
        ack_required: mirror_path.is_none() && ack_required(decision),
        control_correlation_id: context.control_correlation_id.clone(),
        path: mirror_path,
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

/// RTAI-007: mid-edit telemetry mirror tests. Kept in a dedicated
/// module named `midedit_telemetry` so the work-item validation command
/// `cargo test -p eddacraft-anvil-intercept --lib midedit_telemetry`
/// selects exactly this surface.
#[cfg(test)]
mod midedit_telemetry {
    use anvil_kernel_types::diagnostics::{
        Category, ControlDecision, DiagnosticSource, KnownMode, Location, Severity,
    };
    use anvil_kernel_types::{Diagnostic, Mode};

    use super::{MirrorPath, TelemetryCorrelation, TelemetryEmitter, midedit_decision_class};
    use crate::fanout::{
        CrossSessionPolicy, Delivery, Fanout, OwnershipResolver, SubscriberId,
        TelemetryRedactionKey,
    };

    fn diag(rule_id: &str, severity: Severity, file: &str) -> Diagnostic {
        Diagnostic::new(
            format!("diag-{rule_id}"),
            severity,
            "reasoning-pattern violation",
            Location {
                file: file.to_string(),
                line: Some(7),
                column: None,
                end_line: None,
                end_column: None,
            },
            Category::Reasoning,
            DiagnosticSource {
                rule_id: rule_id.to_string(),
                source_module: "anvil-checks::reasoning".to_string(),
            },
            Mode::known(KnownMode::MidEdit),
        )
    }

    fn emitter() -> TelemetryEmitter {
        TelemetryEmitter::for_tests("producer-midedit", "2026-06-02T10:00:00Z")
    }

    /// Resolver that denies every subscriber, forcing the fan-out down
    /// the cross-session branch so we can exercise INTD-015 redaction.
    struct DenyAllResolver;
    impl OwnershipResolver for DenyAllResolver {
        fn is_authorised(&self, _subscriber: &SubscriberId, _originating_session_id: &str) -> bool {
            false
        }
    }

    #[test]
    fn block_decision_carries_midedit_path_and_intercept_source() {
        let mut emitter = emitter();
        let envelope = emitter.midedit_envelope_for_decision(
            TelemetryCorrelation::default(),
            &[diag(
                "anvil.reasoning.ai-001",
                Severity::Error,
                "src/lib.rs",
            )],
        );

        let value = serde_json::to_value(&envelope).expect("serialise envelope");
        assert_eq!(value["schema"], "anvil.notification.v1");
        assert_eq!(value["correlation"]["source"], "intercept");
        // mirror.decision is the advisory class; mirror.path is the new
        // RTAI-007 discriminator rendered as the canonical "midEdit".
        assert_eq!(value["mirror"]["decision"], "block");
        assert_eq!(value["mirror"]["path"], "midEdit");
        // Title is the offending rule id (no rule-id parsing needed for
        // the in-flight/save-time split — that is what mirror.path is for).
        assert_eq!(value["notification"]["title"], "anvil.reasoning.ai-001");
        let mirror = envelope.mirror.as_ref().unwrap();
        assert_eq!(mirror.path, Some(MirrorPath::MidEdit));
        // Mid-edit is advisory — even a `block` carries no ack
        // requirement, because there is no mid-edit ack channel.
        assert!(
            !mirror.ack_required,
            "mid-edit block must stay advisory (ack_required=false)"
        );
    }

    #[test]
    fn block_titles_with_error_even_when_warning_leads_the_batch() {
        // Ordering coherence: midedit_lead_diagnostic is first-wins by
        // severity class, not slice position. A `[Warning, Error]` batch
        // resolves to `block` and titles with the Error's rule id — not
        // the leading Warning — so the title never disagrees with the
        // decision.
        let mut emitter = emitter();
        let envelope = emitter.midedit_envelope_for_decision(
            TelemetryCorrelation::default(),
            &[
                diag("warn-rule", Severity::Warning, "src/a.rs"),
                diag("err-rule", Severity::Error, "src/b.rs"),
            ],
        );
        let mirror = envelope.mirror.as_ref().expect("mirror");
        assert_eq!(mirror.decision, ControlDecision::Block);
        assert_eq!(envelope.notification.title, "err-rule");
        let context = envelope.notification.context.as_ref().expect("context");
        assert_eq!(context.file.as_deref(), Some("src/b.rs"));
    }

    #[test]
    fn mirror_path_round_trips_and_tolerates_unknown_future_values() {
        // "midEdit" round-trips through serde.
        assert_eq!(
            serde_json::to_value(MirrorPath::MidEdit).unwrap(),
            serde_json::json!("midEdit"),
        );
        let parsed: MirrorPath = serde_json::from_value(serde_json::json!("midEdit")).unwrap();
        assert_eq!(parsed, MirrorPath::MidEdit);
        // Forward-compat: a value emitted by a newer producer folds to
        // `Unknown` rather than hard-erroring the envelope deserialise.
        let future: MirrorPath =
            serde_json::from_value(serde_json::json!("saveTime")).expect("unknown folds, not errs");
        assert_eq!(future, MirrorPath::Unknown);
    }

    #[test]
    fn decision_class_takes_worst_severity_and_never_interrupts() {
        assert_eq!(midedit_decision_class(&[]), ControlDecision::Allow);
        assert_eq!(
            midedit_decision_class(&[diag("r", Severity::Info, "a")]),
            ControlDecision::Allow,
        );
        assert_eq!(
            midedit_decision_class(&[diag("r", Severity::Warning, "a")]),
            ControlDecision::Warn,
        );
        assert_eq!(
            midedit_decision_class(&[diag("r", Severity::Error, "a")]),
            ControlDecision::Block,
        );
        // Mixed batch → worst (block); interrupt is unreachable here.
        let mixed = midedit_decision_class(&[
            diag("i", Severity::Info, "a"),
            diag("w", Severity::Warning, "a"),
            diag("e", Severity::Error, "a"),
        ]);
        assert_eq!(mixed, ControlDecision::Block);
        assert_ne!(mixed, ControlDecision::Interrupt);
    }

    #[test]
    fn allow_decision_uses_generic_copy_but_still_marks_midedit() {
        let mut emitter = emitter();
        // Info-only batch → allow, but the decision is still mirrored
        // on the mid-edit path so the observability story stays "one
        // shape across surfaces".
        let envelope = emitter.midedit_envelope_for_decision(
            TelemetryCorrelation::default(),
            &[diag("anvil.reasoning.ai-002", Severity::Info, "src/lib.rs")],
        );
        let mirror = envelope.mirror.as_ref().expect("mirror present");
        assert_eq!(mirror.decision, ControlDecision::Allow);
        assert_eq!(mirror.path, Some(MirrorPath::MidEdit));
        assert!(!mirror.ack_required);
        assert_eq!(envelope.notification.title, "allowed");
    }

    #[test]
    fn warn_decision_titles_with_first_warning_rule() {
        let mut emitter = emitter();
        let envelope = emitter.midedit_envelope_for_decision(
            TelemetryCorrelation::default(),
            &[
                diag("info-rule", Severity::Info, "src/a.rs"),
                diag("warn-rule", Severity::Warning, "src/b.rs"),
            ],
        );
        let mirror = envelope.mirror.as_ref().expect("mirror");
        assert_eq!(mirror.decision, ControlDecision::Warn);
        assert_eq!(mirror.path, Some(MirrorPath::MidEdit));
        assert_eq!(envelope.notification.title, "warn-rule");
        let context = envelope.notification.context.as_ref().expect("context");
        assert_eq!(context.file.as_deref(), Some("src/b.rs"));
    }

    #[test]
    fn save_time_envelope_omits_mirror_path() {
        use crate::enforcement::EnforcementDecision;
        use std::path::PathBuf;

        let mut emitter = emitter();
        let decision = EnforcementDecision::Allow {
            affected_paths: vec![PathBuf::from("src/ok.rs")],
        };
        let envelope =
            emitter.delivered_envelope_for_decision(TelemetryCorrelation::default(), &decision);
        // In-struct: save-time leaves path None.
        assert_eq!(envelope.mirror.as_ref().unwrap().path, None);
        // On the wire: the `path` key is omitted entirely, so the
        // pre-RTAI-007 save-time shape is byte-identical.
        let line = serde_json::to_string(&envelope).expect("serialise");
        assert!(
            !line.contains("\"path\""),
            "save-time mirror must omit the path discriminator: {line}"
        );
        assert!(!line.contains("midEdit"));
    }

    #[test]
    fn cross_session_redaction_preserves_midedit_discriminator() {
        let mut emitter = emitter();
        // Originating session set so the fan-out has a scoping key; the
        // DenyAll resolver pushes it down the cross-session branch.
        let envelope = emitter.midedit_envelope_for_decision_from(
            "sess-A",
            "driver-test",
            &[diag(
                "anvil.secret.aws",
                Severity::Error,
                "src/api/client.ts",
            )],
        );

        let fanout = Fanout::with_cross_session_policy_and_key(
            Box::new(DenyAllResolver),
            CrossSessionPolicy::Redact,
            TelemetryRedactionKey::from_bytes([0x42; 32]),
        );
        let subscriber = SubscriberId::new("peer:uid=1000:bin=stranger");
        fanout.register(subscriber);

        let routed = fanout.route(&envelope);
        let Delivery::Redact(redacted) = &routed[0].delivery else {
            panic!("expected redacted delivery, got {:?}", routed[0].delivery);
        };

        // The mid-edit discriminator AND the decision survive redaction
        // unchanged — a cross-session subscriber can still split
        // in-flight from save-time and see *that* a block happened,
        // without seeing the redacted payload.
        let mirror = redacted.mirror.as_ref().expect("mirror preserved");
        assert_eq!(mirror.path, Some(MirrorPath::MidEdit));
        assert_eq!(mirror.decision, ControlDecision::Block);
        // The file path is hashed, the message is the fixed marker.
        let file = redacted
            .notification
            .context
            .as_ref()
            .and_then(|c| c.file.as_deref())
            .expect("redacted file");
        assert!(file.starts_with("[redacted:"), "file hashed: {file}");
        assert!(!file.contains("client.ts"));
        assert_eq!(redacted.notification.message, "[redacted]");
        // The path-bearing grouping key must also be hashed — the
        // mid-edit decision groups on `intercept:decision:<file>`, which
        // would leak the path otherwise.
        let key = redacted
            .grouping
            .as_ref()
            .and_then(|g| g.key.as_deref())
            .expect("redacted grouping key");
        assert!(key.starts_with("[redacted:"), "grouping key hashed: {key}");
        assert!(!key.contains("client.ts"));
    }
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
