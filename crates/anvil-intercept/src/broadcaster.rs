//! MLP2-071 Phase 2: telemetry broadcaster — the production delivery
//! machinery that sits beside the [`Fanout`](crate::fanout::Fanout)
//! decision core.
//!
//! The [`Fanout`](crate::fanout::Fanout) decides *whether* and *in what
//! form* (full / redacted / denied) each registered subscriber may see
//! a [`NotificationEnvelope`]. The broadcaster owns the *delivery* half:
//! a per-subscriber outbound channel and the [`Self::broadcast`] entry
//! that routes an envelope through the fan-out and pushes the
//! per-subscriber result onto the matching channel.
//!
//! ## Ownership boundary (design pass addendum, 2026-06-08)
//!
//! MLP2-071 Phase 2 owns this broadcaster and the IPC subscriber
//! surface that feeds it. The *producer call sites* that build real
//! transition envelopes and call [`Self::broadcast`] are
//! [DSV-044](../../../plans/modules/daemon-save-time-validation.aps.md)'s
//! slice. This module is therefore the stable handle DSV-044 attaches
//! to: it ships live, callable, and tested (the Phase 2 e2e fires
//! envelopes through [`Self::broadcast`] over a real socket), and
//! DSV-044 later wires assurance/fence transitions to it without
//! touching `ipc.rs` / `lib.rs`.
//!
//! ## Non-blocking delivery (INTD-016)
//!
//! Each subscriber has a bounded channel
//! ([`TELEMETRY_SUBSCRIBER_CHANNEL_CAP`]). When a subscriber's channel
//! is full the broadcaster **drops** the envelope for that subscriber
//! and increments [`Self::dropped_envelopes`] rather than blocking the
//! producer — INTD-016's "the daemon does not block on a misbehaving
//! peer" rule. A slow subscriber degrades only its own stream.

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use tokio::sync::mpsc;

use crate::fanout::{Delivery, Fanout, SubscriberId};
use crate::telemetry::NotificationEnvelope;
use std::sync::Arc;

/// Per-subscriber telemetry-lane channel capacity (INTD-016-family
/// budget). Sized for the v1 single-subscriber-per-session MCP /
/// driver-client shape: deep enough to ride out a brief consumer
/// stall, shallow enough that a wedged subscriber cannot pin
/// unbounded daemon memory. A full channel drops + counts (see module
/// doc); it never blocks the producer.
pub const TELEMETRY_SUBSCRIBER_CHANNEL_CAP: usize = 256;

/// JSON-RPC method name for a telemetry notification frame. Frames are
/// JSON-RPC *notifications* (no `id`), so a subscriber connection can
/// distinguish pushed telemetry from control-lane responses.
pub const TELEMETRY_NOTIFICATION_METHOD: &str = "telemetry.event";

/// Outcome of a single [`TelemetryBroadcaster::broadcast`] call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BroadcastOutcome {
    /// Subscribers the envelope was successfully queued for.
    pub delivered: usize,
    /// Subscribers whose channel was full; the envelope was dropped
    /// for them and the cumulative [`TelemetryBroadcaster::dropped_envelopes`]
    /// counter advanced.
    pub dropped: usize,
}

/// One subscriber's delivery state: its outbound frame sender plus the
/// optional client-supplied narrowing filter.
struct SubscriberChannel {
    sender: mpsc::Sender<String>,
    /// MLP2-071 D1: optional `session_ids` narrowing hint. When
    /// `Some`, the broadcaster forwards only envelopes whose
    /// `originating_session_id` is in the list. This is applied
    /// *after* the fan-out's load-bearing ownership / cross-session
    /// decision — it can only ever narrow what a subscriber sees, never
    /// widen it. `None` = no narrowing (every envelope the fan-out
    /// approves).
    session_id_filter: Option<Vec<String>>,
}

/// Production telemetry delivery surface. Wraps the [`Fanout`] (the
/// authorisation + redaction core) with the per-subscriber outbound
/// channels and a drop counter.
pub struct TelemetryBroadcaster {
    fanout: Arc<Fanout>,
    /// `SubscriberId` → that subscriber's delivery state. Kept in
    /// lock-step with the fan-out's subscriber set by
    /// [`Self::register`] / [`Self::unregister`].
    channels: Mutex<HashMap<SubscriberId, SubscriberChannel>>,
    /// Cumulative count of envelopes dropped because a subscriber's
    /// channel was full, read via [`Self::dropped_envelopes`]. Exposing
    /// it through the daemon `query_status` telemetry lane is a DSV-044
    /// prerequisite (it stays `0` until a producer broadcasts), tracked
    /// in the CHANGELOG known-gaps entry — not wired in this slice.
    dropped: AtomicU64,
}

