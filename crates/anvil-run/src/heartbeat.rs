//! INTL-009: liveness heartbeat for an active session.
//!
//! The daemon's session registry (INTD-003) evicts records that
//! have not heartbeated within a 30s TTL. The launcher emits a
//! heartbeat every [`HEARTBEAT_INTERVAL`] (10s by default — well
//! inside the eviction window) on a background thread; the thread
//! exits when the [`HeartbeatHandle`] is dropped.
//!
//! Errors emitting a single heartbeat are logged but do not tear
//! the thread down — a transient socket-busy condition should not
//! cost the session its TTL.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use anvil_intercept_proto::SessionId;

/// Default heartbeat cadence. Daemon TTL is 30s; this leaves room
/// for at least two missed heartbeats before eviction.
pub const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(10);

/// Handle on a running heartbeat thread. Drop to stop the thread.
pub struct HeartbeatHandle {
    stop: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
}

impl HeartbeatHandle {
    /// Start a heartbeat thread that emits a beat every `interval`
    /// for `session_id`. The closure is called once per tick so
    /// tests can substitute the daemon round-trip.
    pub fn spawn<F>(session_id: SessionId, interval: Duration, mut tick: F) -> Self
    where
        F: FnMut(&SessionId) + Send + 'static,
    {
        let stop = Arc::new(AtomicBool::new(false));
        let stop_clone = stop.clone();
        let join = thread::spawn(move || {
            // Honour `stop` with sub-interval responsiveness so
            // tests do not have to wait a full tick to observe
            // shutdown. We poll a fixed 50ms cadence and only
            // dispatch when an interval has elapsed.
            let poll = Duration::from_millis(50);
            let mut last = Instant::now() - interval;
            while !stop_clone.load(Ordering::Acquire) {
                if last.elapsed() >= interval {
                    tick(&session_id);
                    last = Instant::now();
                }
                thread::sleep(poll);
            }
        });
        Self {
            stop,
            join: Some(join),
        }
    }

    /// Start the production heartbeat thread that dispatches
    /// through [`crate::session::heartbeat`].
    pub fn spawn_default(session_id: SessionId) -> Self {
        Self::spawn(session_id, HEARTBEAT_INTERVAL, |id| {
            if let Err(err) = crate::session::heartbeat(id) {
                let mut stderr = std::io::stderr().lock();
                let _ = std::io::Write::write_all(
                    &mut stderr,
                    format!("anvil-run: heartbeat for {} failed: {err}\n", id.as_str(),).as_bytes(),
                );
            }
        })
    }

    /// Stop the thread and block until it exits.
    pub fn stop(mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(handle) = self.join.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for HeartbeatHandle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(handle) = self.join.take() {
            let _ = handle.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[test]
    fn heartbeat_thread_fires_at_least_once_within_a_short_window() {
        let calls = Arc::new(Mutex::new(0_u32));
        let calls_clone = calls.clone();
        let handle = HeartbeatHandle::spawn(
            SessionId::new("sess_h"),
            Duration::from_millis(20),
            move |_| {
                *calls_clone.lock().unwrap() += 1;
            },
        );
        // Wait long enough for several ticks but stop before the
        // test times out.
        thread::sleep(Duration::from_millis(200));
        handle.stop();
        let observed = *calls.lock().unwrap();
        assert!(
            observed >= 2,
            "expected the heartbeat thread to fire multiple times within 200ms; got {observed}",
        );
    }

    #[test]
    fn dropping_the_handle_stops_the_thread() {
        let calls = Arc::new(Mutex::new(0_u32));
        let calls_clone = calls.clone();
        {
            let _handle = HeartbeatHandle::spawn(
                SessionId::new("sess_h"),
                Duration::from_millis(20),
                move |_| {
                    *calls_clone.lock().unwrap() += 1;
                },
            );
            thread::sleep(Duration::from_millis(80));
        }
        let after_drop = *calls.lock().unwrap();
        thread::sleep(Duration::from_millis(200));
        let later = *calls.lock().unwrap();
        assert_eq!(
            after_drop, later,
            "no further ticks should fire after handle drop",
        );
    }
}
