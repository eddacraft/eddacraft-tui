//! INTD-015: daemon-enforced telemetry subscription scoping.
//!
//! Every `anvil.notification.v1` envelope the daemon emits must travel
//! through this fan-out filter before it reaches a subscriber. The
//! filter is **deny-by-default**: a subscriber sees the full envelope
//! only for sessions whose ownership is proven against the daemon's
//! authoritative session registry; events for other sessions are
//! redacted to `{ rule_id, hash_of_path }` per the diagnostic-envelope
//! coordination spec lines 222-229, OR dropped outright when the
//! daemon's `telemetry.allow_cross_session` flag is `false` (the
//! default).
//!
//! ## Threat model
//!
//! Before INTD-015, per-session event filtering was treated as a
//! driver-promised capability — the spec and KERN-052 supersession
//! delegated the access-control check to the subscriber. The 2026-04-24
//! council review (M5, security-analyst) flagged this as the wrong
//! placement: a hostile or mis-configured driver could subscribe to
//! cross-session telemetry and exfiltrate file paths, secret-detection
//! content excerpts, and architectural metadata for sessions it does
//! not own.
//!
//! INTD-015 moves the check daemon-side. The fan-out reads two pieces
//! of envelope metadata that the daemon itself populates:
//!
//! - `correlation.originating_session_id` — the session that produced
//!   the event. The daemon sets this from the change attribution path,
//!   not from any driver claim.
//! - `correlation.originating_driver_id` — the stable identity of the
//!   driver that produced the event. The daemon mints this from the
//!   socket-peer credentials of the connection (UID + binary path /
//!   install-time token), **never** from a driver-supplied
//!   `driverName`. A same-UID peer self-declaring `"driverName":
//!   "vscode"` cannot impersonate the real `VSCode` driver because the
//!   id this fan-out reads comes from `SO_PEERCRED` / equivalent, not
//!   the manifest.
//!
//! ## Decision shape
//!
//! For each `(envelope, subscriber)` pair the fan-out emits one of:
//!
//! - [`Delivery::Allow`] — the subscriber is authorised for this
//!   originating session; deliver the full envelope.
//! - [`Delivery::Redact`] — cross-session subscription is enabled
//!   and the subscriber sees a redacted envelope (`rule_id` plus
//!   `hash_of_path` only).
//! - [`Delivery::Deny`] — cross-session subscription is disabled (or
//!   the originating session id is absent / unknown); the subscriber
//!   sees nothing.
//!
//! IPC delivery itself is out of scope here — the fan-out is a pure
//! filter. The IPC listener (INTD-002) calls [`Fanout::route`] for
//! each envelope it would otherwise broadcast and writes only the
//! envelopes the fan-out approves.
//!
//! ## What this module is **not**
//!
//! - **Not the redaction policy for diagnostics.** The shared
//!   `anvil.diagnostic.v1` envelope owned by AIGUARD-002 is **locked**
//!   for this PR; the fan-out only operates on the
//!   `anvil.notification.v1` outer envelope where redaction means
//!   replacing path-bearing strings with `hash_of_path`.
//! - **Not the per-driver allowlist for `Participating` mode.** That
//!   lives under DRVR-007 (`crates/anvil-intercept/src/auth.rs`,
//!   future) and gates whether a driver can ack enforcement
//!   decisions. INTD-015's allowlist gates *visibility*, not
//!   *authority*.
//! - **Not the rate-limiter or `DoS` budget.** INTD-016 owns `DoS`
//!   budgets; INTD-015 only filters per-event.
//!
//! ## Deployment posture (wave-1 partial)
//!
//! INTD-015 in this PR ships the **filter, contract, and tests**.
//! The IPC subscribe surface that actually mints `SubscriberId`
//! values from `SO_PEERCRED` / `GetNamedPipeClientProcessId` and
//! routes broadcast envelopes through [`Fanout::route`] does **not
//! yet exist** — there is no `IpcCommand::SubscribeTelemetry` frame
//! in `anvil-intercept-proto` today, and no producer in the daemon
//! currently broadcasts envelopes to network subscribers. The
//! [`crate::telemetry::TelemetryEmitter`] continues to construct
//! envelopes; nothing delivers them to a remote subscriber yet.
//!
//! When the IPC subscribe frame lands (likely INTD-011 status /
//! diagnostics surface or DRVR-001 driver client), the wiring is:
//!
//! 1. The IPC accept loop reads peer credentials from the
//!    connected socket / pipe and constructs a `SubscriberId` from
//!    the resulting tuple. Drivers cannot influence the value.
//! 2. The accept loop calls [`Fanout::register`] with that id.
//! 3. The producer side (currently
//!    `delivered_envelope_for_decision`) calls [`Fanout::route`]
//!    on every envelope it would otherwise broadcast and writes
//!    only the per-subscriber output the fan-out approves.
//!
//! Until that wiring lands the filter is dead code in production —
//! but the contract and tests below are the authoritative
//! specification, and any producer that adds broadcast must go
//! through `Fanout::route` from day one.