impl TelemetryBroadcaster {
    /// Construct a broadcaster over the daemon's per-startup
    /// [`Fanout`]. The same `Arc<Fanout>` lives on `DaemonState`; the
    /// broadcaster shares it so routing decisions use the operator-
    /// configured cross-session policy + per-startup redaction salt.
    #[must_use]
    pub fn new(fanout: Arc<Fanout>) -> Self {
        Self {
            fanout,
            channels: Mutex::new(HashMap::new()),
            dropped: AtomicU64::new(0),
        }
    }

    /// Register a subscriber and return the receiver its connection's
    /// writer task drains.
    ///
    /// Registers with the fan-out (so routing decisions include the
    /// subscriber) *and* inserts the outbound channel. Re-registering
    /// the same id replaces the channel — a reconnecting subscriber
    /// from the same peer mints an identical id and transparently
    /// rebinds; the previous receiver is dropped, which closes the
    /// stale writer task.
    #[must_use]
    pub fn register(
        &self,
        id: SubscriberId,
        session_id_filter: Option<Vec<String>>,
    ) -> mpsc::Receiver<String> {
        let (tx, rx) = mpsc::channel(TELEMETRY_SUBSCRIBER_CHANNEL_CAP);
        // Register the fan-out routing entry and the channel *atomically*
        // under the channels lock. This is the consistent lock order
        // (channels → fanout); `broadcast` only ever holds the channels
        // lock *after* `Fanout::route` has released the fanout lock, so
        // the two never deadlock. Atomicity matters: a concurrent
        // `broadcast` either routes before this `fanout.register`
        // (subscriber not yet visible → nothing to deliver) or blocks on
        // the channels lock we hold until BOTH the routing entry and the
        // channel exist — so it can never observe the subscriber in the
        // routing set while its channel is missing (the silent-skip
        // window the previous fanout-first ordering left open).
        let mut channels = self.channels.lock().expect("broadcaster mutex poisoned");
        self.fanout.register(id.clone());
        channels.insert(
            id,
            SubscriberChannel {
                sender: tx,
                session_id_filter,
            },
        );
        rx
    }

    /// Remove a subscriber: drop its channel and unregister it from the
    /// fan-out. Idempotent. Called on `UnsubscribeTelemetry` and on
    /// connection drop.
    pub fn unregister(&self, id: &SubscriberId) {
        // Same atomic channels → fanout ordering as `register`, so a
        // register/unregister race for the same id (e.g. two connections
        // from one pid minting an identical id) can never leave the
        // routing set and the channel map disagreeing.
        let mut channels = self.channels.lock().expect("broadcaster mutex poisoned");
        channels.remove(id);
        self.fanout.unregister(id);
    }

    /// Number of currently registered subscribers (per the fan-out).
    #[must_use]
    pub fn subscriber_count(&self) -> usize {
        self.fanout.subscriber_count()
    }

