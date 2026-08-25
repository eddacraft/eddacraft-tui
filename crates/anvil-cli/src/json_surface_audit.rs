//! Issue #3947 (operator decision A): `--json` binds CLI-wide.
//!
//! Every terminal command path must be classified here, and the test
//! walks the real clap tree so a NEW command fails this test until its
//! author classifies it — classification is the audit trail that the
//! surface either honours `--json` or documents why no single-document
//! form exists. The categories:
//!
//! - `doc`: success stdout is exactly one JSON document under `--json`.
//! - `format-selector`: ADR-056 `--format` surface; `--json` is the
//!   compatibility alias for `--format json`, and an explicit non-JSON
//!   `--format` outranks it by documented precedence.
//! - `stream`: a documented machine stream (NDJSON / JSON-RPC); never
//!   prose. `--json` selects or preserves the stream contract.
//! - `silent`: no success stdout at all (git-/daemon-invoked or
//!   stderr-only surfaces); nothing to convert.
//! - `interactive-refusal`: interactive-only surface that refuses
//!   `--json` with a structured error instead of drawing on stdout.
//! - `delegated`: stdout belongs to a delegated child process whose
//!   output is captured or suppressed under `--json`.
//!
//! The registry is deliberately a plain table, not clap metadata, so the
//! diff in a failing run names exactly the unclassified or stale paths.

use std::collections::BTreeMap;

/// Every terminal path and its `--json` classification. Keep sorted.
const JSON_SURFACES: &[(&str, &str)] = &[
    ("admin activity", "doc"),
    ("admin approve", "doc"),
    ("admin audit", "doc"),
    ("admin auth set", "doc"),
    ("admin auth status", "doc"),
    ("admin auth unset", "doc"),
    ("admin email-send", "doc"),
    ("admin email-update", "doc"),
    ("admin fleet", "doc"),
    ("admin invite", "doc"),
    ("admin list", "doc"),
    ("admin name-update", "doc"),
    ("admin revoke", "doc"),
    ("admin send-migration", "doc"),
    ("admin show", "doc"),
    ("admin users", "doc"),
    ("architecture show", "doc"),
    ("architecture validate", "doc"),
    ("audit", "format-selector"),
    ("audit-chain", "doc"),
    ("auth login", "silent"),
    ("auth logout", "doc"),
    ("auth refresh", "doc"),
    ("auth whoami", "doc"),
    ("baseline", "doc"),
    ("baseline verify", "doc"),
    ("capsule create", "doc"),
    ("capsule explain", "format-selector"),
    ("capsule prune", "doc"),
    ("capsule verify", "format-selector"),
    ("check", "format-selector"),
    ("config convert", "doc"),
    ("config set", "doc"),
    ("config show", "doc"),
    ("dashboard", "doc"),
    ("doctor", "doc"),
    ("drift compare", "doc"),
    ("drift list", "doc"),
    ("drift migrate", "doc"),
    ("drift report", "doc"),
    ("drift snapshot", "doc"),
    ("edda list", "doc"),
    ("edda show", "doc"),
    ("ember list", "doc"),
    ("exception grant", "doc"),
    ("exception list", "doc"),
    ("exception migrate", "doc"),
    ("exception revoke", "doc"),
    ("exception show", "doc"),
    ("exception verify", "doc"),
    ("export", "doc"),
    ("gate", "format-selector"),
    ("gate-config", "doc"),
    ("gctx egress disable", "doc"),
    ("gctx egress enable", "doc"),
    ("gctx egress status", "doc"),
    ("graph-base build", "doc"),
    ("graph-base gc", "doc"),
    ("hook bootstrap", "doc"),
    ("hook post-commit", "silent"),
    ("hook post-merge", "silent"),
    ("hook post-rewrite", "silent"),
    ("hook pre-commit", "silent"),
    ("hook pre-push", "silent"),
    ("hooks install", "doc"),
    ("hooks status", "doc"),
    ("hooks uninstall", "doc"),
    ("impact", "doc"),
    ("init", "doc"),
    ("insights", "doc"),
    ("intercept start", "silent"),
    ("intercept status", "doc"),
    ("intercept stop", "doc"),
    ("intercept unblock", "doc"),
    ("kindling usage flags", "doc"),
    ("kindling usage principals", "doc"),
    ("kindling usage top", "doc"),
    ("kindling usage unused", "doc"),
    ("l4-validate", "silent"),
    ("licenses", "doc"),
    ("login", "silent"),
    ("logout", "doc"),
    ("lsp", "stream"),
    ("mcp install", "doc"),
    ("mcp pin", "doc"),
    ("mcp refresh", "doc"),
    ("mcp serve", "stream"),
    ("mcp unpin", "doc"),
    ("mcp-config", "doc"),
    ("migrate", "doc"),
    ("migrate architecture", "doc"),
    ("migrate format", "doc"),
    ("migrate gate-config", "doc"),
    ("migrate schema", "doc"),
    ("new", "doc"),
    ("plan dashboard", "doc"),
    ("policy attack-regression", "doc"),
    ("policy diff", "doc"),
    ("policy eval", "doc"),
    ("policy eval-regression", "doc"),
    ("policy explain", "doc"),
    ("policy install", "doc"),
    ("policy list", "doc"),
    ("policy members", "doc"),
    ("policy probe-trends", "doc"),
    ("policy show", "doc"),
    ("policy test", "doc"),
    ("policy validate", "doc"),
    ("report-fp", "doc"),
    ("skill install", "doc"),
    ("start", "doc"),
    ("status", "doc"),
    ("telemetry", "doc"),
    ("telemetry off", "doc"),
    ("telemetry on", "doc"),
    ("telemetry reset-id", "doc"),
    ("tutorial", "interactive-refusal"),
    ("uninstall", "doc"),
    ("update", "delegated"),
    ("validate", "doc"),
    ("version", "doc"),
    ("watch", "stream"),
    ("welcome", "doc"),
    ("whoami", "doc"),
    ("wizard", "doc"),
    ("workspace allow", "doc"),
    ("workspace deny", "doc"),
    ("workspace install-hook", "doc"),
    ("workspace list", "doc"),
    ("workspace mode", "doc"),
    ("workspace register", "doc"),
    ("workspace unregister", "doc"),
];