use std::collections::HashSet;
use std::sync::Mutex;

use sha2::{Digest, Sha256};

use crate::telemetry::{
    NotificationCorrelation, NotificationEnvelope, NotificationGrouping, NotificationTransition,
};
use anvil_kernel_types::{Notification, NotificationContext};

/// Stable identifier for a telemetry subscriber, minted by the daemon
/// from socket-peer credentials when a connection enters
/// telemetry-subscriber mode.
///
/// This is intentionally **not** built from any driver-supplied
/// string: a hostile peer setting `driverName: "vscode"` cannot
/// re-use another driver's identity because the daemon mints the
/// `SubscriberId` from `SO_PEERCRED` / `GetNamedPipeClientProcessId`
/// plus a binary-path or install-time token resolved out-of-band.
/// The wrapped string is the opaque post-mint identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SubscriberId(String);

impl SubscriberId {
    /// Construct a subscriber id from a daemon-minted string. The
    /// daemon (not user code) is expected to call this; tests
    /// construct ids directly via this constructor with stable
    /// values that simulate real socket-peer identities.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Resolves whether a subscriber is authorised to see events for a
/// given originating session. The daemon backs this with the live
/// session registry (INTD-003) plus optional capability grants; the
/// trait keeps the fan-out testable without the registry.
///
/// Implementations MUST default to deny — `is_authorised(...)` MUST
/// return `false` when the originating session is not in the
/// subscriber's owned set, regardless of any side-channel hint. The
/// fan-out enforces this invariant by treating `false` as "no full
/// delivery"; a buggy `true` cannot be retro-corrected.
pub trait OwnershipResolver: Send + Sync {
    /// Return `true` only when the subscriber owns the originating
    /// session (i.e. the subscriber registered the session, or is
    /// the session itself).
    fn is_authorised(&self, subscriber: &SubscriberId, originating_session_id: &str) -> bool;
}

/// Cross-session redaction policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CrossSessionPolicy {
    /// Default: deny. Subscribers see nothing for sessions they do
    /// not own. The 2026-04-24 council review M5 (security-analyst)
    /// pinned this as the safe default.
    #[default]
    Deny,
    /// Operator opt-in: subscribers see a redacted envelope for
    /// cross-session events — `notification.title` becomes the
    /// `rule_id` (or empty string for non-rule events), file paths
    /// and worktree become `hash_of_path`, and human-readable
    /// messages are replaced with a fixed `[redacted]` marker.
    Redact,
}

/// Telemetry fan-out filter.
///
/// Tracks subscribers, their authorised originating-session ids
/// (resolved through the [`OwnershipResolver`]), and the
/// cross-session policy. Per-call entry point is [`Fanout::route`];
/// it returns a vector of `(SubscriberId, Delivery)` decisions in
/// stable iteration order so the caller can rely on test fixtures.
pub struct Fanout {
    inner: Mutex<FanoutInner>,
    resolver: Box<dyn OwnershipResolver>,
}

struct FanoutInner {
    subscribers: HashSet<SubscriberId>,
    /// Stable order of subscribers, in registration order. Tests
    /// rely on this for deterministic fixture comparison.
    order: Vec<SubscriberId>,
    cross_session: CrossSessionPolicy,
}

