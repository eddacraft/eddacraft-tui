//! Shared rayon global-pool initialiser (V050F-007).
//!
//! Single place to size/init the pool so every rayon consumer agrees.

use std::sync::Once;

static POOL_INIT: Once = Once::new();

/// Anvil's pool-cap policy as a pure function: half the available
/// cores, with a floor of 1.
///
/// Factored out of [`init_global`] so the cap can be pinned by a unit
/// test without touching rayon's process-global pool — the global path
/// is untestable in shared-process unit tests because whichever rayon
/// consumer drives `build_global` first wins (see [`init_global`]).
///
/// The `.max(1)` floor matters: `rayon::ThreadPoolBuilder::build_global`
/// rejects a zero-thread pool, so a single-core (or a pathological
/// zero-core report) machine must still resolve to a valid pool size.
///
/// CLAWP-038 — regression coverage for Clawpatch finding
/// `fnd_sig-feat-library-8a1266b4d7-3821_ea242dc15a`.
fn cap_threads(available_cores: usize) -> usize {
    (available_cores / 2).max(1)
}

/// Initialise rayon's global thread pool with anvil's half-cores cap.
///
/// First call: builds the global pool with `(num_cpus::get() / 2).max(1)`
/// threads. Subsequent calls: no-op (the underlying `Once` short-
/// circuits). Idempotent; safe to call from any thread, any number
/// of times.
///
/// `build_global()` returns `Err` if rayon's global pool was already
/// initialised by a different code path (e.g. a scan that called
/// `par_iter` before this function ran). The error is intentionally
/// dropped — we cannot un-initialise rayon, and the pre-V050F-007
/// behaviour also silently absorbed this case. The contract this
/// function provides is "if anvil owns the first init, the cap is
/// applied"; if a non-anvil code path initialised first, anvil's
/// cap is moot anyway.
///
/// V050F-007 — flagged by kernel-maintainer (rounds 2 + 3).
pub fn init_global() {
    POOL_INIT.call_once(|| {
        let threads = cap_threads(num_cpus::get());
        let _ = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build_global();
    });
}

#[cfg(test)]
mod tests {
    use super::{cap_threads, init_global};

    /// Idempotency contract: calling [`init_global`] twice from
    /// the same process must not panic. The second call is a
    /// no-op via the underlying [`Once`].
    ///
    /// This test deliberately does NOT assert the resulting thread
    /// count — Rust's lib unit tests share a process, and a
    /// previous test may have already initialised rayon (via
    /// `scan_artifact` or any other rayon consumer), in which
    /// case `current_num_threads()` reflects whichever path won
    /// the race. The behavioural assertion lives at the
    /// CLI-entry-point level (the binary calls `init_global`
    /// FIRST, before any rayon consumer can run).
    #[test]
    fn init_global_is_idempotent() {
        init_global();
        init_global();
    }

    /// The cap policy is "half available cores, minimum 1". The
    /// global-pool path is untestable in shared-process unit tests
    /// (whichever rayon consumer runs first wins the `build_global`
    /// race), so [`init_global`] delegates its thread count to the
    /// pure [`cap_threads`] helper and we pin the policy here.
    ///
    /// Regression coverage for CLAWP-038 (Clawpatch finding
    /// `fnd_sig-feat-library-8a1266b4d7-3821_ea242dc15a`): the prior
    /// sole test only asserted no-panic by implication and never
    /// pinned the cap.
    #[test]
    fn cap_threads_is_half_cores_minimum_one() {
        // Half-cores rounding (integer division).
        assert_eq!(cap_threads(8), 4);
        assert_eq!(cap_threads(7), 3);
        assert_eq!(cap_threads(2), 1);

        // The `.max(1)` floor: even on a single core or the
        // pathological zero-core report, anvil never asks rayon
        // for a zero-thread pool (which `build_global` rejects).
        assert_eq!(cap_threads(1), 1);
        assert_eq!(cap_threads(0), 1);
    }
}
