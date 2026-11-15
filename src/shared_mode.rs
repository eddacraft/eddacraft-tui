//! Reference-counted ownership of one process-wide terminal mode (GH #6557).
//!
//! Raw mode, the alternate screen and mouse capture belong to the process, so
//! guards that switch them on may overlap: a nested surface, or two threads.
//! [`SharedMode`] counts the live holders of one mode. The first holder turns
//! the mode on and the last one turns it off; holders in between change
//! nothing, so releasing an inner guard cannot tear down an outer session.

use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard};

/// Live-holder count for one process-wide terminal mode.
///
/// The lock is held across the enter and restore calls, so a holder that
/// arrives while the first one is still switching the mode on waits for it,
/// and a release never races an acquire.
pub(crate) struct SharedMode {
    holders: Mutex<usize>,
    /// Set when something outside the holder count (a panic hook, or a
    /// manual restore) switched the mode off while holders may still be
    /// live, so the next acquire enters the mode again. An atomic rather
    /// than state under the lock: the panic hook sets it, and the panic may
    /// have unwound out of a call that holds the lock on this thread.
    forced_off: AtomicBool,
}

impl SharedMode {
    pub(crate) const fn new() -> Self {
        Self {
            holders: Mutex::new(0),
            forced_off: AtomicBool::new(false),
        }
    }

    /// A poisoned lock still holds a valid count: the mode calls made under
    /// it report failure through `io::Result`, not by panicking.
    fn holders(&self) -> MutexGuard<'_, usize> {
        self.holders
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Become a holder, running `enter` when there was none or when the
    /// mode was [forced off](Self::mark_forced_off) under live holders. A
    /// failed `enter` adds no holder, so the next acquire tries again.
    pub(crate) fn acquire(&self, enter: impl FnOnce() -> io::Result<()>) -> io::Result<()> {
        let mut holders = self.holders();
        let forced_off = self.forced_off.swap(false, Ordering::SeqCst);
        if (*holders == 0 || forced_off)
            && let Err(error) = enter()
        {
            // Live holders are still outside the mode; keep the mark so
            // the next acquire retries the entry.
            if forced_off && *holders > 0 {
                self.forced_off.store(true, Ordering::SeqCst);
            }
            return Err(error);
        }
        *holders += 1;
        Ok(())
    }

    /// Stop being a holder, running `restore` when this is the last one.
    ///
    /// The holder is retired whatever `restore` returns. When `restore`
    /// fails, `cleanup` runs best effort under the same lock before the
    /// holder goes, so no acquire can join a half-restored mode, and the
    /// original `restore` error is returned. Only `TerminalGuard` reports
    /// restore errors.
    #[cfg(any(test, feature = "lifecycle"))]
    pub(crate) fn release(
        &self,
        restore: impl FnOnce() -> io::Result<()>,
        cleanup: impl FnOnce(),
    ) -> io::Result<()> {
        let mut holders = self.holders();
        let result = if *holders == 1 {
            restore().inspect_err(|_| cleanup())
        } else {
            Ok(())
        };
        *holders = holders.saturating_sub(1);
        result
    }

    /// Stop being a holder, running the infallible `restore` when this is
    /// the last one. For `Drop`, where errors cannot be reported.
    pub(crate) fn release_best_effort(&self, restore: impl FnOnce()) {
        let mut holders = self.holders();
        if *holders == 1 {
            restore();
        }
        *holders = holders.saturating_sub(1);
    }

    /// Record that the mode was switched off outside the holder count, so
    /// the next acquire enters it again even while holders are live. Takes
    /// no lock, so the panic hook can call it.
    #[cfg(any(test, feature = "lifecycle"))]
    pub(crate) fn mark_forced_off(&self) {
        self.forced_off.store(true, Ordering::SeqCst);
    }