impl Fanout {
    /// Construct a new fan-out with no subscribers yet. The resolver
    /// is the load-bearing security boundary — the daemon wires it
    /// to the live session registry (INTD-003).
    #[must_use]
    pub fn new(resolver: Box<dyn OwnershipResolver>) -> Self {
        Self::with_cross_session_policy(resolver, CrossSessionPolicy::default())
    }

    /// Construct a fan-out with an explicit cross-session policy.
    /// The default policy ([`CrossSessionPolicy::Deny`]) is the
    /// safe choice; operators opt into [`CrossSessionPolicy::Redact`]
    /// via INTD-008's `telemetry.allow_cross_session` config flag.
    #[must_use]
    pub fn with_cross_session_policy(
        resolver: Box<dyn OwnershipResolver>,
        cross_session: CrossSessionPolicy,
    ) -> Self {
        Self {
            inner: Mutex::new(FanoutInner {
                subscribers: HashSet::new(),
                order: Vec::new(),
                cross_session,
            }),
            resolver,
        }
    }

    /// Register a new subscriber. Idempotent — re-registering an
    /// existing id is a no-op (the daemon may re-affirm the
    /// identity on reconnect without producing duplicates).
    pub fn register(&self, id: SubscriberId) {
        let mut guard = self.inner.lock().expect("fanout mutex poisoned");
        if guard.subscribers.insert(id.clone()) {
            guard.order.push(id);
        }
    }

    /// Remove a subscriber. Returns `true` if the subscriber was
    /// registered, `false` otherwise.
    pub fn unregister(&self, id: &SubscriberId) -> bool {
        let mut guard = self.inner.lock().expect("fanout mutex poisoned");
        let was_present = guard.subscribers.remove(id);
        if was_present {
            guard.order.retain(|sub| sub != id);
        }
        was_present
    }

    /// Number of registered subscribers.
    #[must_use]
    pub fn subscriber_count(&self) -> usize {
        self.inner
            .lock()
            .expect("fanout mutex poisoned")
            .order
            .len()
    }

    /// Compute per-subscriber delivery decisions for an envelope.
    ///
    /// The envelope's `correlation.originating_session_id` is the
    /// load-bearing scoping field. When it is absent the fan-out
    /// MUST deny all subscribers — there is no proven originator,
    /// so cross-session filtering cannot be performed safely. This
    /// matches the diagnostic-envelope coordination spec's
    /// "Subscribers MUST treat unknown session ids as not
    /// authorised" rule.
    #[must_use]
    pub fn route(&self, envelope: &NotificationEnvelope) -> Vec<RoutedDelivery> {
        let guard = self.inner.lock().expect("fanout mutex poisoned");
        let originator = envelope.correlation.originating_session_id.as_deref();

        guard
            .order
            .iter()
            .map(|subscriber| {
                let delivery = self.decide(subscriber, originator, envelope, guard.cross_session);
                RoutedDelivery {
                    subscriber: subscriber.clone(),
                    delivery,
                }
            })
            .collect()
    }

    fn decide(
        &self,
        subscriber: &SubscriberId,
        originator: Option<&str>,
        envelope: &NotificationEnvelope,
        cross_session: CrossSessionPolicy,
    ) -> Delivery {
        // No originator → deny. The default-deny rule from the
        // diagnostic-envelope coordination spec lines 222-229 says
        // unknown session ids are "not authorised". An envelope
        // missing its `originating_session_id` is exactly that
        // case: the daemon failed to attach scoping metadata, and
        // the safe response is to drop the event for this
        // subscriber rather than guess.
        let Some(originator) = originator else {
            return Delivery::Deny;
        };

        if self.resolver.is_authorised(subscriber, originator) {
            return Delivery::Allow;
        }

        match cross_session {
            CrossSessionPolicy::Deny => Delivery::Deny,
            CrossSessionPolicy::Redact => Delivery::Redact(Box::new(redact_envelope(envelope))),
        }
    }
}

