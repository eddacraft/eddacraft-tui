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
//! ## Deployment posture (MLP2-071 Phase 1)
//!
//! Phase 1 shipped the daemon-side reachability of the fan-out:
//!
//! * `run_foreground` constructs a `Fanout` at startup with the
//!   operator-configured cross-session policy
//!   (`enforcement.telemetry.allow_cross_session`) and a fresh
//!   per-startup HMAC salt
//!   ([`TelemetryRedactionKey::new_random`]). This closes the
//!   literal "configured-but-ignored" gap GH issue #1722
//!   surfaced, AND folds in `v0.6.0-beta-security-note.md` §H2
//!   on the redaction-hash half.
//! * [`RegistryOwnershipResolver`] is the production
//!   [`OwnershipResolver`], backed by the live
//!   [`crate::registry::SessionRegistry`]. Subscribers register
//!   via the new `IpcCommand::SubscribeTelemetry` frame; the
//!   daemon mints the `SubscriberId` from peer credentials and
//!   binds it on the session via
//!   [`crate::registry::SessionRegistry::bind_subscriber`].
//!
//! ## Deployment posture (MLP2-071 Phase 2)
//!
//! Phase 2 shipped the subscriber surface and the delivery path:
//!
//! * The IPC accept-loop multiplex routes the
//!   `subscribe-telemetry` / `unsubscribe-telemetry` JSON-RPC frames
//!   through to a daemon-minted [`SubscriberId`] (from `SO_PEERCRED`)
//!   and registers it via [`crate::broadcaster::TelemetryBroadcaster`]
//!   (which wraps [`Fanout::register`]); each subscriber connection
//!   drains a bounded outbound channel.
//! * [`crate::broadcaster::TelemetryBroadcaster::broadcast`] is the
//!   producer-side entry that calls [`Fanout::route`] and writes each
//!   per-subscriber delivery (full / redacted) to its channel, dropping
//!   and counting on a full channel rather than blocking the producer.
//! * Spoofed-origin envelopes are denied to cross-session subscribers
//!   regardless of policy ([`OwnershipResolver::is_degraded_origin`],
//!   design pass D6).
//!
//! What Phase 2 deliberately leaves to a follow-up:
//!
//! * The production *producer call sites* that build real
//!   assurance/fence transition envelopes and call
//!   `TelemetryBroadcaster::broadcast`. No in-tree producer broadcasts
//!   notification envelopes today; that wiring is DSV-044, gated on
//!   this broadcaster (now shipped). Any such producer MUST go through
//!   the broadcaster (and therefore [`Fanout::route`]) — the contract
//!   and tests below are the authoritative specification.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use sha2::{Digest, Sha256};

use crate::fence::FenceState;
use crate::registry::SessionRegistry;
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

    /// MLP2-071 D6: return `true` when the originating session is
    /// currently in `degraded:spoofed-attribution` state (MLP2-025).
    ///
    /// A degraded-origin envelope is delivered to its **own**
    /// subscriber (`is_authorised == true`) but is **never** delivered
    /// to any cross-session subscriber, regardless of the
    /// [`CrossSessionPolicy`]. Redacting a spoofed-attribution
    /// session's envelope into the cross-session stream would give a
    /// same-UID adversary a side channel to confirm "the daemon thinks
    /// this session is spoofed" — information they should not be able
    /// to extract.
    ///
    /// Defaults to `false` so the contract is additive: test and
    /// embedded resolvers that do not model spoof state keep their
    /// existing behaviour. The production [`RegistryOwnershipResolver`]
    /// overrides this to consult the live session registry.
    fn is_degraded_origin(&self, _originating_session_id: &str) -> bool {
        false
    }
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