    #[cfg(test)]
    pub(crate) fn live_holders(&self) -> usize {
        *self.holders()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::{Arc, Barrier};

    use super::*;

    /// A fake terminal mode that records how often it was switched and fails
    /// if it is switched on twice or off while already off.
    #[derive(Default)]
    struct FakeMode {
        on: AtomicBool,
        enters: AtomicUsize,
        restores: AtomicUsize,
    }

    // `io::Result` so each method drops straight into `acquire` / `release`.
    #[allow(clippy::unnecessary_wraps)]
    impl FakeMode {
        fn enter(&self) -> io::Result<()> {
            assert!(!self.on.swap(true, Ordering::SeqCst), "entered twice");
            self.enters.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        fn restore(&self) -> io::Result<()> {
            assert!(self.on.swap(false, Ordering::SeqCst), "restored while off");
            self.restores.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        fn is_on(&self) -> bool {
            self.on.load(Ordering::SeqCst)
        }

        fn counts(&self) -> (usize, usize) {
            (
                self.enters.load(Ordering::SeqCst),
                self.restores.load(Ordering::SeqCst),
            )
        }
    }

    #[test]
    fn only_the_last_holder_restores() {
        let shared = SharedMode::new();
        let mode = FakeMode::default();

        shared.acquire(|| mode.enter()).expect("outer");
        shared.acquire(|| mode.enter()).expect("inner");
        shared
            .release(|| mode.restore(), || {})
            .expect("inner leave");
        assert!(mode.is_on(), "releasing the inner holder restored the mode");

        shared.acquire(|| mode.enter()).expect("second inner");
        shared.release_best_effort(|| mode.restore().expect("restore"));
        assert!(mode.is_on(), "dropping the inner holder restored the mode");

        shared
            .release(|| mode.restore(), || {})
            .expect("outer leave");
        assert!(!mode.is_on(), "the last holder did not restore the mode");
        assert_eq!(mode.counts(), (1, 1));
    }

    #[test]
    fn the_mode_is_entered_again_after_a_full_release() {
        let shared = SharedMode::new();
        let mode = FakeMode::default();

        shared.acquire(|| mode.enter()).expect("first");
        shared
            .release(|| mode.restore(), || {})
            .expect("first leave");
        shared.acquire(|| mode.enter()).expect("second");
        assert!(mode.is_on());
        shared.release_best_effort(|| mode.restore().expect("restore"));
        assert_eq!(mode.counts(), (2, 2));
    }

    #[test]
    fn a_failed_enter_adds_no_holder() {
        let shared = SharedMode::new();
        let mode = FakeMode::default();

        let error = shared
            .acquire(|| Err(io::Error::other("no tty")))
            .expect_err("enter failure surfaces");
        assert_eq!(error.to_string(), "no tty");

        // With no holder left behind, the next acquire enters the mode.
        shared.acquire(|| mode.enter()).expect("retry");
        assert!(mode.is_on());
        shared.release(|| mode.restore(), || {}).expect("leave");
        assert!(!mode.is_on());
    }

    /// Raw mode and the alternate screen as two parts of one session, so a
    /// restore can fail half-way.
    #[derive(Default)]
    struct Session {
        alt: FakeMode,
        raw: FakeMode,
    }

    impl Session {
        fn enter(&self) -> io::Result<()> {
            self.raw.enter()?;
            self.alt.enter()
        }

        /// Leaves the alternate screen, then fails to disable raw mode.
        fn restore_failing_raw(&self) -> io::Result<()> {
            self.alt.restore()?;
            Err(io::Error::other("raw mode stuck"))
        }

        /// Best-effort restore of whichever parts are still on.
        fn cleanup(&self) {
            if self.alt.is_on() {
                self.alt.restore().expect("alt cleanup");
            }
            if self.raw.is_on() {
                self.raw.restore().expect("raw cleanup");
            }
        }

        fn is_off(&self) -> bool {
            !self.alt.is_on() && !self.raw.is_on()
        }
    }

    #[test]
    fn a_failed_restore_cleans_up_and_retires_the_holder() {
        let shared = SharedMode::new();
        let session = Session::default();

        shared.acquire(|| session.enter()).expect("enter");
        let error = shared
            .release(|| session.restore_failing_raw(), || session.cleanup())
            .expect_err("restore failure surfaces");
        assert_eq!(error.to_string(), "raw mode stuck", "original error kept");
        assert!(
            session.is_off(),
            "the half-restored session was not cleaned up"
        );
        assert_eq!(shared.live_holders(), 0, "the failed holder was kept");

        // An acquire right after the failed release (before the departing
        // guard's `Drop` could run) enters the session again rather than
        // joining a half-restored one.
        shared.acquire(|| session.enter()).expect("re-enter");
        assert!(session.alt.is_on() && session.raw.is_on());
        assert_eq!(shared.live_holders(), 1);
        assert_eq!(session.alt.counts(), (2, 1));
        shared
            .release(|| session.alt.restore().and(session.raw.restore()), || {})
            .expect("leave");
        assert!(session.is_off());
    }

    #[test]
    fn a_failed_restore_under_other_holders_runs_nothing() {
        let shared = SharedMode::new();
        let mode = FakeMode::default();

        shared.acquire(|| mode.enter()).expect("outer");
        shared.acquire(|| mode.enter()).expect("inner");
        shared
            .release(
                || panic!("restore ran under a live holder"),
                || panic!("cleanup ran under a live holder"),
            )
            .expect("inner leave");
        assert!(mode.is_on());
        shared
            .release(|| mode.restore(), || {})
            .expect("outer leave");
        assert!(!mode.is_on());
    }

    #[test]
    fn a_forced_restore_makes_the_next_holder_enter_again() {
        let shared = SharedMode::new();
        let mode = FakeMode::default();

        shared.acquire(|| mode.enter()).expect("outer");
        // A panic hook or `restore_terminal` switches the mode off while the
        // outer holder survives.
        mode.restore().expect("forced restore");
        shared.mark_forced_off();

        shared.acquire(|| mode.enter()).expect("inner");
        assert!(mode.is_on(), "the inner holder joined a restored mode");
        shared
            .release(|| mode.restore(), || {})
            .expect("inner leave");
        assert!(mode.is_on(), "the inner holder restored under the outer");

        // The mark is consumed: a further holder joins without entering.
        shared.acquire(|| mode.enter()).expect("second inner");
        shared.release_best_effort(|| mode.restore().expect("restore"));
        shared
            .release(|| mode.restore(), || {})
            .expect("outer leave");
        assert!(!mode.is_on());
        assert_eq!(mode.counts(), (2, 2));
        assert_eq!(shared.live_holders(), 0);
    }

    #[test]
    fn a_failed_reentry_after_a_forced_restore_retries() {
        let shared = SharedMode::new();
        let mode = FakeMode::default();

        shared.acquire(|| mode.enter()).expect("outer");
        mode.restore().expect("forced restore");
        shared.mark_forced_off();

        shared
            .acquire(|| Err(io::Error::other("no tty")))
            .expect_err("re-entry failure surfaces");
        assert_eq!(
            shared.live_holders(),
            1,
            "the failed re-entry added a holder"
        );

        shared.acquire(|| mode.enter()).expect("retry");
        assert!(mode.is_on(), "the retry did not re-enter the mode");
        shared
            .release(|| mode.restore(), || {})
            .expect("inner leave");
        shared
            .release(|| mode.restore(), || {})
            .expect("outer leave");
        assert!(!mode.is_on());
    }

    #[test]
    fn concurrent_holders_enter_and_restore_once() {
        const THREADS: usize = 8;
        let shared = Arc::new(SharedMode::new());
        let mode = Arc::new(FakeMode::default());
        let all_held = Arc::new(Barrier::new(THREADS));

        let handles: Vec<_> = (0..THREADS)
            .map(|index| {
                let (shared, mode, all_held) = (
                    Arc::clone(&shared),
                    Arc::clone(&mode),
                    Arc::clone(&all_held),
                );
                std::thread::spawn(move || {
                    shared.acquire(|| mode.enter()).expect("acquire");
                    all_held.wait();
                    // Every holder sees the mode on until it lets go, however
                    // the releases interleave.
                    assert!(mode.is_on(), "another holder restored the mode early");
                    if index % 2 == 0 {
                        shared.release(|| mode.restore(), || {}).expect("release");
                    } else {
                        shared.release_best_effort(|| mode.restore().expect("restore"));
                    }
                })
            })
            .collect();
        for handle in handles {
            handle.join().expect("holder thread");
        }

        assert!(!mode.is_on());
        assert_eq!(mode.counts(), (1, 1));
    }
}