/// One subscriber's filtered view of an envelope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutedDelivery {
    pub subscriber: SubscriberId,
    pub delivery: Delivery,
}

/// Per-subscriber delivery outcome.
///
/// `Redact` boxes its envelope so the enum's stack size matches the
/// other variants — the `NotificationEnvelope` is large
/// (~hundreds of bytes once strings are accounted for) and most
/// deliveries are `Allow` / `Deny`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Delivery {
    /// The subscriber owns the originating session — deliver the
    /// envelope verbatim.
    Allow,
    /// Cross-session policy is `Redact`; the subscriber sees the
    /// envelope with `rule_id` + `hash_of_path` only.
    Redact(Box<NotificationEnvelope>),
    /// Default-deny: the subscriber sees nothing.
    Deny,
}

/// Build the redacted form of an envelope for cross-session
/// delivery. The redaction rule is the one pinned in
/// `plans/specs/2026-04-26-diagnostic-envelope-coordination.md`
/// lines 222-229: subscribers not authorised for the originating
/// session see only `rule_id` (in `notification.title`) plus
/// `hash_of_path` (replacing `notification.context.file` and
/// `correlation.worktree`). All free-text fields that may carry
/// project-sensitive content (`notification.message`, transition
/// labels, etc.) are replaced with the fixed `[redacted]` marker.
fn redact_envelope(envelope: &NotificationEnvelope) -> NotificationEnvelope {
    let redacted_message = REDACTED_MARKER.to_string();
    let redacted_context = NotificationContext {
        file: envelope
            .notification
            .context
            .as_ref()
            .and_then(|c| c.file.as_deref())
            .map(hash_of_path),
        source: envelope
            .notification
            .context
            .as_ref()
            .and_then(|c| c.source.clone()),
    };

    let redacted_notification = Notification {
        class: envelope.notification.class,
        priority: envelope.notification.priority,
        // Title is by convention the rule_id for finding/interrupt
        // events; preserve it verbatim. For non-rule events
        // (`info`/`progress`/etc.) the title may contain a path —
        // hash it conservatively when it looks like one.
        title: redact_title(&envelope.notification.title),
        message: redacted_message,
        context: Some(redacted_context),
    };

    NotificationEnvelope {
        schema: envelope.schema.clone(),
        producer_instance_id: envelope.producer_instance_id.clone(),
        seq: envelope.seq,
        timestamp: envelope.timestamp.clone(),
        correlation: NotificationCorrelation {
            session_id: envelope.correlation.session_id.as_deref().map(hash_of_path),
            worktree: envelope.correlation.worktree.as_deref().map(hash_of_path),
            // Drop run_id — it can join external traces back to
            // session activity.
            run_id: None,
            source: envelope.correlation.source.clone(),
            // Preserve originating ids so subscribers can dedupe /
            // group cross-session events; these are themselves
            // opaque ids the operator chose to expose by enabling
            // `Redact`.
            originating_session_id: envelope.correlation.originating_session_id.clone(),
            originating_driver_id: envelope.correlation.originating_driver_id.clone(),
        },
        notification: redacted_notification,
        grouping: envelope.grouping.as_ref().map(redact_grouping),
        // Preserve the `mirror` so subscribers still see *that* an
        // enforcement decision happened — but the file-bearing
        // `control_correlation_id` is dropped.
        mirror: envelope.mirror.clone().map(|mut m| {
            m.control_correlation_id = None;
            m
        }),
    }
}

fn redact_grouping(grouping: &NotificationGrouping) -> NotificationGrouping {
    NotificationGrouping {
        key: grouping.key.as_deref().map(hash_of_path),
        // Transitions encode operationally-relevant state changes
        // (`active -> fenced`, etc.) using a fixed vocabulary; the
        // strings themselves do not carry project-sensitive content,
        // so they round-trip unchanged.
        transition: grouping
            .transition
            .as_ref()
            .map(|t| NotificationTransition {
                from: t.from.clone(),
                to: t.to.clone(),
            }),
    }
}

