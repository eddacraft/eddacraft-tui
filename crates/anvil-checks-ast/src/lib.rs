//! AST-aware anti-pattern detection for Rust, run at gate-time only.
//!
//! ADR-071: the resident intercept daemon links `anvil-checks` (regex,
//! parser-free) on the save-time hot path and must not link tree-sitter
//! (ADR-064). This crate is the AST tier — a **terminal command-path crate**
//! only `anvil-cli` and test crates may depend on — that consumes the
//! registry's dormant `Detection::Ast` rules and runs them on whole-repo
//! `anvil check` / `anvil gate`. The `daemon_dep_boundary` guard verifies the
//! daemon never reaches it.
//!
//! Each AST rule is a tree-sitter query (the `ast_query` in the compiled
//! registry — the single source of truth) plus a Rust predicate keyed by rule
//! id ([`predicates::kind_for`]). Findings are emitted as the same
//! [`anvil_checks::Warning`] the regex scanner produces — identical `family` /
//! `fingerprint` / `severity` / `definition_ref` / `spectrum_position` metadata
//! — so downstream output (text / JSON / SARIF) treats both tiers uniformly.

use std::path::PathBuf;

use anvil_checks::antipattern::registry_loader::{
    CompiledPattern, Detection, LoadRegistryOptions, load_compiled_registry,
};
use anvil_checks::antipattern::scanner::parse_suppression;
use anvil_checks::antipattern::types::{
    Confidence, Location, Suppression, SuppressionScope, Warning, WarningCategory, WarningSeverity,
    create_warning_fingerprint,
};
use tree_sitter::{Node, Query, QueryCursor, QueryMatch, StreamingIterator};

mod predicates;

pub use predicates::{AstRuleKind, kind_for, known_rule_ids};

/// Diagnostic id emitted when a file's AST rules are skipped because the file
/// did not parse cleanly (ADR-071 §8 — fail-safe, warnings-over-blocks).
pub const AST_PARSE_SKIP_ID: &str = "anvil-ast-parse-skip";

/// Options for an AST scan pass.
#[derive(Debug, Clone, Default)]
pub struct AstScanOptions {
    /// Explicit registry path (tests); `None` uses the standard discovery +
    /// embedded fallback the regex loader uses.
    pub registry_path: Option<PathBuf>,
    /// Include `opt_in` rules (off by default, matching the regex scanner).
    pub include_opt_in: bool,
}

/// Result of an AST scan pass.
#[derive(Debug, Clone, Default)]
pub struct AstScanOutput {
    /// Findings (including suppressed ones — `suppressed` is set, mirroring the
    /// regex scanner so the gate can filter on it).
    pub warnings: Vec<Warning>,
    /// Rule ids that were loaded and run.
    pub patterns_checked: Vec<String>,
    /// Number of `.rs` files actually scanned.
    pub files_scanned: usize,
    /// Scanner-init problems surfaced loudly (ADR-071 §8): a malformed
    /// `ast_query`, or a registry AST rule with no predicate-table entry.
    pub init_errors: Vec<String>,
}

fn rust_language() -> tree_sitter::Language {
    tree_sitter_rust::LANGUAGE.into()
}

struct LoadedRule {
    cp: CompiledPattern,
    query: Query,
    kind: AstRuleKind,
    /// Allowlist globs compiled once at load (not per file × rule).
    allowlist: Vec<glob::Pattern>,
}

struct LoadOutcome {
    rules: Vec<LoadedRule>,
    init_errors: Vec<String>,
}

