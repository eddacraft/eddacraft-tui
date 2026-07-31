//! MLP2-009: sliding-window rate limiter for emitters.

use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Outcome of [`RateWindow::record`]. Tests assert on the variant
/// so consumers can branch on the result without scraping the
/// internal counter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateDecision {
    /// The event was admitted. Consumer should proceed with the
    /// emit. If `pending_drops > 0`, the consumer's emission
    /// should also carry a `degraded:observation-throttled`
    /// marker for the cumulative drop count since the previous
    /// `Allow` — the count is reported here so the consumer can
    /// attach it to the same envelope and avoid a second sync
    /// round-trip.
    Allow { pending_drops: u32 },
    /// The event was dropped because the cap was at capacity.
    /// `drops` is the running total of drops since the previous
    /// `Allow` (including this one). Consumers MAY suppress the
    /// envelope entirely; the running total is recovered by the
    /// next `Allow`'s `pending_drops`.
    Throttle { drops: u32 },
}

/// Sliding-window rate primitive. Lock-protected so multiple
/// consumers can share an `Arc<RateWindow>`. Internal queue is a
/// `VecDeque<Instant>` sized to `capacity`; eviction is `O(k)` in
/// the number of expired timestamps per call.
#[derive(Debug)]
pub struct RateWindow {
    inner: Mutex<Inner>,
}

#[derive(Debug)]
struct Inner {
    capacity: usize,
    window: Duration,
    /// Timestamps of admitted events still within the window.
    /// Ordered oldest-first so the `pop_front` eviction loop is
    /// `O(k)` per call rather than `O(n)`.
    admitted: VecDeque<Instant>,
    /// Cumulative drop count since the previous `Allow`. Reset to
    /// zero each time an event is admitted.
    pending_drops: u32,
}

impl RateWindow {
    /// Build a rate window allowing up to `capacity` events per
    /// `window` duration. A `capacity` of zero is clamped to 1 —
    /// matches MLP2-024's defensive-clamp pattern; refusing every
    /// event would never be the operator's intent.
    #[must_use]
    pub fn new(capacity: usize, window: Duration) -> Self {
        let capacity = capacity.max(1);
        Self {
            inner: Mutex::new(Inner {
                capacity,
                window,
                admitted: VecDeque::with_capacity(capacity),
                pending_drops: 0,
            }),
        }
    }

    /// Record an event at `now`. Returns [`RateDecision::Allow`]
    /// when the event is admitted (and the consumer should emit)
    /// or [`RateDecision::Throttle`] when the cap is at capacity
    /// (consumer drops the event).
    ///
    /// Tests pass `now` explicitly so they can drive the window
    /// without `std::thread::sleep`; production callers pass
    /// `Instant::now()`.
    pub fn record(&self, now: Instant) -> RateDecision {
        let mut inner = self.lock();
        // Evict timestamps older than `now - window`. `Instant`
        // is monotonic on every supported platform, but we use
        // `saturating_duration_since` to defend against a clock
        // step inside the same process (extremely rare, but a
        // free invariant to preserve).
        let window = inner.window;
        while let Some(front) = inner.admitted.front() {
            if now.saturating_duration_since(*front) > window {
                inner.admitted.pop_front();
            } else {
                break;
            }
        }
        if inner.admitted.len() < inner.capacity {
            inner.admitted.push_back(now);
            let pending_drops = inner.pending_drops;
            inner.pending_drops = 0;
            RateDecision::Allow { pending_drops }
        } else {
            inner.pending_drops = inner.pending_drops.saturating_add(1);
            RateDecision::Throttle {
                drops: inner.pending_drops,
            }
        }
    }