/// Title heuristic: rule_id-shaped titles (e.g.
/// `secret-aws-access-key`, `anvil.reasoning.ai-001`) are preserved
/// verbatim because the spec calls them out as the safe payload.
/// Anything containing a slash or backslash is treated as a path
/// candidate and hashed.
fn redact_title(title: &str) -> String {
    if title.contains('/') || title.contains('\\') {
        hash_of_path(title)
    } else {
        title.to_string()
    }
}

/// Stable hash function for redacted path-like fields. Hex-encoded
/// SHA-256, prefixed so subscribers can distinguish a redacted
/// value from a real string at a glance. The hash is deterministic
/// across runs so subscribers can dedupe on the redacted form.
#[must_use]
pub fn hash_of_path(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let digest = hasher.finalize();
    format!("[redacted:{}]", hex_encode(&digest))
}

fn hex_encode(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        // Writing to a `String` via `Write` is infallible — the
        // `write!` macro returns `Result<()>` only because the
        // trait is generic.
        let _ = write!(out, "{byte:02x}");
    }
    out
}

const REDACTED_MARKER: &str = "[redacted]";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enforcement::{EnforcementDecision, InterruptDecision};
    use crate::telemetry::{TelemetryCorrelation, TelemetryEmitter};
    use std::path::PathBuf;
    use std::sync::Mutex;

    /// Test resolver with a fixed map of (subscriber, session) ->
    /// authorisation. Tests construct one of these to simulate the
    /// session registry without the registry's full machinery.
    struct StubResolver {
        // `(SubscriberId, originating_session_id)` pairs that are
        // explicitly authorised. Anything not in the set returns
        // `false` — matching the trait's default-deny invariant.
        authorised: Mutex<Vec<(SubscriberId, String)>>,
    }

    impl StubResolver {
        fn new() -> Self {
            Self {
                authorised: Mutex::new(Vec::new()),
            }
        }

        fn authorise(&self, subscriber: &SubscriberId, session_id: &str) {
            self.authorised
                .lock()
                .unwrap()
                .push((subscriber.clone(), session_id.to_string()));
        }
    }

    impl OwnershipResolver for StubResolver {
        fn is_authorised(&self, subscriber: &SubscriberId, originating_session_id: &str) -> bool {
            self.authorised
                .lock()
                .unwrap()
                .iter()
                .any(|(sub, sess)| sub == subscriber && sess == originating_session_id)
        }
    }

    fn make_envelope(
        emitter: &mut TelemetryEmitter,
        session_id: &str,
        rule_id: &str,
        path: &str,
    ) -> NotificationEnvelope {
        let decision = EnforcementDecision::Interrupt(InterruptDecision {
            rule_id: rule_id.to_string(),
            message: "secret leaked into commit".to_string(),
            line: Some(42),
            affected_paths: vec![PathBuf::from(path)],
        });
        let correlation = TelemetryCorrelation {
            session_id: Some(session_id.to_string()),
            worktree: Some(format!("/worktrees/{session_id}")),
            originating_session_id: Some(session_id.to_string()),
            originating_driver_id: Some("driver-test".to_string()),
            ..TelemetryCorrelation::default()
        };
        emitter.delivered_envelope_for_decision(correlation, &decision)
    }

    // -------- Authorised subscribers see full envelope --------

    #[test]
    fn own_session_subscribe_is_honoured_with_full_delivery() {
        let mut emitter = TelemetryEmitter::for_tests("p", "2026-05-06T00:00:00Z");
        let resolver = StubResolver::new();
        let owner = SubscriberId::new("subscriber-owner");
        resolver.authorise(&owner, "sess-A");

        let fanout = Fanout::new(Box::new(resolver));
        fanout.register(owner.clone());

        let envelope = make_envelope(&mut emitter, "sess-A", "anvil.secret.aws", "src/secret.ts");
        let routed = fanout.route(&envelope);

        assert_eq!(routed.len(), 1);
        assert_eq!(routed[0].subscriber, owner);
        assert_eq!(
            routed[0].delivery,
            Delivery::Allow,
            "subscriber owning the originating session must see the full envelope",
        );
    }

    // -------- Unauthorised cross-session subscribers default-deny --------

    #[test]
    fn cross_session_subscribe_is_denied_by_default() {
        let mut emitter = TelemetryEmitter::for_tests("p", "2026-05-06T00:00:00Z");
        let resolver = StubResolver::new();
        let foreign = SubscriberId::new("subscriber-foreign");
        // Foreign subscriber owns "sess-B"; the envelope's
        // originator is "sess-A" — must be denied.
        resolver.authorise(&foreign, "sess-B");

        let fanout = Fanout::new(Box::new(resolver));
        fanout.register(foreign.clone());

        let envelope = make_envelope(&mut emitter, "sess-A", "anvil.secret.aws", "src/secret.ts");
        let routed = fanout.route(&envelope);

        assert_eq!(routed.len(), 1);
        assert_eq!(
            routed[0].delivery,
            Delivery::Deny,
            "default cross-session policy MUST deny — \
             council finding M5 (security-analyst), 2026-04-24",
        );
    }

    // -------- Cross-session redaction (operator opt-in) --------

    #[test]
    fn cross_session_subscribe_under_redact_policy_returns_redacted_envelope() {
        let mut emitter = TelemetryEmitter::for_tests("p", "2026-05-06T00:00:00Z");
        let resolver = StubResolver::new();
        let foreign = SubscriberId::new("subscriber-foreign");

        let fanout =
            Fanout::with_cross_session_policy(Box::new(resolver), CrossSessionPolicy::Redact);
        fanout.register(foreign.clone());

        let envelope = make_envelope(
            &mut emitter,
            "sess-A",
            "anvil.secret.aws",
            "src/api/client.ts",
        );
        let routed = fanout.route(&envelope);

        assert_eq!(routed.len(), 1);
        let Delivery::Redact(redacted) = &routed[0].delivery else {
            panic!("expected redacted delivery, got {:?}", routed[0].delivery);
        };

        // rule_id (the title for interrupt events) is preserved.
        assert_eq!(redacted.notification.title, "anvil.secret.aws");
        // Free-text message is replaced with the fixed marker.
        assert_eq!(redacted.notification.message, REDACTED_MARKER);
        // File path is hashed, not echoed.
        let context = redacted
            .notification
            .context
            .as_ref()
            .expect("redacted context");
        let hashed_file = context.file.as_ref().expect("file hash present");
        assert!(
            hashed_file.starts_with("[redacted:"),
            "file path must be hashed, got {hashed_file}",
        );
        assert!(
            !hashed_file.contains("client.ts"),
            "redacted file must not echo the original path: {hashed_file}",
        );
        // Worktree is hashed.
        let hashed_worktree = redacted
            .correlation
            .worktree
            .as_ref()
            .expect("worktree hash present");
        assert!(hashed_worktree.starts_with("[redacted:"));
        // run_id dropped (mirror.control_correlation_id too).
        assert!(redacted.correlation.run_id.is_none());
        if let Some(mirror) = &redacted.mirror {
            assert!(
                mirror.control_correlation_id.is_none(),
                "redacted mirror must drop control_correlation_id",
            );
        }
        // Originating ids are preserved (operator opted in via Redact).
        assert_eq!(
            redacted.correlation.originating_session_id.as_deref(),
            Some("sess-A"),
        );
    }

    // -------- Hash determinism --------

    #[test]
    fn hash_of_path_is_deterministic_and_distinguishes_paths() {
        let a = hash_of_path("src/api/client.ts");
        let b = hash_of_path("src/api/client.ts");
        let c = hash_of_path("src/api/server.ts");
        assert_eq!(a, b, "hash must be deterministic for the same input");
        assert_ne!(a, c, "different paths must produce different hashes");
        assert!(a.starts_with("[redacted:"));
    }

    // -------- Default deny on missing originator --------

    #[test]
    fn missing_originating_session_id_denies_all_subscribers() {
        let mut emitter = TelemetryEmitter::for_tests("p", "2026-05-06T00:00:00Z");
        let resolver = StubResolver::new();
        let sub = SubscriberId::new("subscriber");
        // Even authorising the subscriber for *something* must not
        // grant delivery when the envelope is missing its
        // originator — the fan-out cannot prove ownership.
        resolver.authorise(&sub, "sess-A");

        let fanout =
            Fanout::with_cross_session_policy(Box::new(resolver), CrossSessionPolicy::Redact);
        fanout.register(sub.clone());

        let decision = EnforcementDecision::Allow {
            affected_paths: vec![PathBuf::from("src/x.rs")],
        };
        // Empty correlation → no originating_session_id.
        let envelope =
            emitter.delivered_envelope_for_decision(TelemetryCorrelation::default(), &decision);
        assert!(envelope.correlation.originating_session_id.is_none());
        let routed = fanout.route(&envelope);
        assert_eq!(routed[0].delivery, Delivery::Deny);
    }

    // -------- Subscriber identity is daemon-minted (defence
    //         against `driverName` self-declaration) --------

    #[test]
    fn fanout_keys_subscriber_by_daemon_minted_id_not_driver_name() {
        // This test pins the council finding (6) defence: a
        // subscriber's authorisation is keyed by `SubscriberId`
        // (daemon-minted from socket-peer credentials), NOT by any
        // string the driver self-declares. We model that by
        // showing two `SubscriberId`s with different opaque ids
        // get independent allowlists even if a hostile driver
        // wanted them to be treated as the same identity.
        let mut emitter = TelemetryEmitter::for_tests("p", "2026-05-06T00:00:00Z");
        let resolver = StubResolver::new();
        let real_vscode = SubscriberId::new("peer:uid=1000:bin=vscode-real");
        let imposter = SubscriberId::new("peer:uid=1000:bin=imposter");
        // Only the real driver is authorised for sess-A.
        resolver.authorise(&real_vscode, "sess-A");

        let fanout = Fanout::new(Box::new(resolver));
        fanout.register(real_vscode.clone());
        fanout.register(imposter.clone());

        let envelope = make_envelope(&mut emitter, "sess-A", "rule-x", "src/x.rs");
        let routed = fanout.route(&envelope);

        let real = routed
            .iter()
            .find(|d| d.subscriber == real_vscode)
            .expect("real subscriber routed");
        let fake = routed
            .iter()
            .find(|d| d.subscriber == imposter)
            .expect("imposter routed");

        assert_eq!(real.delivery, Delivery::Allow);
        assert_eq!(
            fake.delivery,
            Delivery::Deny,
            "imposter must NOT see allow even if it claims the same `driverName`",
        );
    }

    // -------- Subscriber lifecycle --------

    #[test]
    fn register_unregister_round_trip() {
        let resolver = StubResolver::new();
        let fanout = Fanout::new(Box::new(resolver));
        let id = SubscriberId::new("s1");
        fanout.register(id.clone());
        assert_eq!(fanout.subscriber_count(), 1);
        // Idempotent re-register.
        fanout.register(id.clone());
        assert_eq!(fanout.subscriber_count(), 1);
        assert!(fanout.unregister(&id));
        assert_eq!(fanout.subscriber_count(), 0);
        assert!(!fanout.unregister(&id), "second unregister must be false");
    }

    // -------- INTD-008 ↔ INTD-015 integration --------

    #[test]
    fn cross_session_policy_threads_through_intd_008_resolved_config() {
        // Sanity-check the wiring: a `Resolved` config produced by
        // INTD-008 maps onto the fan-out's `CrossSessionPolicy`
        // exactly the same way the operator declared it. This test
        // pins the contract surface so a future refactor cannot
        // silently flip the default from Deny to Redact.
        use crate::config::Resolved;

        let default = Resolved::default();
        assert_eq!(
            default.cross_session_policy(),
            CrossSessionPolicy::Deny,
            "INTD-008 default → INTD-015 Deny",
        );

        let opt_in = Resolved {
            telemetry_allow_cross_session: true,
            ..Resolved::default()
        };
        assert_eq!(
            opt_in.cross_session_policy(),
            CrossSessionPolicy::Redact,
            "operator opt-in → INTD-015 Redact",
        );
    }
}