/// Load the AST-detection rules from the compiled registry, compiling each
/// `ast_query` and pairing it with its predicate. Malformed queries and
/// missing predicates are collected as loud init errors (not silent skips); the
/// rules that do load still run, keeping the scan warnings-over-blocks.
fn load_rules(opts: &AstScanOptions) -> LoadOutcome {
    let loaded = load_compiled_registry(&LoadRegistryOptions {
        registry_path: opts.registry_path.clone(),
    });
    let Some(registry) = loaded.registry else {
        // CIB-050 / ADR-071 §3: a registry that cannot be loaded or parsed
        // must fail loudly, never silently produce nothing. The loader's
        // warnings (missing file, parse error, schema mismatch) are the only
        // record of why — fold them into `init_errors` so `AstScanOutput`
        // surfaces them like any other scanner-init failure instead of
        // reporting a default clean scan with AST rules silently disabled.
        return LoadOutcome {
            rules: Vec::new(),
            init_errors: loaded.warnings,
        };
    };

    let language = rust_language();
    let mut rules = Vec::new();
    // Same surfacing for a registry that loaded with warnings (e.g. a
    // configured registry path that does not exist, falling back to the
    // embedded catalogue): the scan still runs, but the operator must see
    // the misconfiguration.
    let mut init_errors = loaded.warnings;

    for cp in &registry.patterns {
        let Detection::Ast { ast_query } = &cp.detection else {
            continue;
        };
        if !cp.enabled || (cp.opt_in && !opts.include_opt_in) {
            continue;
        }
        let Some((kind, _expected)) = predicates::kind_for(&cp.id) else {
            // ADR-071 §3 registry-completeness: a registry `ast` rule with no
            // scanner predicate must fail loudly, never silently produce
            // nothing. The completeness guard test catches this at build; here
            // it surfaces at runtime rather than dropping the rule in silence.
            init_errors.push(format!(
                "registry AST rule {} has no predicate in anvil-checks-ast (registry-completeness, ADR-071 §3)",
                cp.id
            ));
            continue;
        };
        match Query::new(&language, ast_query) {
            Ok(query) => {
                if query.capture_index_for_name("target").is_none() {
                    init_errors.push(format!("AST rule {} query has no `@target` capture", cp.id));
                    continue;
                }
                let allowlist = cp
                    .allowlist
                    .iter()
                    .filter_map(|p| glob::Pattern::new(p).ok())
                    .collect();
                rules.push(LoadedRule {
                    cp: cp.clone(),
                    query,
                    kind,
                    allowlist,
                });
            }
            Err(err) => init_errors.push(format!(
                "AST rule {} has a malformed ast_query: {err}",
                cp.id
            )),
        }
    }

    LoadOutcome { rules, init_errors }
}

/// Scan already-read file bytes (gate-time core). `files` pairs each path with
/// its UTF-8 source bytes; non-`.rs` files and files a rule's allowlist covers
/// are skipped.
#[must_use]
pub fn scan_bytes(
    files: &[(&str, &[u8])],
    workspace_root: Option<&str>,
    opts: &AstScanOptions,
) -> AstScanOutput {
    let LoadOutcome { rules, init_errors } = load_rules(opts);
    for err in &init_errors {
        tracing::error!(target: "anvil_checks_ast", "{err}");
    }
    if rules.is_empty() {
        return AstScanOutput {
            init_errors,
            ..AstScanOutput::default()
        };
    }

    let patterns_checked: Vec<String> = rules.iter().map(|r| r.cp.id.clone()).collect();
    let mut warnings = Vec::new();
    let mut files_scanned = 0_usize;

    // One parser for the whole pass — `Parser::parse` is reusable across files
    // (council perf finding); a fresh allocation per file is wasted work.
    let mut parser = tree_sitter::Parser::new();
    if parser.set_language(&rust_language()).is_err() {
        return AstScanOutput {
            init_errors,
            ..AstScanOutput::default()
        };
    }

    for (path, bytes) in files {
        if !path_has_rust_extension(path) {
            continue;
        }
        let Ok(content) = std::str::from_utf8(bytes) else {
            continue;
        };
        let relative = normalise_path(path, workspace_root);
        files_scanned += 1;
        scan_one(&mut parser, &relative, content, &rules, &mut warnings);
    }

    sort_warnings(&mut warnings);
    AstScanOutput {
        warnings,
        patterns_checked,
        files_scanned,
        init_errors,
    }
}

/// Scan files by reading each path from disk (the CLI surfaces that legitimately
/// read from cwd). Unreadable / non-UTF-8 files are skipped.
#[must_use]
pub fn scan_paths(
    files: &[&str],
    workspace_root: Option<&str>,
    opts: &AstScanOptions,
) -> AstScanOutput {
    let owned: Vec<(String, Vec<u8>)> = files
        .iter()
        .filter(|p| path_has_rust_extension(p))
        .filter_map(|p| std::fs::read(p).ok().map(|b| ((*p).to_string(), b)))
        .collect();
    let refs: Vec<(&str, &[u8])> = owned
        .iter()
        .map(|(p, b)| (p.as_str(), b.as_slice()))
        .collect();
    scan_bytes(&refs, workspace_root, opts)
}