    /// Number of events admitted within the window at `now`.
    /// Diagnostic surface for tests and telemetry; not part of
    /// the hot path.
    #[must_use]
    pub fn admitted_at(&self, now: Instant) -> usize {
        let mut inner = self.lock();
        let window = inner.window;
        while let Some(front) = inner.admitted.front() {
            if now.saturating_duration_since(*front) > window {
                inner.admitted.pop_front();
            } else {
                break;
            }
        }
        inner.admitted.len()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        // Mutex poisoning here would indicate a panic mid-record;
        // the inner state is a `VecDeque<Instant>` + counters, no
        // torn-write hazard. Taking the poisoned guard is safe.
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn events_within_capacity_are_admitted() {
        let rw = RateWindow::new(3, Duration::from_secs(1));
        let now = Instant::now();
        assert_eq!(rw.record(now), RateDecision::Allow { pending_drops: 0 });
        assert_eq!(
            rw.record(now + Duration::from_millis(10)),
            RateDecision::Allow { pending_drops: 0 }
        );
        assert_eq!(
            rw.record(now + Duration::from_millis(20)),
            RateDecision::Allow { pending_drops: 0 }
        );
    }

    #[test]
    fn fourth_event_above_cap_is_throttled() {
        let rw = RateWindow::new(3, Duration::from_secs(1));
        let now = Instant::now();
        for _ in 0..3 {
            assert!(matches!(rw.record(now), RateDecision::Allow { .. }));
        }
        let decision = rw.record(now);
        assert_eq!(decision, RateDecision::Throttle { drops: 1 });
    }

    #[test]
    fn consecutive_throttles_accumulate_drops() {
        let rw = RateWindow::new(2, Duration::from_secs(1));
        let now = Instant::now();
        rw.record(now);
        rw.record(now);
        assert_eq!(rw.record(now), RateDecision::Throttle { drops: 1 });
        assert_eq!(rw.record(now), RateDecision::Throttle { drops: 2 });
        assert_eq!(rw.record(now), RateDecision::Throttle { drops: 3 });
    }

    #[test]
    fn allow_after_throttle_carries_pending_drop_count() {
        let rw = RateWindow::new(1, Duration::from_secs(1));
        let now = Instant::now();
        rw.record(now); // admitted
        rw.record(now); // throttled (1)
        rw.record(now); // throttled (2)
        rw.record(now); // throttled (3)
        // After the window expires, the next record should Allow
        // and report the cumulative drops.
        let later = now + Duration::from_secs(2);
        assert_eq!(rw.record(later), RateDecision::Allow { pending_drops: 3 });
    }

    #[test]
    fn allow_resets_pending_drops_to_zero() {
        let rw = RateWindow::new(1, Duration::from_secs(1));
        let now = Instant::now();
        rw.record(now); // admitted
        rw.record(now); // throttled (1)
        let later = now + Duration::from_secs(2);
        // First allow after the burst reports the drops...
        assert_eq!(rw.record(later), RateDecision::Allow { pending_drops: 1 });
        // ...wait past the window again so the next event is also
        // admitted, and assert the counter has reset.
        let later2 = later + Duration::from_secs(2);
        assert_eq!(rw.record(later2), RateDecision::Allow { pending_drops: 0 });
    }

    #[test]
    fn window_slides_admitting_new_events_as_old_ones_expire() {
        let rw = RateWindow::new(2, Duration::from_millis(100));
        let t0 = Instant::now();
        rw.record(t0);
        rw.record(t0 + Duration::from_millis(10));
        // At t0+50ms, both still inside the window → throttled.
        assert_eq!(
            rw.record(t0 + Duration::from_millis(50)),
            RateDecision::Throttle { drops: 1 }
        );
        // At t0+150ms, the first event has expired → admit again.
        // The throttled event from t0+50ms left drops=1 pending.
        assert_eq!(
            rw.record(t0 + Duration::from_millis(150)),
            RateDecision::Allow { pending_drops: 1 }
        );
    }

    #[test]
    fn zero_capacity_is_clamped_to_one() {
        let rw = RateWindow::new(0, Duration::from_secs(1));
        let now = Instant::now();
        // Clamp lifts capacity to 1, so one event passes...
        assert!(matches!(rw.record(now), RateDecision::Allow { .. }));
        // ...and the second is throttled.
        assert!(matches!(rw.record(now), RateDecision::Throttle { .. }));
    }

    #[test]
    fn admitted_at_reports_current_inflight_count() {
        let rw = RateWindow::new(5, Duration::from_millis(100));
        let t0 = Instant::now();
        rw.record(t0);
        rw.record(t0 + Duration::from_millis(10));
        rw.record(t0 + Duration::from_millis(20));
        assert_eq!(rw.admitted_at(t0 + Duration::from_millis(30)), 3);
        // After the window expires, count drops to 0.
        assert_eq!(rw.admitted_at(t0 + Duration::from_millis(200)), 0);
    }

    /// Burst stress: 1000 record calls in a tight loop with a
    /// cap of 50 → exactly 50 admitted, 950 throttled.
    #[test]
    fn burst_of_a_thousand_at_cap_fifty_admits_fifty() {
        let rw = RateWindow::new(50, Duration::from_secs(10));
        let now = Instant::now();
        let mut allowed = 0;
        let mut throttled = 0;
        for _ in 0..1000 {
            match rw.record(now) {
                RateDecision::Allow { .. } => allowed += 1,
                RateDecision::Throttle { .. } => throttled += 1,
            }
        }
        assert_eq!(allowed, 50);
        assert_eq!(throttled, 950);
    }

    /// Concurrent record under a single shared window — multiple
    /// threads contribute to the same cap. Total `Allow`s across
    /// threads cannot exceed the capacity within the window.
    #[test]
    fn concurrent_records_share_the_window_cap() {
        const THREADS: usize = 8;
        const PER_THREAD: usize = 200;
        let rw = Arc::new(RateWindow::new(20, Duration::from_secs(5)));
        let mut handles = Vec::with_capacity(THREADS);
        let now = Instant::now();
        for _ in 0..THREADS {
            let rw = Arc::clone(&rw);
            handles.push(thread::spawn(move || {
                let mut local_allowed = 0;
                for _ in 0..PER_THREAD {
                    if matches!(rw.record(now), RateDecision::Allow { .. }) {
                        local_allowed += 1;
                    }
                }
                local_allowed
            }));
        }
        let total_allowed: usize = handles.into_iter().map(|h| h.join().expect("join")).sum();
        assert_eq!(
            total_allowed, 20,
            "no more than `capacity` events admitted within a window across threads"
        );
    }
}
