//! Reference-counted ownership of one process-wide terminal mode (GH #6557).
//!
//! Raw mode, the alternate screen and mouse capture belong to the process, so
//! guards that switch them on may overlap: a nested surface, or two threads.
//! [`SharedMode`] counts the live holders of one mode. The first holder turns
//! the mode on and the last one turns it off; holders in between change
//! nothing, so releasing an inner guard cannot tear down an outer session.

use std::io;
use std::sync::{Mutex, MutexGuard};

/// Live-holder count for one process-wide terminal mode.
///
/// The lock is held across the enter and restore calls, so a holder that
/// arrives while the first one is still switching the mode on waits for it,
/// and a release never races an acquire.
pub(crate) struct SharedMode {
    holders: Mutex<usize>,
}

impl SharedMode {
    pub(crate) const fn new() -> Self {
        Self {
            holders: Mutex::new(0),
        }
    }

    /// A poisoned lock still holds a valid count: the mode calls made under
    /// it report failure through `io::Result`, not by panicking.
    fn holders(&self) -> MutexGuard<'_, usize> {
        self.holders
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Become a holder, running `enter` when there was none. A failed
    /// `enter` adds no holder, so the next acquire tries again.
    pub(crate) fn acquire(&self, enter: impl FnOnce() -> io::Result<()>) -> io::Result<()> {
        let mut holders = self.holders();
        if *holders == 0 {
            enter()?;
        }
        *holders += 1;
        Ok(())
    }

    /// Stop being a holder, running `restore` when this is the last one. A
    /// failed `restore` keeps the holder, so a later best-effort release
    /// still restores the mode. Only `TerminalGuard` reports restore errors.
    #[cfg(any(test, feature = "lifecycle"))]
    pub(crate) fn release(&self, restore: impl FnOnce() -> io::Result<()>) -> io::Result<()> {
        let mut holders = self.holders();
        if *holders == 1 {
            restore()?;
        }
        *holders = holders.saturating_sub(1);
        Ok(())
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
        shared.release(|| mode.restore()).expect("inner leave");
        assert!(mode.is_on(), "releasing the inner holder restored the mode");

        shared.acquire(|| mode.enter()).expect("second inner");
        shared.release_best_effort(|| mode.restore().expect("restore"));
        assert!(mode.is_on(), "dropping the inner holder restored the mode");

        shared.release(|| mode.restore()).expect("outer leave");
        assert!(!mode.is_on(), "the last holder did not restore the mode");
        assert_eq!(mode.counts(), (1, 1));
    }

    #[test]
    fn the_mode_is_entered_again_after_a_full_release() {
        let shared = SharedMode::new();
        let mode = FakeMode::default();

        shared.acquire(|| mode.enter()).expect("first");
        shared.release(|| mode.restore()).expect("first leave");
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
        shared.release(|| mode.restore()).expect("leave");
        assert!(!mode.is_on());
    }

    #[test]
    fn a_failed_restore_keeps_the_holder_for_the_drop_retry() {
        let shared = SharedMode::new();
        let mode = FakeMode::default();

        shared.acquire(|| mode.enter()).expect("enter");
        let error = shared
            .release(|| Err(io::Error::other("write failed")))
            .expect_err("restore failure surfaces");
        assert_eq!(error.to_string(), "write failed");
        assert!(mode.is_on());

        shared.release_best_effort(|| mode.restore().expect("restore"));
        assert!(!mode.is_on(), "the drop retry did not restore the mode");
        assert_eq!(mode.counts(), (1, 1));
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
                        shared.release(|| mode.restore()).expect("release");
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