fn scan_one(
    parser: &mut tree_sitter::Parser,
    path: &str,
    content: &str,
    rules: &[LoadedRule],
    out: &mut Vec<Warning>,
) {
    let Some(tree) = parser.parse(content, None) else {
        out.push(parse_skip_warning(
            path,
            "tree-sitter returned no parse tree",
        ));
        return;
    };
    let root = tree.root_node();
    if root.has_error() {
        // ADR-071 §8: a partial/error tree skips this file's AST rules and emits
        // a single skipped-file diagnostic rather than risking false findings
        // from a malformed parse. Never aborts the run.
        out.push(parse_skip_warning(path, "file did not parse cleanly"));
        return;
    }

    let src = content.as_bytes();
    let lines: Vec<&str> = content.lines().collect();

    for rule in rules {
        if rule_is_allowlisted(path, rule) {
            continue;
        }
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&rule.query, root, src);
        while let Some(m) = matches.next() {
            let Some(target) = capture_node(&rule.query, m, "target") else {
                continue;
            };
            let ctx = PredCtx {
                target,
                query: &rule.query,
                m,
                src,
                path,
            };
            if !eval(rule.kind, &ctx) {
                continue;
            }
            // Anchor the finding on the most specific captured token — the
            // `unwrap`/`expect` method or the macro name — so a multi-line
            // method chain reports on the `.unwrap()` line (and the
            // `@anvil-ignore` directive sits directly above it), not on the
            // start of the receiver expression. Falls back to `@target`.
            let anchor = capture_node(&rule.query, m, "method")
                .or_else(|| capture_node(&rule.query, m, "name"))
                .unwrap_or(target);
            let pos = anchor.start_position();
            // `line` is 1-based (tree-sitter `row` is 0-based). `column` stays a
            // 0-based byte offset to match the regex scanner's `Warning.location`
            // convention (anvil-checks scanner.rs) — tree-sitter `column` is
            // already a 0-based byte offset within the row (Copilot review).
            let line = pos.row + 1;
            let column = pos.column;
            let suppressed = suppression_for(&lines, line, &rule.cp.id);
            out.push(warning_from_match(&rule.cp, path, line, column, suppressed));
        }
    }
}

// =============================================================================
// Predicate dispatch
// =============================================================================

struct PredCtx<'a, 'tree> {
    target: Node<'tree>,
    query: &'a Query,
    m: &'a QueryMatch<'a, 'tree>,
    src: &'a [u8],
    path: &'a str,
}

impl PredCtx<'_, '_> {
    fn capture_text(&self, name: &str) -> Option<&str> {
        capture_node(self.query, self.m, name).map(|n| predicates::node_text(n, self.src))
    }
}

fn eval(kind: AstRuleKind, ctx: &PredCtx) -> bool {
    match kind {
        AstRuleKind::UnwrapExpect => {
            let Some(method) = ctx.capture_text("method") else {
                return false;
            };
            (method == "unwrap" || method == "expect")
                && !predicates::path_is_test_target(ctx.path)
                && !predicates::in_cfg_test(ctx.target, ctx.src)
        }
        AstRuleKind::Panic => {
            let Some(name) = ctx.capture_text("name") else {
                return false;
            };
            name == "panic"
                && !predicates::path_is_test_target(ctx.path)
                && !predicates::in_cfg_test(ctx.target, ctx.src)
        }
        AstRuleKind::UnsafeNoSafety => {
            // `unsafe` outside shipped runtime code is not the target: exclude
            // `#[cfg(test)]` modules and the paths `path_is_test_target` covers
            // (`tests/`/`benches/`/`examples/` targets, `tests.rs`/`test.rs`/
            // `bench.rs` module files, and `build.rs` scripts) — the same
            // exclusion RS-001/RS-002 apply (external-FP dogfood: tokio tests).
            !predicates::has_preceding_safety_comment(ctx.target, ctx.src)
                && !predicates::path_is_test_target(ctx.path)
                && !predicates::in_cfg_test(ctx.target, ctx.src)
        }
        AstRuleKind::SerdeDenyUnknown => predicates::struct_lacks_deny_unknown(ctx.target, ctx.src),
        AstRuleKind::TodoMacro => {
            // Shares RS-002's `macro_invocation` query; dispatch on the macro
            // name and exclude test scaffolding the same way. Moving RS-005 off
            // the regex engine also drops its doc-comment false positives — the
            // parser never sees `/// unimplemented!()` as a macro call
            // (external-FP dogfood).
            let Some(name) = ctx.capture_text("name") else {
                return false;
            };
            (name == "todo" || name == "unimplemented")
                && !predicates::path_is_test_target(ctx.path)
                && !predicates::in_cfg_test(ctx.target, ctx.src)
        }
    }
}

fn capture_node<'tree>(
    query: &Query,
    m: &QueryMatch<'_, 'tree>,
    name: &str,
) -> Option<Node<'tree>> {
    let idx = query.capture_index_for_name(name)?;
    m.captures.iter().find(|c| c.index == idx).map(|c| c.node)
}

