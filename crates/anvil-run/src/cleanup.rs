//! INTL-005: session cleanup on every exit path.
//!
//! The launcher must unregister the session with the daemon whether
//! the child exits normally, takes a signal, or the launcher itself
//! crashes between `register` and `wait_for_child`. Implemented as
//! a [`Drop`] guard so the cleanup path is automatic — no matter
//! how the calling function unwinds, the destructor sends
//! `session.unregister`.
//!
//! The guard is intentionally tolerant of daemon errors: if the
//! daemon went away while the child was running there is nothing
//! useful to retry, so we record the failure on stderr and move on.

use anvil_intercept_proto::SessionId;

use crate::session;

/// Cleanup guard around a registered session id. Drop sends a
/// `session.unregister` request. The guard can be `disarm`-ed when
/// cleanup has already happened on a normal path so the destructor
/// becomes a no-op.
/// Pluggable drop hook. Default destructor calls
/// [`session::unregister`]; tests can substitute a recording double
/// via [`SessionGuard::with_drop_hook`].
type DropHook = Box<dyn FnMut(&SessionId) + Send + 'static>;

pub struct SessionGuard {
    session_id: Option<SessionId>,
    on_drop: Option<DropHook>,
}

impl SessionGuard {
    /// Arm a guard. Cleanup runs unless [`SessionGuard::disarm`] is
    /// called first.
    #[must_use]
    pub fn arm(session_id: SessionId) -> Self {
        Self {
            session_id: Some(session_id),
            on_drop: None,
        }
    }

    /// Arm a guard with a test override. The destructor will call
    /// `on_drop` rather than talking to the daemon.
    #[must_use]
    pub fn with_drop_hook<F>(session_id: SessionId, on_drop: F) -> Self
    where
        F: FnMut(&SessionId) + Send + 'static,
    {
        Self {
            session_id: Some(session_id),
            on_drop: Some(Box::new(on_drop)),
        }
    }

    /// Disarm the guard. After this call the destructor is a
    /// no-op. Use when cleanup has already run on a normal path.
    pub fn disarm(mut self) {
        self.session_id.take();
        // `self.on_drop` would no-op anyway once `session_id` is
        // None — but drop the closure now so it cannot capture
        // resources past the disarm point.
        self.on_drop.take();
    }
}

impl Drop for SessionGuard {
    fn drop(&mut self) {
        let Some(id) = self.session_id.take() else {
            return;
        };
        if let Some(hook) = self.on_drop.as_mut() {
            hook(&id);
            return;
        }
        if let Err(err) = session::unregister(&id) {
            // Stderr rather than `eprintln!` directly so cargo test
            // captures stay readable; the message is the only signal
            // an operator gets that the daemon never saw the
            // unregister.
            let mut stderr = std::io::stderr().lock();
            let _ = std::io::Write::write_all(
                &mut stderr,
                format!(
                    "anvil-run: failed to unregister session {}: {err}\n",
                    id.as_str(),
                )
                .as_bytes(),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn guard_drop_invokes_hook_with_the_session_id() {
        let calls = Arc::new(Mutex::new(Vec::<String>::new()));
        let calls_clone = calls.clone();
        {
            let _guard = SessionGuard::with_drop_hook(SessionId::new("sess_x"), move |id| {
                calls_clone.lock().unwrap().push(id.as_str().to_owned());
            });
        }
        let recorded = calls.lock().unwrap();
        assert_eq!(*recorded, vec!["sess_x".to_owned()]);
    }

    #[test]
    fn guard_disarm_suppresses_the_hook() {
        let calls = Arc::new(Mutex::new(Vec::<String>::new()));
        let calls_clone = calls.clone();
        let guard = SessionGuard::with_drop_hook(SessionId::new("sess_x"), move |id| {
            calls_clone.lock().unwrap().push(id.as_str().to_owned());
        });
        guard.disarm();
        assert!(
            calls.lock().unwrap().is_empty(),
            "disarmed guard must not run the hook",
        );
    }

    #[test]
    fn guard_drop_runs_exactly_once_when_armed() {
        let count = Arc::new(Mutex::new(0_u32));
        let count_clone = count.clone();
        {
            let _guard = SessionGuard::with_drop_hook(SessionId::new("sess_once"), move |_| {
                *count_clone.lock().unwrap() += 1;
            });
        }
        assert_eq!(*count.lock().unwrap(), 1);
    }
}
