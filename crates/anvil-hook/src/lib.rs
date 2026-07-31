//! Hook surface primitives (MLP-003).
//!
//! Shared concerns for every `anvil hook <name>`: SHA validation, exit
//! mapping, and policy wiring helpers.

mod bootstrap;
mod coexistence;
mod framework;
mod panic;
mod post;
mod pre_push;
mod shell;
mod suppression;
mod verdict;

pub use bootstrap::{
    BOOTSTRAP_RECOVERY_VALIDATION_AT, BootstrapPlan, HuskyRuntime, PlainHookFile,
    build_bootstrap_plan, generate_husky_runtime, render_success_message,
};
pub use coexistence::{
    CoexistenceError, CoexistenceFile, CoexistencePlan, MARKER_BEGIN, MARKER_END, apply,
    plan_install, plan_uninstall,
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