/// Walk the clap tree and collect every terminal path (a subcommand with
/// no children of its own). Hidden commands count — they still parse.
fn collect_leaf_paths(cmd: &clap::Command, prefix: &str, out: &mut Vec<String>) {
    let mut has_children = false;
    for sub in cmd.get_subcommands() {
        // `help` is clap's own; it has no output contract of ours.
        if sub.get_name() == "help" {
            continue;
        }
        has_children = true;
        let path = if prefix.is_empty() {
            sub.get_name().to_string()
        } else {
            format!("{prefix} {}", sub.get_name())
        };
        collect_leaf_paths(sub, &path, out);
    }
    if !prefix.is_empty() {
        // A parent whose subcommand is optional (`baseline`, `telemetry`,
        // `migrate`, …) is itself a terminal path when invoked bare. The
        // bare root (`anvil` ensure) is covered by its own contract tests
        // and the runbook, not this registry.
        if !has_children || !cmd.is_subcommand_required_set() {
            out.push(prefix.to_string());
        }
    }
}

#[test]
fn every_terminal_command_path_is_classified_for_json() {
    use clap::CommandFactory;

    let cmd = crate::Cli::command();
    let mut leaves = Vec::new();
    collect_leaf_paths(&cmd, "", &mut leaves);
    leaves.sort();

    let registry: BTreeMap<&str, &str> = JSON_SURFACES.iter().copied().collect();
    assert_eq!(
        registry.len(),
        JSON_SURFACES.len(),
        "duplicate path in JSON_SURFACES"
    );

    let unclassified: Vec<&String> = leaves
        .iter()
        .filter(|path| !registry.contains_key(path.as_str()))
        .collect();
    let stale: Vec<&&str> = registry
        .keys()
        .filter(|path| !leaves.iter().any(|leaf| leaf == **path))
        .collect();

    assert!(
        unclassified.is_empty() && stale.is_empty(),
        "issue #3947: `--json` binds CLI-wide, so every terminal command path must be \
         classified in JSON_SURFACES (and honour its classification).\n\
         Unclassified (new commands — classify them and honour --json): {unclassified:?}\n\
         Stale registry entries (paths that no longer exist): {stale:?}"
    );
}