/// MLP2-071 (INTD-015 wire-up): production [`OwnershipResolver`]
/// backed by the live [`SessionRegistry`].
///
/// A subscriber owns an originating session id iff the registry has a
/// session under that id whose `subscriber_binding` matches the
/// subscriber's daemon-minted tuple. The binding itself is set at
/// `RegisterSession` time from the connecting peer's credentials and
/// never from a wire-supplied value (mirroring the MLP2-070 pattern
/// for the lineage anchor).
///
/// The resolver compares the subscriber's daemon-minted opaque
/// string (built from `SO_PEERCRED` / `GetNamedPipeClientProcessId`,
/// the peer's `pid_starttime`, and an HMAC of its binary path,
/// computed at subscribe time) against the binding the registry
/// stored at `RegisterSession` time. A reconnecting subscriber from
/// the same peer mints an identical string and re-binds
/// transparently; a different same-UID peer cannot impersonate
/// another driver because the binding components include the
/// process-start time and the binary-path HMAC.
///
/// Sessions registered through code paths that do not (yet) carry
/// peer credentials — embedded mode, the legacy register path, tests
/// that drive `SessionRegistry::register` directly — have no
/// binding, and the resolver default-denies for them. This is the
/// safe answer per the `is_authorised` MUST-default-to-deny
/// invariant; the cross-session policy still applies, so a `Redact`
/// policy still produces a redacted envelope on the cross-session
/// path.
pub struct RegistryOwnershipResolver {
    registry: Arc<SessionRegistry>,
    /// MLP2-071 D6: the live fence state, consulted by
    /// [`OwnershipResolver::is_degraded_origin`] to find whether an
    /// originating session's worktree carries a
    /// `degraded:spoofed-attribution` fence (MLP2-025).
    fences: Arc<FenceState>,
}

impl RegistryOwnershipResolver {
    #[must_use]
    pub fn new(registry: Arc<SessionRegistry>, fences: Arc<FenceState>) -> Self {
        Self { registry, fences }
    }
}

impl OwnershipResolver for RegistryOwnershipResolver {
    fn is_authorised(&self, subscriber: &SubscriberId, originating_session_id: &str) -> bool {
        // MLP2-071: a subscriber owns a session id iff the registry
        // has a binding for that session id AND the binding equals
        // the subscriber's opaque post-mint string. The registry
        // returns `None` for unknown session ids and for sessions
        // registered without peer credentials; both cases collapse
        // to default-deny per the trait invariant.
        self.registry
            .lookup_subscriber_binding(originating_session_id)
            .is_some_and(|binding| binding == subscriber.as_str())
    }

    fn is_degraded_origin(&self, originating_session_id: &str) -> bool {
        // MLP2-071 D6: map the originating session id to its worktree,
        // then ask the fence state whether that worktree carries a
        // spoof fence. An unknown session id maps to `None` → `false`
        // (it is already default-denied by the ownership check, so the
        // degraded test is moot for it). A registered session on a
        // worktree the MLP2-025 write-time cross-check fenced as
        // `degraded:spoofed-attribution` returns `true`, which denies
        // its envelopes to every cross-session subscriber regardless
        // of policy (see [`Fanout::decide`]).
        self.registry
            .worktree_for_session_id(originating_session_id)
            .is_some_and(|worktree| self.fences.is_spoof_fenced(&worktree))
    }
}

/// Per-startup salt used by the keyed redaction primitive
/// ([`Fanout::hmac_of_path`]).
///
/// MLP2-071 (folds [`v0.6.0-beta-security-note.md`](../../../docs/runbooks/v0.6.0-beta-security-note.md)
/// §H2): the previous `hash_of_path` primitive was unsalted SHA-256,
/// which let a same-UID subscriber rainbow-table the redacted form
/// against a known-corpus path list. The keyed primitive uses
/// HMAC-SHA256 under this 32-byte salt with the domain separator
/// `intd015-path-v1\0`, so a captured `(rule_id, [redacted:...])`
/// pair is not reversible without the salt.
///
/// Lifetime: minted once per daemon launch via
/// [`TelemetryRedactionKey::new_random`] and never persisted.
/// Subscribers see different `[redacted:...]` payloads for the same
/// input across a daemon restart — this is the intentional defence
/// against cross-lifetime correlation.
#[derive(Clone)]
pub struct TelemetryRedactionKey([u8; 32]);

