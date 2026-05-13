//! Anvil hook surface primitives (MLP-003).
//!
//! Owns the cross-cutting hook concerns that every `anvil hook <name>`
//! subcommand will need, factored out into a self-contained library
//! so the CLI integration (lands in a follow-up) stays thin and the
//! pure logic is unit-testable without spawning a real hook process.
//!
//! ## Scope (MLP-003 v1 library)
//!
//! - [`Verdict`] / [`render_verdict`] — the ADR-038 §D-1 noise-
//!   discipline contract. Pure function: a verdict in, one line of
//!   terse stderr (or silence) out. Includes exit-code policy from
//!   §D-6 (validation block = exit 1; all other failures, including
//!   internal panic, = exit 0).
//! - [`SuppressionLog`] — ADR-038 repeat-suppression. Same
//!   `(class, detail)` won't re-emit within a session; the
//!   `daemon-down` message fires once, not 82 times during a sub-
//!   agent burst.
//! - [`detect_framework`] — non-destructive identification of the
//!   user's existing hook framework (Husky / Lefthook / pre-commit
//!   framework / cargo-husky / plain `core.hooksPath` / nothing).
//! - [`shell_template`] — the 3-line shell wrapper from ADR-038
//!   §D-5. Per-hook (pre-commit, post-commit, pre-push, etc.).
//! - [`panic_catcher_hook`] — `std::panic::set_hook` payload that
//!   demotes a panic to a single stderr line + log file + exit-0,
//!   per ADR-038 §D-7. Returns the witness-error record so the
//!   caller can append it to the chain.
//!
//! ## Out of scope (deferred to consumers / CLI lane)
//!
//! - `anvil hook <name>` subcommands themselves — owned by
//!   `crates/anvil-cli/src/commands/hook.rs`, which threads the
//!   primitives here with anvil-checks (validation), anvil-witness
//!   (append), anvil-baseline (filter accepted findings), and
//!   anvil-rules (`rules_sha`).
//! - Framework-specific install paths — owned by MLP-008 (`anvil
//!   hook bootstrap`).
//! - Witness append integration — owned by MLP-002's writer + the
//!   CLI subcommand call sites.
//! - Daemon RPC path / embedded fallback — owned by `anvil-intercept`
//!   client side; the hook surface here is daemon-agnostic.
//!
//! ## ADR-038 cross-reference
//!
//! - §D-1 Serena rule → [`Verdict`] / [`render_verdict`].
//! - §D-3 hook surface → [`HookKind`].
//! - §D-4 framework integration → [`detect_framework`].
//! - §D-5 self-contained binary → [`shell_template`].
//! - §D-6 failure-mode taxonomy → [`render_verdict`] exit-code map.
//! - §D-7 panic catcher → [`panic_catcher_hook`].

mod bootstrap;
mod framework;
mod panic;
mod post;
mod pre_push;
mod shell;
mod suppression;
mod verdict;

pub use bootstrap::{
    BootstrapPlan, HuskyRuntime, PlainHookFile, build_bootstrap_plan, generate_husky_runtime,
    render_success_message,
};
pub use framework::{HookFramework, detect_framework};
pub use panic::{PANIC_LOG_FILE, PanicReport, format_panic_report, panic_catcher_hook};
pub use post::{
    MergeWitnessPlan, POST_REWRITE_VALIDATION_AT, PostRewriteParseError, RetroactiveWitness,
    RewritePair, merge_witness_plan, parse_post_rewrite_input,
};
pub use pre_push::{
    PrePushParseError, PushKind, PushRef, ZERO_SHA, is_hex_sha, is_zero_sha, parse_pre_push_input,
};
pub use shell::{HookKind, shell_template};
pub use suppression::{SuppressionKey, SuppressionLog};
pub use verdict::{BlockReason, ErrorClass, RenderedVerdict, Verdict, render_verdict};
