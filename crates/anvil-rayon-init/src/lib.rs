//! Shared rayon global-pool initialiser (V050F-007).
//!
//! Hosted as a dedicated micro-crate so every rayon consumer in the
//! workspace can depend on it without dragging unrelated kernel /
//! checks code (council finding: kernel-maintainer flagged a heavy
//! `anvil-kernel` dep on `anvil-checks-napi` for what is genuinely
//! four lines of pool init).
//!
//! Anvil caps rayon's global thread pool at half available cores
//! (minimum 1) so a long-running editor / VS Code extension host
//! coexisting with a scan does not get its UI thread starved. The
//! cap policy lives here, in one place, so every consumer of rayon
//! across the workspace can reach it via a single
//! [`init_global`] call:
//!
//! - The CLI binary entry point (`crates/anvil-cli/src/main.rs`)
//!   calls it before any subcommand runs.
//! - The kernel's own watch / embedded entry points
//!   (`watch::run_watch`, `embedded::run_embedded`) call it
//!   defensively for direct lib consumers.
//! - The NAPI binding (`crates/anvil-checks-napi`) calls it from
//!   `scan_artifact_json` (the only entry that actually drives a
//!   rayon `par_iter` via `scan_artifact_rust`); other NAPI
//!   entries (`version`, `get_default_patterns_json`,
//!   `get_pattern_json`) only read the registry and do not need
//!   the call. If a future NAPI export touches a parallel scan
//!   path, it should call `init_global` at its top.
//!
//! ## Why centralise this
//!
//! Pre-V050F-007 the kernel had two duplicated `POOL_INIT`
//! `std::sync::Once` blocks (one in `watch.rs`, one in
//! `embedded.rs`) AND `anvil-checks::antipattern::scan_artifact`
//! reached for rayon's global pool with no `build_global` of its
//! own. The first consumer to drive a `par_iter` won the race —
//! if `scan_artifact` fired first (the `anvil check` path),
//! rayon initialised the global pool to the default `num_cpus`
//! threads, and the subsequent `POOL_INIT.call_once` in `watch.rs`
//! / `embedded.rs` was a no-op. The half-cores cap was silently
//! absent on every `anvil check` run.
//!
//! The fix is structural: the binary calls [`init_global`] BEFORE
//! any command can dispatch to a rayon-using path, so the cap is
//! always in force regardless of which scan path runs first. The
//! kernel's defensive `call_once` blocks are kept (now delegating
//! here) because the kernel is also consumed as a library by
//! tests and downstream binaries that bypass `main.rs`.
//!
//! The function is idempotent: it wraps the underlying
//! `rayon::ThreadPoolBuilder::new(...).build_global()` call in a
//! [`std::sync::Once`], so repeated calls (CLI startup, kernel
//! defensive `call_once`, NAPI per-entry) cost only an atomic load
//! after the first.

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
