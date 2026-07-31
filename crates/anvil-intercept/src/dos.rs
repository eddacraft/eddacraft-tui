//! INTD-016: `DoS` budgets for the IPC listener.
//!
//! Connection, request-rate, and payload limits that protect the daemon from
//! abusive or runaway clients. Separate from per-event telemetry fan-out.

use std::time::{Duration, Instant};

use anvil_intercept_proto::enforcement_config::DosConfigFile;

/// Pinned default values for [`IpcLimits`]. Exposed as `pub const`
/// so tests, telemetry, and the operator-visible `anvil intercept
/// status` surface can read the same numbers without re-declaring
/// them. Each constant carries an INTD-016-spec rationale comment.
pub const DEFAULT_MAX_CONCURRENT_CONNECTIONS: usize = 64;
pub const DEFAULT_RPS_SUSTAINED: f64 = 100.0;
pub const DEFAULT_RPS_BURST: u32 = 1000;
pub const DEFAULT_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);
pub const DEFAULT_IDLE_TIMEOUT: Duration = Duration::from_mins(1);
/// 64 KiB — the cap for control-lane (non-`scan_buffer`) NDJSON
/// frames. Manifests, register-session, heartbeat, and friends are
/// all small JSON-RPC objects; the cap is generous (a 64 KiB
/// register-session frame would already be pathological) but
/// well below the 1 MiB `scan_buffer` payload that INTD-005 sized.
pub const DEFAULT_CONTROL_FRAME_MAX_BYTES: usize = 64 * 1024;
/// CIB-154 — 32 distinct workspace roots per connection. `Open`-mode
/// admission pins one real file descriptor (`WorkspaceAnchor`) per
/// distinct admitted root, so without a ceiling a same-uid peer can
/// exhaust the daemon's descriptor table by naming many roots. 32 is a
/// coarse, path-oriented sibling of the finer per-worktree session cap
/// (`DEFAULT_PER_WORKTREE_MAX = 16`): generous enough for real multi-
/// root workflows (a monorepo plus a handful of sibling checkouts) yet
/// far below any descriptor-exhaustion threshold. Clamped to a minimum
/// of 1 at [`IpcLimits::from_config`].
pub const DEFAULT_MAX_ADMITTED_ROOTS: usize = 32;

/// Configuration for the IPC listener's `DoS` budgets. Constructed
/// at daemon start from [`crate::config::Resolved`] (which reads
/// the `enforcement.dos.*` keys); tests construct one directly to
/// drive specific budgets.
///
/// Field types are picked to match how the listener reads them:
/// `f64` for sustained RPS so the bucket refill rate has natural
/// fractional precision; `u32` for burst capacity because the
/// bucket increments by integer counts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IpcLimits {
    pub max_concurrent_connections: usize,
    pub rps_sustained: f64,
    pub rps_burst: u32,
    pub handshake_timeout: Duration,
    pub idle_timeout: Duration,
    pub control_frame_max_bytes: usize,
    /// CIB-154: per-connection cap on distinct admitted workspace roots.
    pub max_admitted_roots: usize,
}

impl Default for IpcLimits {
    fn default() -> Self {
        Self {
            max_concurrent_connections: DEFAULT_MAX_CONCURRENT_CONNECTIONS,
            rps_sustained: DEFAULT_RPS_SUSTAINED,
            rps_burst: DEFAULT_RPS_BURST,
            handshake_timeout: DEFAULT_HANDSHAKE_TIMEOUT,
            idle_timeout: DEFAULT_IDLE_TIMEOUT,
            control_frame_max_bytes: DEFAULT_CONTROL_FRAME_MAX_BYTES,
            max_admitted_roots: DEFAULT_MAX_ADMITTED_ROOTS,
        }
    }
}

impl IpcLimits {
    /// Construct from an INTD-008 / INTD-016 config block. Missing
    /// fields fall back to the defaults defined above; values that
    /// would weaken the daemon (e.g. zero connection cap) are
    /// clamped to a minimum of 1 — the daemon must always accept
    /// at least one connection or the operator cannot recover.
    #[must_use]
    pub fn from_config(config: &DosConfigFile) -> Self {
        let mut out = Self::default();
        if let Some(max) = config.max_connections {
            out.max_concurrent_connections = max.max(1);
        }
        if let Some(sustained) = config.rps_sustained {
            out.rps_sustained = sustained.max(0.0);
        }
        if let Some(burst) = config.rps_burst {
            out.rps_burst = burst.max(1);
        }
        if let Some(timeout_s) = config.handshake_timeout_seconds {
            out.handshake_timeout = Duration::from_secs(timeout_s.max(1));
        }
        if let Some(timeout_s) = config.idle_timeout_seconds {
            out.idle_timeout = Duration::from_secs(timeout_s.max(1));
        }
        if let Some(bytes) = config.control_frame_max_bytes {
            out.control_frame_max_bytes = bytes.max(256);
        }
        if let Some(max) = config.max_admitted_roots {
            // Clamp to a minimum of 1: a connection must be able to admit at
            // least its own workspace root or no verb could ever be served.
            out.max_admitted_roots = max.max(1);
        }
        out
    }
}

