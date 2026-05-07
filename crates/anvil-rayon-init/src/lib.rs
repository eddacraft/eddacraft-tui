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
//!   each entry point so the editor host inherits the cap.
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
        let threads = (num_cpus::get() / 2).max(1);
        let _ = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build_global();
    });
}

#[cfg(test)]
mod tests {
    use super::init_global;

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
}