// =============================================================================
// Warning construction (mirrors the regex scanner's CompiledPattern → Warning)
// =============================================================================

fn warning_from_match(
    cp: &CompiledPattern,
    path: &str,
    line: usize,
    column: usize,
    suppressed: Option<Suppression>,
) -> Warning {
    let mut warning = Warning {
        id: cp.id.clone(),
        fingerprint: None,
        category: WarningCategory::AntiPattern,
        severity: cp.severity,
        confidence: cp.confidence,
        title: cp.title.clone(),
        message: format!("Found {} at line {line}", cp.title),
        explanation: cp.explanation.clone(),
        suggestion: cp.suggestion.clone(),
        nudge: Some(cp.nudge.clone()),
        location: Location {
            file: path.to_string(),
            line,
            column: Some(column),
            end_line: None,
            end_column: None,
        },
        pattern: Some(cp.id.clone()),
        suppressed,
        family: Some(cp.family.clone()),
        definition_ref: Some(cp.definition_ref.clone()),
        spectrum_position: Some(cp.spectrum_position),
    };
    warning.fingerprint = Some(create_warning_fingerprint(&warning));
    warning
}

fn parse_skip_warning(path: &str, reason: &str) -> Warning {
    let mut warning = Warning {
        id: AST_PARSE_SKIP_ID.to_string(),
        fingerprint: None,
        category: WarningCategory::AntiPattern,
        severity: WarningSeverity::Info,
        confidence: Confidence::Low,
        title: "AST rules skipped (parse error)".to_string(),
        message: format!("Skipped Rust AST anti-pattern rules for {path}: {reason}"),
        explanation: "The AST tier could not build a clean parse tree for this \
                      file, so its AST rules were skipped to avoid false findings."
            .to_string(),
        suggestion: "Check the file parses with the pinned tree-sitter-rust grammar.".to_string(),
        nudge: None,
        location: Location {
            file: path.to_string(),
            line: 1,
            // 0-based byte-offset column, matching the regex scanner convention.
            column: Some(0),
            end_line: None,
            end_column: None,
        },
        pattern: Some(AST_PARSE_SKIP_ID.to_string()),
        suppressed: None,
        family: None,
        definition_ref: None,
        spectrum_position: None,
    };
    warning.fingerprint = Some(create_warning_fingerprint(&warning));
    warning
}

// =============================================================================
// Suppression, allowlist, path, ordering
// =============================================================================

/// Resolve an `// @anvil-ignore <ID> -- <reason>` directive on the line directly
/// above the finding's anchor line (the match node's start line — ADR-071 §5),
/// reusing the authoritative ADR-029 parser.
fn suppression_for(lines: &[&str], line_number: usize, pattern_id: &str) -> Option<Suppression> {
    if line_number <= 1 {
        return None;
    }
    let previous = lines.get(line_number - 2)?;
    let (id, reason) = parse_suppression(previous)?;
    if id != pattern_id {
        return None;
    }
    Some(Suppression {
        reason,
        author: None,
        timestamp: None,
        scope: SuppressionScope::Line,
    })
}

fn rule_is_allowlisted(path: &str, rule: &LoadedRule) -> bool {
    // Match against the full path and the basename, so both path globs
    // (`**/generated/**`) and bare basename globs (`build.rs`, `*.gen.rs`) work
    // — a plain `glob::Pattern` won't cross `/`, so a bare basename pattern
    // would otherwise never match a nested path (Copilot review).
    let basename = path.rsplit(['/', '\\']).next().unwrap_or(path);
    rule.allowlist
        .iter()
        .any(|g| g.matches(path) || g.matches(basename))
}

fn path_has_rust_extension(path: &str) -> bool {
    std::path::Path::new(path)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("rs"))
}

fn normalise_path(path: &str, workspace_root: Option<&str>) -> String {
    match workspace_root {
        Some(root) => path.strip_prefix(root).map_or_else(
            || path.to_string(),
            |rel| rel.trim_start_matches(['/', '\\']).to_string(),
        ),
        None => path.to_string(),
    }
}

/// Same `(line, column, id)` ordering the regex scanner applies, so merged
/// output stays deterministic (ADR-071 §6).
fn sort_warnings(warnings: &mut [Warning]) {
    warnings.sort_by(|a, b| {
        a.location
            .file
            .cmp(&b.location.file)
            .then_with(|| a.location.line.cmp(&b.location.line))
            .then_with(|| a.location.column.cmp(&b.location.column))
            .then_with(|| a.id.cmp(&b.id))
    });
}

#[cfg(test)]
mod tests;