    /// Cumulative count of envelopes dropped due to full subscriber
    /// channels since daemon start.
    #[must_use]
    pub fn dropped_envelopes(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    /// Broadcast one envelope to every authorised subscriber.
    ///
    /// Routes through the fan-out, then for each non-`Deny` decision
    /// serialises the per-subscriber envelope (full for `Allow`,
    /// redacted for `Redact`) into a JSON-RPC notification frame and
    /// `try_send`s it on the subscriber's channel. A full channel
    /// drops the frame for that subscriber and advances the drop
    /// counter; a closed channel (writer task already gone) is skipped
    /// silently — the connection-drop path will `unregister` it. The
    /// call never blocks and never awaits, so any producer — including
    /// DSV-044's transition emitters — can call it from a synchronous
    /// context.
    pub fn broadcast(&self, envelope: &NotificationEnvelope) -> BroadcastOutcome {
        let routed = self.fanout.route(envelope);
        let channels = self.channels.lock().expect("broadcaster mutex poisoned");

        let mut outcome = BroadcastOutcome::default();
        for routed in &routed {
            let delivery_envelope = match &routed.delivery {
                Delivery::Allow => envelope,
                Delivery::Redact(redacted) => redacted.as_ref(),
                Delivery::Deny => continue,
            };
            let Some(channel) = channels.get(&routed.subscriber) else {
                // Subscriber is in the routing set but its channel was
                // already torn down (registration/teardown race). Skip.
                continue;
            };
            // D1 narrowing hint: a subscriber that asked for a subset of
            // session ids only sees those. Applied after the fan-out's
            // ownership decision, so it can only narrow, never widen.
            if let Some(allowed) = &channel.session_id_filter {
                let originator = envelope.correlation.originating_session_id.as_deref();
                let passes = originator.is_some_and(|id| allowed.iter().any(|a| a == id));
                if !passes {
                    continue;
                }
            }
            let frame = match serialise_notification_frame(delivery_envelope) {
                Ok(frame) => frame,
                Err(err) => {
                    tracing::warn!(
                        target: "anvil_intercept::broadcaster",
                        error = %err,
                        "failed to serialise telemetry notification frame; skipping subscriber",
                    );
                    continue;
                }
            };
            match channel.sender.try_send(frame) {
                Ok(()) => outcome.delivered += 1,
                Err(mpsc::error::TrySendError::Full(_)) => {
                    outcome.dropped += 1;
                    // `debug`, not `warn`: a slow subscriber under a
                    // high-rate producer would fire this once per envelope,
                    // flooding the log. The durable, rate-free signal is
                    // the cumulative `dropped_envelopes` counter; per-event
                    // detail stays at debug. (Producers land with DSV-044.)
                    tracing::debug!(
                        target: "anvil_intercept::broadcaster",
                        subscriber = routed.subscriber.as_str(),
                        "telemetry subscriber channel full; dropping envelope (INTD-016)",
                    );
                }
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    // Writer task gone; the connection-drop path
                    // unregisters. Not a drop-counter event — the
                    // subscriber is leaving, not falling behind.
                }
            }
        }
        if outcome.dropped > 0 {
            self.dropped
                .fetch_add(outcome.dropped as u64, Ordering::Relaxed);
        }
        outcome
    }
}

