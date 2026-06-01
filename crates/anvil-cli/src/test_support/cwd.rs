//! Shared serialisation guard for tests that mutate the process-global
//! current working directory (`std::env::set_current_dir`).
//!
//! Cargo runs unit and integration tests on a shared thread pool, so any
//! two tests that swap the process cwd concurrently corrupt each other's
//! relative-path resolution. Historically each module (`check.rs`,
//! `doctor.rs`, `validate_write.rs`) carried its own independent `Mutex`,
//! which meant they serialised *within* a module but raced *across*
//! modules. CIB-026 collapses them onto this single workspace-wide guard
//! so every cwd-mutating test path serialises against every other.
//!
//! Use [`with_cwd_in`] for the common case: it locks the guard, swaps the
//! process cwd to `dir`, runs `body`, and restores the original cwd via an
//! RAII drop — even if `body` panics. The mutex is recovered from
//! [`std::sync::PoisonError`] so a panicking test does not wedge every
//! later cwd test behind a poisoned lock.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

/// Process-wide guard serialising every cwd-mutating test in the crate.
static CWD_GUARD: Mutex<()> = Mutex::new(());

/// Restores the captured cwd on drop so the test runner's working
/// directory is always reinstated — including on panic, where the drop
/// runs during unwinding.
struct CwdRestore(PathBuf);

impl Drop for CwdRestore {
    fn drop(&mut self) {
        // Best-effort: if the original dir is gone there is nothing we can
        // do, and panicking inside a drop during unwinding would abort.
        let _ = std::env::set_current_dir(&self.0);
    }
}

/// Run `body` with the process cwd swapped to `dir`, restoring the
/// original cwd on return and on panic. **Caller must already hold
/// [`CWD_GUARD`]** — this is the unlocked primitive shared by
/// [`with_cwd_in`] and by the guard's own self-tests, which need to read
/// the surrounding cwd under the same lock hold rather than racing for it.
fn swap_cwd_in<R>(dir: &Path, body: impl FnOnce() -> R) -> R {
    let original = std::env::current_dir().expect("test runner has a readable cwd");
    std::env::set_current_dir(dir).expect("cd into target dir");
    // `_restore` reinstates the original cwd when it drops — including
    // during unwinding if `body` panics.
    let _restore = CwdRestore(original);
    body()
}

/// Run `body` with the process cwd swapped to `dir`, serialised against
/// every other caller of this helper via the shared [`CWD_GUARD`].
///
/// The original cwd is restored before this function returns, even if
/// `body` panics. A poisoned guard (from an earlier panicking test) is
/// recovered rather than propagated, so one failing test does not cascade
/// into spurious failures for every later cwd test.
pub fn with_cwd_in<R>(dir: &Path, body: impl FnOnce() -> R) -> R {
    let _lock = CWD_GUARD.lock().unwrap_or_else(PoisonError::into_inner);
    // Drop order: `swap_cwd_in`'s internal `_restore` drops before `_lock`,
    // so the cwd is reinstated before the mutex is released and any other
    // test can grab the lock.
    swap_cwd_in(dir, body)
}

#[cfg(test)]
mod tests {
    use super::*;

    // These self-tests must read the surrounding cwd *under* `CWD_GUARD`,
    // not via `with_cwd_in` alone: capturing the "original" cwd or
    // asserting the restored cwd while unlocked lets a concurrent
    // cwd-mutating test (which has swapped the process cwd to its own
    // tempdir) be observed as our baseline, producing a deterministic
    // false failure. Holding the guard for the whole capture → act →
    // assert sequence serialises us against every other cwd test, and we
    // drive the swap through the unlocked [`swap_cwd_in`] primitive.

    #[test]
    fn restores_cwd_after_body_returns() {
        let _lock = CWD_GUARD.lock().unwrap_or_else(PoisonError::into_inner);
        let original = std::env::current_dir().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        // `canonicalize` because macOS tempdirs live under a symlinked
        // `/var` → `/private/var`, so the raw tempdir path won't compare
        // equal to the post-cd `current_dir`.
        let expected = tmp.path().canonicalize().unwrap();

        // Canonicalize the observed cwd too: on Windows `current_dir`
        // reports the 8.3 short form without the `\\?\` verbatim prefix
        // (e.g. `C:\Users\RUNNER~1\...`), whereas `canonicalize` returns
        // the long, verbatim-prefixed form — the same directory, but
        // unequal as raw strings. Canonicalizing both sides compares the
        // resolved path, mirroring the macOS `/var` → `/private/var` case.
        let observed = swap_cwd_in(tmp.path(), || {
            std::env::current_dir().unwrap().canonicalize().unwrap()
        });

        assert_eq!(observed, expected, "body should observe the swapped cwd");
        assert_eq!(
            std::env::current_dir().unwrap(),
            original,
            "cwd must be restored after the body returns"
        );
    }

    #[test]
    fn restores_cwd_even_when_body_panics() {
        let _lock = CWD_GUARD.lock().unwrap_or_else(PoisonError::into_inner);
        let original = std::env::current_dir().unwrap();
        let tmp = tempfile::tempdir().unwrap();

        let result = std::panic::catch_unwind(|| {
            swap_cwd_in(tmp.path(), || {
                panic!("body blew up after swapping cwd");
            })
        });

        assert!(result.is_err(), "panic should propagate out of the helper");
        assert_eq!(
            std::env::current_dir().unwrap(),
            original,
            "cwd must be restored even when the body panics"
        );
    }
}