/// Per-connection token bucket for RPS enforcement.
///
/// The bucket starts full at [`IpcLimits::rps_burst`] tokens and
/// refills at [`IpcLimits::rps_sustained`] tokens per second.
/// [`RpsBucket::try_consume`] removes one token; if the bucket is
/// empty, the call returns `false` and the listener sends a
/// structured rate-limit error WITHOUT closing the connection.
///
/// Time is injected explicitly so tests can drive the refill
/// curve without `tokio::time::sleep`. Production callers pass
/// `Instant::now()`; tests pass a controlled clock.
#[derive(Debug, Clone)]
pub struct RpsBucket {
    capacity: f64,
    refill_per_second: f64,
    tokens: f64,
    last_refill: Instant,
}

impl RpsBucket {
    /// Construct a fresh bucket — full at `burst`, refills at
    /// `sustained` tokens/s.
    #[must_use]
    pub fn new(burst: u32, sustained: f64, now: Instant) -> Self {
        Self {
            capacity: f64::from(burst),
            refill_per_second: sustained.max(0.0),
            tokens: f64::from(burst),
            last_refill: now,
        }
    }

    /// Construct a bucket sized to an [`IpcLimits`].
    #[must_use]
    pub fn from_limits(limits: &IpcLimits, now: Instant) -> Self {
        Self::new(limits.rps_burst, limits.rps_sustained, now)
    }

    /// Refill the bucket based on elapsed wall time, then attempt
    /// to consume one token. Returns `true` if a token was
    /// consumed; `false` if the bucket was empty.
    pub fn try_consume(&mut self, now: Instant) -> bool {
        self.refill(now);
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    fn refill(&mut self, now: Instant) {
        let elapsed = now.saturating_duration_since(self.last_refill);
        if elapsed.is_zero() {
            return;
        }
        let added = elapsed.as_secs_f64() * self.refill_per_second;
        self.tokens = (self.tokens + added).min(self.capacity);
        self.last_refill = now;
    }

    /// Diagnostic accessor for the current token count. Tests use
    /// this to assert refill behaviour; production code never
    /// reads it directly.
    #[cfg(test)]
    pub(crate) fn tokens(&self) -> f64 {
        self.tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limits_defaults_match_intd_016_spec() {
        let limits = IpcLimits::default();
        assert_eq!(limits.max_concurrent_connections, 64);
        assert!((limits.rps_sustained - 100.0).abs() < f64::EPSILON);
        assert_eq!(limits.rps_burst, 1000);
        assert_eq!(limits.handshake_timeout, Duration::from_secs(5));
        assert_eq!(limits.idle_timeout, Duration::from_mins(1));
        assert_eq!(limits.control_frame_max_bytes, 64 * 1024);
        assert_eq!(limits.max_admitted_roots, 32);
    }

    #[test]
    fn limits_from_config_clamps_unsafe_values() {
        // Construct a config that asks for a 0-connection cap and a
        // negative RPS rate. Both must be clamped — daemon must
        // remain reachable; rate must be a real number.
        let config = DosConfigFile {
            max_connections: Some(0),
            rps_sustained: Some(-5.0),
            rps_burst: Some(0),
            handshake_timeout_seconds: Some(0),
            idle_timeout_seconds: Some(0),
            control_frame_max_bytes: Some(1),
            max_admitted_roots: Some(0),
        };
        let limits = IpcLimits::from_config(&config);
        assert!(limits.max_concurrent_connections >= 1);
        assert!(limits.rps_sustained >= 0.0);
        assert!(limits.rps_burst >= 1);
        assert!(limits.handshake_timeout >= Duration::from_secs(1));
        assert!(limits.idle_timeout >= Duration::from_secs(1));
        assert!(limits.control_frame_max_bytes >= 256);
        assert!(
            limits.max_admitted_roots >= 1,
            "a 0 root budget must clamp to 1 so the connection can admit its own root"
        );
    }

    #[test]
    fn rps_bucket_starts_full_at_burst_capacity() {
        let now = Instant::now();
        let bucket = RpsBucket::new(10, 1.0, now);
        assert!((bucket.tokens() - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn rps_bucket_consumes_one_token_per_request() {
        let now = Instant::now();
        let mut bucket = RpsBucket::new(3, 0.0, now);
        assert!(bucket.try_consume(now));
        assert!(bucket.try_consume(now));
        assert!(bucket.try_consume(now));
        assert!(
            !bucket.try_consume(now),
            "fourth request must fail when burst exhausted",
        );
    }

    #[test]
    fn rps_bucket_refills_at_sustained_rate() {
        let now = Instant::now();
        let mut bucket = RpsBucket::new(5, 10.0, now);
        // Drain.
        for _ in 0..5 {
            assert!(bucket.try_consume(now));
        }
        assert!(!bucket.try_consume(now));

        // 200 ms later: 10 tokens/s × 0.2 s = 2 tokens added.
        let later = now + Duration::from_millis(200);
        assert!(bucket.try_consume(later));
        assert!(bucket.try_consume(later));
        assert!(
            !bucket.try_consume(later),
            "bucket cannot exceed refill rate",
        );
    }

    #[test]
    fn rps_bucket_caps_at_capacity_during_long_idle() {
        let now = Instant::now();
        let mut bucket = RpsBucket::new(5, 100.0, now);
        // Drain.
        for _ in 0..5 {
            assert!(bucket.try_consume(now));
        }
        // 10 s later: refill rate would deposit 1000 tokens, but the
        // bucket caps at burst (5).
        let later = now + Duration::from_secs(10);
        for _ in 0..5 {
            assert!(bucket.try_consume(later));
        }
        assert!(
            !bucket.try_consume(later),
            "bucket must cap at burst capacity even after long idle",
        );
    }
}