/// Serialise an envelope into a single-line JSON-RPC notification
/// frame (no trailing newline — the writer adds the NDJSON delimiter).
fn serialise_notification_frame(
    envelope: &NotificationEnvelope,
) -> Result<String, serde_json::Error> {
    let frame = serde_json::json!({
        "jsonrpc": "2.0",
        "method": TELEMETRY_NOTIFICATION_METHOD,
        "params": envelope,
    });
    serde_json::to_string(&frame)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enforcement::{EnforcementDecision, InterruptDecision};
    use crate::fanout::{CrossSessionPolicy, OwnershipResolver};
    use crate::telemetry::{TelemetryCorrelation, TelemetryEmitter};
    use std::path::PathBuf;
    use std::sync::Mutex as StdMutex;

    /// Minimal resolver: a fixed set of authorised
    /// `(subscriber, session)` pairs, default-deny otherwise.
    struct StubResolver {
        authorised: StdMutex<Vec<(SubscriberId, String)>>,
    }

    impl StubResolver {
        fn with(pairs: &[(&SubscriberId, &str)]) -> Self {
            Self {
                authorised: StdMutex::new(
                    pairs
                        .iter()
                        .map(|(sub, sess)| ((*sub).clone(), (*sess).to_string()))
                        .collect(),
                ),
            }
        }
    }

    impl OwnershipResolver for StubResolver {
        fn is_authorised(&self, subscriber: &SubscriberId, session: &str) -> bool {
            self.authorised
                .lock()
                .unwrap()
                .iter()
                .any(|(sub, sess)| sub == subscriber && sess == session)
        }
    }

    fn envelope(session_id: &str) -> NotificationEnvelope {
        let mut emitter = TelemetryEmitter::for_tests("p", "2026-06-08T00:00:00Z");
        let decision = EnforcementDecision::Interrupt(InterruptDecision {
            rule_id: "anvil.secret.aws".to_string(),
            message: "secret leaked".to_string(),
            line: Some(7),
            affected_paths: vec![PathBuf::from("src/secret.ts")],
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

    fn broadcaster_with(
        resolver: StubResolver,
        policy: CrossSessionPolicy,
    ) -> TelemetryBroadcaster {
        let fanout = Arc::new(Fanout::with_cross_session_policy(
            Box::new(resolver),
            policy,
        ));
        TelemetryBroadcaster::new(fanout)
    }

    #[test]
    fn own_session_subscriber_receives_full_frame() {
        let owner = SubscriberId::new("owner");
        let broadcaster = broadcaster_with(
            StubResolver::with(&[(&owner, "sess-A")]),
            CrossSessionPolicy::Deny,
        );
        let mut rx = broadcaster.register(owner.clone(), None);

        let outcome = broadcaster.broadcast(&envelope("sess-A"));

        assert_eq!(outcome.delivered, 1);
        assert_eq!(outcome.dropped, 0);
        let frame = rx.try_recv().expect("frame queued");
        let value: serde_json::Value = serde_json::from_str(&frame).expect("frame json");
        assert_eq!(value["method"], TELEMETRY_NOTIFICATION_METHOD);
        // Full delivery: the original message survives (not redacted).
        assert_eq!(value["params"]["notification"]["message"], "secret leaked");
    }

    #[test]
    fn cross_session_subscriber_under_deny_receives_nothing() {
        let foreign = SubscriberId::new("foreign");
        // Foreign owns sess-B; the envelope originates from sess-A.
        let broadcaster = broadcaster_with(
            StubResolver::with(&[(&foreign, "sess-B")]),
            CrossSessionPolicy::Deny,
        );
        let mut rx = broadcaster.register(foreign.clone(), None);

        let outcome = broadcaster.broadcast(&envelope("sess-A"));

        assert_eq!(outcome.delivered, 0, "default-deny must not deliver");
        assert!(
            rx.try_recv().is_err(),
            "denied subscriber channel must stay empty"
        );
    }

    #[test]
    fn cross_session_subscriber_under_redact_receives_redacted_frame() {
        let foreign = SubscriberId::new("foreign");
        let broadcaster = broadcaster_with(StubResolver::with(&[]), CrossSessionPolicy::Redact);
        let mut rx = broadcaster.register(foreign.clone(), None);

        let outcome = broadcaster.broadcast(&envelope("sess-A"));

        assert_eq!(outcome.delivered, 1);
        let frame = rx.try_recv().expect("redacted frame queued");
        let value: serde_json::Value = serde_json::from_str(&frame).expect("frame json");
        // The free-text message is replaced with the fixed marker.
        assert_eq!(value["params"]["notification"]["message"], "[redacted]");
    }

    #[test]
    fn full_channel_drops_and_counts_without_blocking() {
        let owner = SubscriberId::new("owner");
        let broadcaster = broadcaster_with(
            StubResolver::with(&[(&owner, "sess-A")]),
            CrossSessionPolicy::Deny,
        );
        // Do NOT drain the receiver, so the channel fills.
        let _rx = broadcaster.register(owner.clone(), None);

        // Fill the channel to capacity.
        for _ in 0..TELEMETRY_SUBSCRIBER_CHANNEL_CAP {
            let outcome = broadcaster.broadcast(&envelope("sess-A"));
            assert_eq!(outcome.delivered, 1);
        }
        // The next broadcast must drop (channel full) and count, not block.
        let outcome = broadcaster.broadcast(&envelope("sess-A"));
        assert_eq!(outcome.delivered, 0);
        assert_eq!(outcome.dropped, 1);
        assert_eq!(broadcaster.dropped_envelopes(), 1);
    }

    #[test]
    fn session_id_filter_narrows_delivery_after_ownership() {
        // D1: a subscriber that owns two sessions but filters to one
        // sees only the filtered session's envelopes. The filter can
        // only narrow — it never widens past the fan-out's decision.
        let owner = SubscriberId::new("owner");
        let broadcaster = broadcaster_with(
            StubResolver::with(&[(&owner, "sess-A"), (&owner, "sess-B")]),
            CrossSessionPolicy::Deny,
        );
        let mut rx = broadcaster.register(owner.clone(), Some(vec!["sess-A".to_string()]));

        // sess-A passes the filter and is owned → delivered.
        assert_eq!(broadcaster.broadcast(&envelope("sess-A")).delivered, 1);
        assert!(rx.try_recv().is_ok());

        // sess-B is owned but filtered out → not delivered, not dropped.
        let outcome = broadcaster.broadcast(&envelope("sess-B"));
        assert_eq!(outcome.delivered, 0);
        assert_eq!(outcome.dropped, 0);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn unregister_removes_subscriber_from_routing_and_delivery() {
        let owner = SubscriberId::new("owner");
        let broadcaster = broadcaster_with(
            StubResolver::with(&[(&owner, "sess-A")]),
            CrossSessionPolicy::Deny,
        );
        let _rx = broadcaster.register(owner.clone(), None);
        assert_eq!(broadcaster.subscriber_count(), 1);

        broadcaster.unregister(&owner);
        assert_eq!(broadcaster.subscriber_count(), 0);

        let outcome = broadcaster.broadcast(&envelope("sess-A"));
        assert_eq!(outcome.delivered, 0, "unregistered subscriber gets nothing");
    }
}