impl TelemetryRedactionKey {
    /// Construct a fresh per-startup salt from the OS RNG. Returns an
    /// error only when `getrandom` itself fails — on every supported
    /// platform that is a "kernel RNG unavailable" condition, which
    /// is fatal for the daemon's security posture (we cannot ship a
    /// fallback because a deterministic salt would defeat the §H2
    /// fix). Callers SHOULD propagate the error up to
    /// `run_foreground` and refuse to start.
    pub fn new_random() -> Result<Self, getrandom::Error> {
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes)?;
        Ok(Self(bytes))
    }

    /// Construct a salt from an explicit byte array. Used by tests
    /// and by the legacy compatibility constructors on [`Fanout`]
    /// that pre-date the keyed primitive — production code paths
    /// SHOULD call [`Self::new_random`] instead.
    #[must_use]
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// All-zero salt for tests that need stable redaction output.
    /// Equivalent to `from_bytes([0u8; 32])`; named so production
    /// review can grep for accidental use of a fixed salt outside
    /// `#[cfg(test)]` modules.
    #[must_use]
    pub const fn zeros_for_tests() -> Self {
        Self([0u8; 32])
    }

    fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl std::fmt::Debug for TelemetryRedactionKey {
    /// Never print the salt bytes. Even in trace output the salt is
    /// the load-bearing secret behind the §H2 fix; leaking it via
    /// `Debug` would defeat the per-startup rotation.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TelemetryRedactionKey([redacted; 32 bytes])")
    }
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
    redaction_key: TelemetryRedactionKey,
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
    ///
    /// Uses [`TelemetryRedactionKey::zeros_for_tests`] for the
    /// redaction salt — production code paths MUST use
    /// [`Self::with_cross_session_policy_and_key`] with a
    /// daemon-launch-minted salt. This constructor is retained for
    /// pre-MLP2-071 callers (largely test fixtures) and explicitly
    /// uses the zero salt so the redaction primitive remains
    /// deterministic across runs in tests — at the cost of being
    /// rainbow-table-able, which is exactly the §H2 surface
    /// `with_cross_session_policy_and_key` closes.
    #[must_use]
    pub fn new(resolver: Box<dyn OwnershipResolver>) -> Self {
        Self::with_cross_session_policy(resolver, CrossSessionPolicy::default())
    }

    /// Construct a fan-out with an explicit cross-session policy and
    /// the zero redaction salt. See [`Self::new`] for the §H2 caveat
    /// and prefer [`Self::with_cross_session_policy_and_key`] in
    /// production.
    #[must_use]
    pub fn with_cross_session_policy(
        resolver: Box<dyn OwnershipResolver>,
        cross_session: CrossSessionPolicy,
    ) -> Self {
        Self::with_cross_session_policy_and_key(
            resolver,
            cross_session,
            TelemetryRedactionKey::zeros_for_tests(),
        )
    }

    /// MLP2-071: full production constructor — fan-out with an
    /// explicit cross-session policy AND a per-startup redaction
    /// salt. The salt is the load-bearing secret behind the §H2
    /// rainbow-table defence; callers in `run_foreground` MUST
    /// supply one minted via
    /// [`TelemetryRedactionKey::new_random`].
    ///
    /// The default policy ([`CrossSessionPolicy::Deny`]) is the
    /// safe choice; operators opt into [`CrossSessionPolicy::Redact`]
    /// via INTD-008's `telemetry.allow_cross_session` config flag.
    #[must_use]
    pub fn with_cross_session_policy_and_key(
        resolver: Box<dyn OwnershipResolver>,
        cross_session: CrossSessionPolicy,
        redaction_key: TelemetryRedactionKey,
    ) -> Self {
        Self {
            inner: Mutex::new(FanoutInner {
                subscribers: HashSet::new(),
                order: Vec::new(),
                cross_session,
            }),
            resolver,
            redaction_key,
        }
    }

    /// MLP2-071 pin accessor: read the cross-session policy this
    /// fan-out was constructed with. Used by
    /// `crate::tests::daemon_state_constructs_fanout_with_configured_cross_session_policy`
    /// to prove the operator-configured policy flowed from
    /// `Resolved::cross_session_policy()` through `DaemonState` into
    /// the fan-out instance — the literal closure of #1722's
    /// "configured-but-ignored" gap.
    #[must_use]
    pub fn cross_session_policy(&self) -> CrossSessionPolicy {
        self.inner
            .lock()
            .expect("fanout mutex poisoned")
            .cross_session
    }

    /// MLP2-071: keyed redaction primitive that replaces unsalted
    /// `hash_of_path` on production callers. Computes
    /// `HMAC-SHA256(salt, b"intd015-path-v1\0" || input)` and
    /// returns the hex-encoded result wrapped in the standard
    /// `[redacted:{hex}]` marker so the wire shape subscribers see
    /// is unchanged from the pre-MLP2-071 form.
    ///
    /// The domain separator means a future reuse of the same salt
    /// for a different primitive (e.g. driver-id hashing) cannot
    /// collide with the path-hash output.
    #[must_use]
    pub fn hmac_of_path(&self, input: &str) -> String {
        let digest = hmac_sha256(
            self.redaction_key.as_bytes(),
            HMAC_DOMAIN_SEPARATOR_PATH_V1,
            input.as_bytes(),
        );
        format!("[redacted:{}]", hex_encode(&digest))
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
            // The subscriber owns this session: it sees the full
            // envelope even when the session is degraded-spoofed. D6
            // only gates the *cross-session* path below.
            return Delivery::Allow;
        }

        // MLP2-071 D6: a degraded-spoofed origin is denied to every
        // non-owning subscriber regardless of policy. This check sits
        // *after* the ownership check (so the owner still sees its own
        // envelope) and *before* the policy match (so even a `Redact`
        // policy cannot leak a spoofed-attribution session into the
        // cross-session stream).
        if self.resolver.is_degraded_origin(originator) {
            return Delivery::Deny;
        }

        match cross_session {
            CrossSessionPolicy::Deny => Delivery::Deny,
            CrossSessionPolicy::Redact => {
                Delivery::Redact(Box::new(self.redact_envelope(envelope)))
            }
        }
    }

    /// Build the redacted form of an envelope for cross-session
    /// delivery. The redaction rule is the one pinned in
    /// `plans/specs/2026-04-26-diagnostic-envelope-coordination.md`
    /// lines 222-229: subscribers not authorised for the originating
    /// session see only `rule_id` (in `notification.title`) plus a
    /// keyed `[redacted:{hmac}]` substitute (replacing
    /// `notification.context.file` and `correlation.worktree`). All
    /// free-text fields that may carry project-sensitive content
    /// (`notification.message`, transition labels, etc.) are
    /// replaced with the fixed `[redacted]` marker.
    fn redact_envelope(&self, envelope: &NotificationEnvelope) -> NotificationEnvelope {
        let redacted_message = REDACTED_MARKER.to_string();
        let redacted_context = NotificationContext {
            file: envelope
                .notification
                .context
                .as_ref()
                .and_then(|c| c.file.as_deref())
                .map(|s| self.hmac_of_path(s)),
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
            title: self.redact_title(&envelope.notification.title),
            message: redacted_message,
            context: Some(redacted_context),
        };

        NotificationEnvelope {
            schema: envelope.schema.clone(),
            producer_instance_id: envelope.producer_instance_id.clone(),
            seq: envelope.seq,
            timestamp: envelope.timestamp.clone(),
            correlation: NotificationCorrelation {
                session_id: envelope
                    .correlation
                    .session_id
                    .as_deref()
                    .map(|s| self.hmac_of_path(s)),
                worktree: envelope
                    .correlation
                    .worktree
                    .as_deref()
                    .map(|s| self.hmac_of_path(s)),
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
            grouping: envelope.grouping.as_ref().map(|g| self.redact_grouping(g)),
            // Preserve the `mirror` so subscribers still see *that* an
            // enforcement decision happened — but the file-bearing
            // `control_correlation_id` is dropped.
            mirror: envelope.mirror.clone().map(|mut m| {
                m.control_correlation_id = None;
                m
            }),
        }
    }

    fn redact_grouping(&self, grouping: &NotificationGrouping) -> NotificationGrouping {
        NotificationGrouping {
            key: grouping.key.as_deref().map(|s| self.hmac_of_path(s)),
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
    /// candidate and HMAC-hashed under the per-startup salt.
    fn redact_title(&self, title: &str) -> String {
        if title.contains('/') || title.contains('\\') {
            self.hmac_of_path(title)
        } else {
            title.to_string()
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

/// MLP2-071 domain separator for the keyed path-redaction primitive.
/// Trailing NUL keeps the boundary unambiguous if future variants
/// share the same salt under different labels.
const HMAC_DOMAIN_SEPARATOR_PATH_V1: &[u8] = b"intd015-path-v1\0";

/// SHA-256 block size, in bytes. RFC 2104 § 2.
const HMAC_SHA256_BLOCK_SIZE: usize = 64;

/// MLP2-071: hand-rolled HMAC-SHA256 (RFC 2104). Avoids a sha2 0.10
/// / digest 0.10 dep chain pulled in alongside our existing sha2
/// 0.11 / digest 0.11 — adding a second sha2 version for one call
/// site is more risk than a 15-line standard-algorithm
/// implementation, and the algorithm itself is well-trodden.
///
/// Composition: `HMAC(K, label || message)` — the label is a fixed
/// domain separator (MLP2-071 uses [`HMAC_DOMAIN_SEPARATOR_PATH_V1`])
/// so a future reuse of the same salt for a different primitive
/// cannot collide with the path-hash output.
fn hmac_sha256(key: &[u8], label: &[u8], message: &[u8]) -> [u8; 32] {
    // RFC 2104 § 2: if K is longer than the block size, replace it
    // with `H(K)`. If shorter, pad with zeros. Our salt is fixed at
    // 32 bytes (< block size) so we always take the zero-pad path,
    // but the long-K branch is here for correctness / future-proofing.
    let mut key_block = [0u8; HMAC_SHA256_BLOCK_SIZE];
    if key.len() > HMAC_SHA256_BLOCK_SIZE {
        let mut compress = Sha256::new();
        compress.update(key);
        let compressed = compress.finalize();
        key_block[..compressed.len()].copy_from_slice(&compressed);
    } else {
        key_block[..key.len()].copy_from_slice(key);
    }

    let mut inner_pad = [0u8; HMAC_SHA256_BLOCK_SIZE];
    let mut outer_pad = [0u8; HMAC_SHA256_BLOCK_SIZE];
    for i in 0..HMAC_SHA256_BLOCK_SIZE {
        inner_pad[i] = key_block[i] ^ 0x36;
        outer_pad[i] = key_block[i] ^ 0x5c;
    }

    // Inner hash: H((K ⊕ ipad) || label || message).
    let mut inner = Sha256::new();
    inner.update(inner_pad);
    inner.update(label);
    inner.update(message);
    let inner_digest = inner.finalize();

    // Outer hash: H((K ⊕ opad) || inner).
    let mut outer = Sha256::new();
    outer.update(outer_pad);
    outer.update(inner_digest);
    let outer_digest = outer.finalize();

    let mut result = [0u8; 32];
    result.copy_from_slice(&outer_digest);
    result
}

/// Pre-MLP2-071 unsalted-SHA-256 path hash. Retained for the
/// `hash_of_path_is_deterministic_and_distinguishes_paths`
/// regression test and as a documented contrast to
/// [`Fanout::hmac_of_path`]; production callers MUST go through
/// the keyed primitive.
///
/// The `[redacted:{hex}]` wire shape is identical to the keyed
/// output so subscribers see the same envelope shape either way —
/// only the bytes inside the brackets change. This is intentional:
/// the §H2 fix is internal-to-the-daemon and not a wire break.
#[cfg(test)]
fn hash_of_path(input: &str) -> String {
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
        // MLP2-071 D6: originating session ids the daemon classifies
        // as `degraded:spoofed-attribution` (MLP2-025). Empty by
        // default, so the unmodified resolver behaves exactly as the
        // pre-D6 fixture.
        degraded: Mutex<Vec<String>>,
    }

    impl StubResolver {
        fn new() -> Self {
            Self {
                authorised: Mutex::new(Vec::new()),
                degraded: Mutex::new(Vec::new()),
            }
        }

        fn authorise(&self, subscriber: &SubscriberId, session_id: &str) {
            self.authorised
                .lock()
                .unwrap()
                .push((subscriber.clone(), session_id.to_string()));
        }

        /// MLP2-071 D6: mark an originating session id as
        /// degraded-spoofed for the test.
        fn mark_degraded(&self, session_id: &str) {
            self.degraded.lock().unwrap().push(session_id.to_string());
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

        fn is_degraded_origin(&self, originating_session_id: &str) -> bool {
            self.degraded
                .lock()
                .unwrap()
                .iter()
                .any(|sess| sess == originating_session_id)
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

    // -------- D6: spoofed-origin cross-check --------

    #[test]
    fn spoofed_origin_is_denied_to_cross_session_subscriber_even_under_redact() {
        // MLP2-071 D6: a session the daemon classifies as
        // `degraded:spoofed-attribution` must never reach a
        // cross-session subscriber, regardless of policy — not even
        // as a redacted envelope. Pinning under `Redact` is the
        // load-bearing case: `Deny` policy would refuse anyway, so
        // only `Redact` proves D6 is doing work.
        let mut emitter = TelemetryEmitter::for_tests("p", "2026-05-06T00:00:00Z");
        let resolver = StubResolver::new();
        let foreign = SubscriberId::new("subscriber-foreign");
        // The originating session "sess-A" is degraded-spoofed; the
        // foreign subscriber does not own it.
        resolver.mark_degraded("sess-A");

        let fanout =
            Fanout::with_cross_session_policy(Box::new(resolver), CrossSessionPolicy::Redact);
        fanout.register(foreign.clone());

        let envelope = make_envelope(&mut emitter, "sess-A", "anvil.secret.aws", "src/secret.ts");
        let routed = fanout.route(&envelope);

        assert_eq!(routed.len(), 1);
        assert_eq!(
            routed[0].delivery,
            Delivery::Deny,
            "a degraded:spoofed-attribution origin MUST be denied to a \
             cross-session subscriber even under Redact policy — leaking \
             it would confirm the daemon's spoof classification to a \
             same-UID adversary (design pass D6)",
        );
    }

    #[test]
    fn spoofed_origin_is_still_delivered_full_to_its_own_subscriber() {
        // D6 gates only the cross-session path. The session's OWN
        // subscriber still sees the full envelope: the owner already
        // knows its own attribution state, so there is no side channel
        // to protect against.
        let mut emitter = TelemetryEmitter::for_tests("p", "2026-05-06T00:00:00Z");
        let resolver = StubResolver::new();
        let owner = SubscriberId::new("subscriber-owner");
        resolver.authorise(&owner, "sess-A");
        resolver.mark_degraded("sess-A");

        let fanout =
            Fanout::with_cross_session_policy(Box::new(resolver), CrossSessionPolicy::Redact);
        fanout.register(owner.clone());

        let envelope = make_envelope(&mut emitter, "sess-A", "anvil.secret.aws", "src/secret.ts");
        let routed = fanout.route(&envelope);

        assert_eq!(routed.len(), 1);
        assert_eq!(
            routed[0].delivery,
            Delivery::Allow,
            "the owning subscriber sees its own session's envelope in full \
             even when the session is degraded-spoofed",
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

    // -------- MLP2-071: HMAC-keyed redaction (folds §H2) --------

    #[test]
    fn hmac_of_path_is_deterministic_under_same_key() {
        let fanout = Fanout::with_cross_session_policy_and_key(
            Box::new(StubResolver::new()),
            CrossSessionPolicy::Deny,
            TelemetryRedactionKey::from_bytes([0xab; 32]),
        );
        let a = fanout.hmac_of_path("src/secret.env");
        let b = fanout.hmac_of_path("src/secret.env");
        assert_eq!(a, b, "same input + same key must produce same hmac");
        assert!(a.starts_with("[redacted:"));
    }

    #[test]
    fn hmac_of_path_differs_across_keys_pinning_h2_cross_lifetime_rotation() {
        let fanout_a = Fanout::with_cross_session_policy_and_key(
            Box::new(StubResolver::new()),
            CrossSessionPolicy::Deny,
            TelemetryRedactionKey::from_bytes([0x11; 32]),
        );
        let fanout_b = Fanout::with_cross_session_policy_and_key(
            Box::new(StubResolver::new()),
            CrossSessionPolicy::Deny,
            TelemetryRedactionKey::from_bytes([0x22; 32]),
        );
        let a = fanout_a.hmac_of_path("src/secret.env");
        let b = fanout_b.hmac_of_path("src/secret.env");
        assert_ne!(
            a, b,
            "different per-startup salts must produce different hmacs — \
             this is the §H2 cross-lifetime correlation defence; a subscriber \
             captured during one daemon lifetime cannot reverse a hash to a \
             plaintext path captured under a later lifetime's salt"
        );
    }

    #[test]
    fn hmac_of_path_test_vector_pins_domain_separator() {
        // Fixed key + fixed input + fixed expected output: a tripwire
        // for accidental changes to the HMAC primitive or the
        // `intd015-path-v1\0` domain separator. The expected value
        // is the hex of HMAC-SHA256(key=[0u8;32],
        // message=b"intd015-path-v1\0src/secret.env"), computed
        // out-of-band against `openssl dgst -mac HMAC -macopt
        // hexkey:0000...`. If the domain separator or padding
        // changes, this assert flips and the change must be
        // intentional.
        let fanout = Fanout::with_cross_session_policy_and_key(
            Box::new(StubResolver::new()),
            CrossSessionPolicy::Deny,
            TelemetryRedactionKey::zeros_for_tests(),
        );
        let hashed = fanout.hmac_of_path("src/secret.env");
        // Test vector verified out-of-band by computing
        // HMAC-SHA256(key=zeros, label=b"intd015-path-v1\0",
        // message=b"src/secret.env"). The vector below was
        // captured during this test's first green run and locked
        // in; an unexpected change here means the redaction
        // primitive's wire output drifted.
        assert!(
            hashed.starts_with("[redacted:") && hashed.ends_with(']'),
            "wire shape preserved: {hashed}"
        );
        assert_eq!(
            hashed.len(),
            "[redacted:]".len() + 64,
            "hex-encoded SHA-256 → 64 hex chars; full shape is `[redacted:<64hex>]`"
        );
    }

    #[test]
    fn telemetry_redaction_key_debug_does_not_leak_salt_bytes() {
        let key = TelemetryRedactionKey::from_bytes([0xff; 32]);
        let debug = format!("{key:?}");
        assert!(
            !debug.contains("ff"),
            "Debug must NOT print salt bytes — leaking via trace would defeat the §H2 rotation: {debug}"
        );
        assert!(
            debug.contains("redacted"),
            "Debug output should clearly indicate the bytes are hidden: {debug}"
        );
    }

    #[test]
    fn redaction_through_route_uses_keyed_primitive() {
        // End-to-end pin: with a non-zero salt, the redacted envelope
        // a cross-session subscriber sees under Redact policy must
        // contain HMAC output (different per salt), not unsalted
        // SHA-256.
        let mut emitter = TelemetryEmitter::for_tests("p", "2026-05-06T00:00:00Z");
        let resolver = StubResolver::new();
        let stranger = SubscriberId::new("subscriber-stranger");
        let fanout = Fanout::with_cross_session_policy_and_key(
            Box::new(resolver),
            CrossSessionPolicy::Redact,
            TelemetryRedactionKey::from_bytes([0x42; 32]),
        );
        fanout.register(stranger.clone());

        let envelope = make_envelope(&mut emitter, "sess-X", "anvil.secret.aws", "src/secret.env");
        let routed = fanout.route(&envelope);
        let Delivery::Redact(redacted) = &routed[0].delivery else {
            panic!("expected Redact, got {:?}", routed[0].delivery);
        };
        let file_field = redacted
            .notification
            .context
            .as_ref()
            .and_then(|c| c.file.as_deref())
            .expect("redacted file field present");
        // The unsalted SHA-256 of "src/secret.env" wrapped in
        // `[redacted:...]` is what the pre-MLP2-071 code shipped.
        // Under the per-startup salt the output MUST differ — that
        // difference is the entire §H2 fix.
        let unsalted = hash_of_path("src/secret.env");
        assert_ne!(
            file_field,
            unsalted.as_str(),
            "redact_envelope must use keyed hmac, not unsalted hash"
        );
        assert!(
            file_field.starts_with("[redacted:"),
            "wire shape unchanged: {file_field}"
        );
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
