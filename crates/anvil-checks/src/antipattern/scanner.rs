use std::sync::LazyLock;

use rayon::prelude::*;
use regex::Regex;

use crate::antipattern::patterns::all_patterns;
use crate::antipattern::types::{
    AntiPattern, ArtifactKind, Location, Suppression, SuppressionScope, Warning, WarningCategory,
    create_warning_fingerprint,
};

const LEGACY_JS_TS_EXTENSIONS: [&str; 6] = [".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs"];

static LEGACY_JS_TS_EXTENSIONS_OWNED: LazyLock<Vec<String>> = LazyLock::new(|| {
    LEGACY_JS_TS_EXTENSIONS
        .iter()
        .map(|s| (*s).to_string())
        .collect()
});

#[derive(Debug, Clone, Default)]
pub struct ScanOptions {
    pub patterns: Option<Vec<String>>,
    pub include_opt_in: bool,
}

/// Unit of content passed to the scanner. `reference` identifies the source
/// of `content` — a file path for `source`, a PR number or URL for
/// `pr-description`, a commit SHA for `commit-message`, a session id for
/// `agent-output`. It surfaces verbatim on resulting warnings via
/// `location.file` so operators can trace the warning back to its origin.
#[derive(Debug, Clone)]
pub struct Artifact {
    pub kind: ArtifactKind,
    pub reference: String,
    pub content: String,
}

impl Artifact {
    #[must_use]
    pub fn source(path: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            kind: ArtifactKind::Source,
            reference: path.into(),
            content: content.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScanResult {
    pub file: String,
    pub artifact_type: ArtifactKind,
    pub warnings: Vec<Warning>,
    pub patterns_checked: Vec<String>,
}

#[derive(Debug)]
struct PreparedPattern {
    pattern: AntiPattern,
    primary_regex: Option<Regex>,
    secondary_regex: Option<Regex>,
    /// SPG-002: populated when the rule's regex could not be compiled by the
    /// `regex` crate (typically a PCRE lookaround that RE2 rejects). Surfaced
    /// via `registry_compile_diagnostics()` so operators can distinguish
    /// "rule ran, no matches" from "rule never ran".
    compile_error: Option<String>,
    /// SPG-003: hand-coded translation of the rule's PCRE lookaround, applied
    /// after `primary_regex` matches. Only set for the six rules whose
    /// registry pattern the `regex` crate cannot express directly.
    post_filter: Option<PostFilter>,
    /// V050F-006: compiled allowlist globs (compiled `Regex` plus the
    /// precomputed `is_path_glob` flag). Pre-compilation moves the
    /// per-pattern regex build cost out of the per-file hot path.
    /// Original glob source strings remain in `pattern.allowlist`;
    /// this field holds only the runtime artefacts the matcher needs.
    /// Patterns that fail to compile carry `regex: None` (they never
    /// matched at the old call site either, since `glob_to_regex`
    /// returned `None`); the invalid-pattern path is silent because
    /// the registry / `.anvil` authoring tools already validate
    /// globs.
    allowlist_regexes: Vec<AllowlistGlob>,
}

/// Compiled allowlist glob plus the precomputed match-base flag.
/// Held by [`PreparedPattern::allowlist_regexes`].
///
/// V050F-006: the matcher keeps the historical match-base
/// semantics — when the original glob had no `/`, the regex matches
/// against the file's basename, not the full path. The decision is
/// boolean, so we precompute and store it as `is_path_glob` instead
/// of retaining the original glob string only to call `contains('/')`
/// per match (council finding: kernel-maintainer).
#[derive(Debug)]
struct AllowlistGlob {
    /// `true` when the original glob contained a `/`; controls
    /// whether the matcher walks the full normalised path or the
    /// basename only.
    is_path_glob: bool,
    /// Compiled regex equivalent of the original glob. `None` means
    /// the glob failed to compile in `prepare_pattern`; the matcher
    /// treats it as a no-op (matches nothing), which mirrors the
    /// pre-V050F behaviour where `glob_to_regex` returned `None`
    /// and `is_some_and` short-circuited to `false`.
    regex: Option<Regex>,
}

/// Replaces a PCRE lookaround with a Rust-side predicate applied after the
/// primary regex matches. The registry entry's pattern is preserved as the
/// canonical spec (and read by the TS scanner directly); these filters pin
/// the Rust scanner to the same observable behaviour.
#[derive(Debug)]
enum PostFilter {
    /// PCRE `(?!.*ESCAPE)`: suppress the match when `escape` matches any
    /// substring of the line starting from the match's end column.
    NegativeFromMatchEnd { escape: Regex },
    /// PCRE `(?=CHARCLASS|$)`: suppress unless the byte immediately after
    /// the match is in `allowed`, whitespace, or the match ends the line.
    RequireTrailingByteOrEol { allowed: &'static [u8] },
}

/// Structured report produced when a registry rule fails to compile under the
/// Rust `regex` crate. Emitted by `registry_compile_diagnostics()` and
/// surfaced by `anvil doctor` so the silent-drop path is observable.
#[derive(Debug, Clone)]
pub struct CompileDiagnostic {
    pub pattern_id: String,
    pub pattern_title: String,
    pub error: String,
}

/// Prepare every registry pattern exactly once per process. Regex compilation
/// is the dominant cost per scan; moving it behind a `LazyLock` means
/// subsequent scans pay only the match cost. `Regex` is `Send + Sync`, so the
/// cache can be shared across rayon worker threads without wrapping.
static PREPARED_PATTERNS: LazyLock<Vec<PreparedPattern>> =
    LazyLock::new(|| all_patterns().into_iter().map(prepare_pattern).collect());

fn prepared_patterns_for(options: &ScanOptions) -> Vec<&'static PreparedPattern> {
    if let Some(pattern_ids) = &options.patterns
        && !pattern_ids.is_empty()
    {
        return PREPARED_PATTERNS
            .iter()
            .filter(|prepared| pattern_ids.iter().any(|id| id == &prepared.pattern.id))
            .collect();
    }

    PREPARED_PATTERNS
        .iter()
        .filter(|prepared| {
            prepared.pattern.enabled && (options.include_opt_in || !prepared.pattern.opt_in)
        })
        .collect()
}

fn matches_file_extension(file_path: &str, file_extensions: &[String]) -> bool {
    let Some(dot_index) = file_path.rfind('.') else {
        return false;
    };

    let extension = file_path[dot_index..].to_ascii_lowercase();
    file_extensions
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(&extension))
}

fn normalise_path(file_path: &str) -> String {
    file_path.replace('\\', "/")
}

fn basename(file_path: &str) -> &str {
    file_path.rsplit('/').next().unwrap_or(file_path)
}

fn glob_to_regex(pattern: &str) -> Option<Regex> {
    let mut regex = String::from("^");
    let mut chars = pattern.chars().peekable();

    if pattern.starts_with("**/") {
        regex.push_str("(?:.*/)?");
        let _ = chars.next();
        let _ = chars.next();
        let _ = chars.next();
    }

    while let Some(ch) = chars.next() {
        match ch {
            '*' => {
                if chars.peek() == Some(&'*') {
                    let _ = chars.next();
                    regex.push_str(".*");
                } else {
                    regex.push_str("[^/]*");
                }
            }
            '?' => regex.push_str("[^/]"),
            '.' | '+' | '(' | ')' | '[' | ']' | '{' | '}' | '^' | '$' | '|' | '\\' => {
                regex.push('\\');
                regex.push(ch);
            }
            _ => regex.push(ch),
        }
    }

    regex.push('$');
    Regex::new(&regex).ok()
}

/// Match a path against a pre-compiled allowlist (V050F-006).
/// Mirrors the historical `glob_match` match-base semantics — when
/// the original glob has no `/`, the regex matches against the
/// basename, not the full path — without re-parsing or re-compiling
/// on every call.
///
/// `regex == None` (compile failed in `prepare_pattern`) returns
/// `false` for that entry, identical to the previous
/// `glob_to_regex(...).is_some_and(...)` short-circuit.
fn is_file_allowlisted_compiled(file_path: &str, allowlist: &[AllowlistGlob]) -> bool {
    let normalised = normalise_path(file_path);
    let basename = basename(&normalised);
    allowlist.iter().any(|entry| {
        let Some(regex) = entry.regex.as_ref() else {
            return false;
        };
        let target = if entry.is_path_glob {
            normalised.as_str()
        } else {
            basename
        };
        regex.is_match(target)
    })
}

fn create_warning_from_match(
    pattern: &AntiPattern,
    file_path: &str,
    line: usize,
    column: usize,
    suppressed: Option<Suppression>,
) -> Warning {
    let mut warning = Warning {
        id: pattern.id.clone(),
        fingerprint: None,
        category: WarningCategory::AntiPattern,
        severity: pattern.severity,
        confidence: pattern.confidence,
        title: pattern.title.clone(),
        message: format!("Found {} at line {line}", pattern.name),
        explanation: pattern.explanation.clone(),
        suggestion: pattern.suggestion.clone(),
        nudge: pattern.nudge.clone(),
        location: Location {
            file: file_path.to_string(),
            line,
            column: Some(column),
            end_line: None,
            end_column: None,
        },
        pattern: Some(pattern.id.clone()),
        suppressed,
        family: pattern.family.clone(),
        definition_ref: pattern.definition_ref.clone(),
        spectrum_position: pattern.spectrum_position,
    };
    warning.fingerprint = Some(create_warning_fingerprint(&warning));
    warning
}

static SUPPRESSION_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    // Per ADR-029 the Rust parser is the authoritative suppression parser
    // for all new surfaces (SURFENV, SURFSQL, SURFCI, …). The ID capture is
    // therefore broad enough to admit any `<PREFIX>-<TAIL>` shape — `AP-003`,
    // `SURFENV-001`, `SURFSQL-002` — rather than the legacy `AP-\d{3}` form.
    // Downstream callers compare the captured ID to the rule they're
    // checking, so widening here cannot suppress an unrelated rule.
    Regex::new(
        r"(?://|/\*|#|<!--|--)\s*@anvil-ignore\s+([A-Z][A-Z0-9]*-[A-Z0-9]+)(?:\s*--\s*(.+))?",
    )
    .expect("static suppression regex must compile")
});

/// Parse an `@anvil-ignore <ID> -- <reason>` directive from a line.
///
/// Authoritative entry point per
/// [ADR-029](../../../plans/decisions/029-suppression-parser-authority.md):
/// every new Track 3 surface module reuses this parser rather than rolling
/// its own. Callers are expected to gate the result on `id == pattern_id`
/// — this function only extracts the directive.
#[must_use]
pub fn parse_suppression(line: &str) -> Option<(String, String)> {
    let captures = SUPPRESSION_REGEX.captures(line)?;
    let id = captures.get(1).map_or("", |capture| capture.as_str());
    let reason = captures
        .get(2)
        .map_or("No reason provided", |capture| capture.as_str())
        .trim();
    Some((id.to_string(), reason.to_string()))
}

/// Map an `ESLint` rule name (or `None` for a bare `eslint-disable-next-line`)
/// to the Anvil rule IDs it suppresses. The mapping covers the `AP`/`GS` family
/// rules that overlap with standard `@typescript-eslint/*` lints. A bare
/// directive (no rule name) suppresses the whole family because the user
/// has explicitly opted out of *some* lint and Anvil should not double-flag.
fn eslint_rule_suppresses_anvil(eslint_rule: Option<&str>, anvil_id: &str) -> bool {
    match eslint_rule {
        // Bare `eslint-disable-next-line` with no rule — broad opt-out.
        None => matches!(
            anvil_id,
            "AP-001" | "AP-002" | "AP-003" | "AP-004" | "AP-005" | "AP-006" | "AP-007" | "GS-001"
        ),
        Some(rule) => match rule {
            "@typescript-eslint/no-explicit-any" => anvil_id == "AP-003",
            "@typescript-eslint/ban-ts-comment" => anvil_id == "AP-004" || anvil_id == "AP-005",
            "@typescript-eslint/no-non-null-assertion" => anvil_id == "GS-001",
            "no-empty" => anvil_id == "AP-006",
            "no-console" => anvil_id == "AP-007",
            _ => false,
        },
    }
}

static ESLINT_DISABLE_NEXT_LINE: LazyLock<Regex> = LazyLock::new(|| {
    // Captures: (1) optional rule list (comma-separated), (2) optional reason.
    // The trailing `--` reason is convention; when absent the comment may end
    // at end-of-line or have a `*/` block close.
    Regex::new(
        r"(?://|/\*)\s*eslint-disable-next-line(?:\s+([^\s/].*?))?\s*(?:--\s*(.+?))?\s*(?:\*/|$)",
    )
    .expect("eslint-disable-next-line regex must compile")
});

static ESLINT_DISABLE_BLOCK_OPEN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"/\*\s*eslint-disable(?:\s+([^*]+?))?\s*\*/")
        .expect("eslint-disable block-open regex must compile")
});

static ESLINT_DISABLE_BLOCK_CLOSE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"/\*\s*eslint-enable(?:\s+([^*]+?))?\s*\*/")
        .expect("eslint-disable block-close regex must compile")
});

/// Parsed contents of an `ESLint` suppression directive.
struct EslintDirective {
    rules: Vec<String>,
    reason: Option<String>,
}

fn parse_eslint_directive(captures: &regex::Captures<'_>) -> EslintDirective {
    let rules = captures
        .get(1)
        .map(|m| {
            m.as_str()
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let reason = captures
        .get(2)
        .map(|m| m.as_str().trim().to_string())
        .filter(|s| !s.is_empty());
    EslintDirective { rules, reason }
}

fn directive_suppresses(directive: &EslintDirective, anvil_id: &str) -> Option<String> {
    let matches_rule = if directive.rules.is_empty() {
        eslint_rule_suppresses_anvil(None, anvil_id)
    } else {
        directive
            .rules
            .iter()
            .any(|rule| eslint_rule_suppresses_anvil(Some(rule), anvil_id))
    };
    if !matches_rule {
        return None;
    }
    Some(
        directive
            .reason
            .clone()
            .unwrap_or_else(|| "eslint-disable directive".to_string()),
    )
}

/// Look at the line immediately preceding `line_number` (1-based) for a
/// `// eslint-disable-next-line` directive that suppresses `anvil_id`.
fn eslint_next_line_suppression(
    lines: &[&str],
    line_number: usize,
    anvil_id: &str,
) -> Option<String> {
    if line_number <= 1 {
        return None;
    }
    let prior = lines[line_number - 2];
    let captures = ESLINT_DISABLE_NEXT_LINE.captures(prior)?;
    directive_suppresses(&parse_eslint_directive(&captures), anvil_id)
}

/// Walk lines preceding `line_number` looking for an unmatched
/// `/* eslint-disable [rule] */` (no later `/* eslint-enable */`
/// closes it before reaching the current line). Returns the reason
/// when such a block exists and covers `anvil_id`.
fn eslint_block_suppression(lines: &[&str], line_number: usize, anvil_id: &str) -> Option<String> {
    if line_number <= 1 {
        return None;
    }
    // Walk backwards. The first relevant marker we find decides:
    // an `eslint-enable` means the most recent block already closed,
    // so no suppression applies. An `eslint-disable` with a matching
    // rule (or no rule) means we are inside a covering block.
    for prior in lines[..line_number - 1].iter().rev() {
        if let Some(caps) = ESLINT_DISABLE_BLOCK_CLOSE.captures(prior) {
            // If this enables the specific rule we're checking (or
            // is a bare enable), the block is closed — stop scanning.
            let directive = parse_eslint_directive(&caps);
            if directive.rules.is_empty()
                || directive
                    .rules
                    .iter()
                    .any(|rule| eslint_rule_suppresses_anvil(Some(rule), anvil_id))
            {
                return None;
            }
        }
        if let Some(caps) = ESLINT_DISABLE_BLOCK_OPEN.captures(prior)
            && let Some(reason) = directive_suppresses(&parse_eslint_directive(&caps), anvil_id)
        {
            return Some(reason);
        }
    }
    None
}

fn suppression_for_line(
    lines: &[&str],
    line_number: usize,
    pattern_id: &str,
) -> Option<Suppression> {
    // Anvil's native directive takes precedence — same line above the
    // finding, with explicit pattern_id match.
    if line_number > 1 {
        let previous_line = lines[line_number - 2];
        if let Some((id, reason)) = parse_suppression(previous_line)
            && id == pattern_id
        {
            return Some(Suppression {
                reason,
                author: None,
                timestamp: None,
                scope: SuppressionScope::Line,
            });
        }
    }

    // ESLint-disable-next-line: same call site, mapped to Anvil family.
    if let Some(reason) = eslint_next_line_suppression(lines, line_number, pattern_id) {
        return Some(Suppression {
            reason,
            author: None,
            timestamp: None,
            scope: SuppressionScope::Line,
        });
    }

    // ESLint block disable: any earlier `/* eslint-disable [rule] */`
    // not yet closed by `/* eslint-enable */`.
    if let Some(reason) = eslint_block_suppression(lines, line_number, pattern_id) {
        return Some(Suppression {
            reason,
            author: None,
            timestamp: None,
            scope: SuppressionScope::Line,
        });
    }

    None
}

fn prepare_pattern(pattern: AntiPattern) -> PreparedPattern {
    // V050F-006: precompile the allowlist regexes so the hot-path
    // `is_file_allowlisted` does not pay one regex compile per
    // (allowlist entry × scanned file). The `prepare_pcre_rewrite`
    // / AP-001 branches build their PreparedPattern manually, so they
    // call this helper too.
    let allowlist_regexes = compile_allowlist(&pattern.allowlist);

    // AP-001's registry regex uses a PCRE negative-lookahead
    // (`(?!-next-line|-line)`) that Rust's RE2-based `regex` crate cannot
    // compile. Split it into two lookahead-free regexes and OR the matches at
    // call time.
    if pattern.id == "AP-001" {
        return PreparedPattern {
            pattern,
            primary_regex: Regex::new(r"/\*\s*eslint-disable\s*\*/").ok(),
            secondary_regex: Regex::new(r"//\s*eslint-disable\s*$").ok(),
            compile_error: None,
            post_filter: None,
            allowlist_regexes,
        };
    }

    // SPG-003: five rules carry a PCRE negative lookahead and one carries a
    // positive lookahead. RE2 can't compile either. Instead of a blanket
    // silent-drop, translate each into a Rust-side post-filter applied after
    // the base regex matches. The registry entry's pattern is preserved as
    // the canonical TS-scanner spec; the hand-coded halves below must stay
    // semantically aligned with it. Any drift surfaces as a scanner-parity
    // fixture failure in `tests/scanner-parity/fixtures.json`.
    if let Some(prepared) = prepare_pcre_rewrite(&pattern) {
        return prepared;
    }

    match Regex::new(&pattern.regex) {
        Ok(regex) => PreparedPattern {
            primary_regex: Some(regex),
            secondary_regex: None,
            compile_error: None,
            post_filter: None,
            pattern,
            allowlist_regexes,
        },
        Err(err) => PreparedPattern {
            primary_regex: None,
            secondary_regex: None,
            compile_error: Some(err.to_string()),
            post_filter: None,
            pattern,
            allowlist_regexes,
        },
    }
}

/// V050F-006: pre-compile every allowlist glob into its regex
/// equivalent AND precompute the match-base flag so the per-file
/// hot path does no work beyond regex matching. Glob → regex mapping
/// mirrors [`glob_to_regex`] exactly; failures yield `regex: None`
/// so the matcher's behaviour is the same as the previous
/// `glob_to_regex(...).is_some_and(...)` shape.
fn compile_allowlist(allowlist: &[String]) -> Vec<AllowlistGlob> {
    allowlist
        .iter()
        .map(|pattern| AllowlistGlob {
            is_path_glob: pattern.contains('/'),
            regex: glob_to_regex(pattern),
        })
        .collect()
}

/// Specification for a hand-coded PCRE rewrite: the regex-crate-compatible
/// base regex plus the rule-specific post-filter that mirrors the lookaround
/// the base regex cannot express. Kept as a pure data table so `rewrite_spec`
/// can be inspected by tests (e.g. `spg003_rewrite_matches_registry_snapshot`)
/// without having to compile any regexes.
///
/// The `expected_*` fields are drift-guard snapshots read only by the
/// `spg003_rewrite_matches_registry_snapshot` test; they are intentionally
/// unused in non-test builds.
#[allow(dead_code)]
struct RewriteSpec {
    base_regex: &'static str,
    filter: FilterSpec,
    /// Snapshot of the registry's `detection.regex` string at the time the
    /// rewrite was hand-coded. Adversarial-reviewer M-1: if the `.anvil`
    /// source is edited and the compiled registry drifts from this
    /// snapshot, the Rust scanner would silently keep using the stale
    /// rewrite. The snapshot test compares this to the live registry and
    /// fires on any drift, forcing `rewrite_spec` to be revisited.
    expected_registry_regex: &'static str,
    /// Registry `flags` value that was in force when the rewrite was
    /// hand-coded. Same drift-guard purpose as `expected_registry_regex`.
    expected_registry_flags: Option<&'static str>,
}

enum FilterSpec {
    Negative { escape_regex: &'static str },
    TrailingByteOrEol { allowed: &'static [u8] },
}

/// Lookup table for the six hand-coded rewrites. `None` for any other rule.
fn rewrite_spec(id: &str) -> Option<RewriteSpec> {
    Some(match id {
        "DD-001" => RewriteSpec {
            base_regex: r"//\s*(TODO|FIXME)\b",
            filter: FilterSpec::Negative {
                escape_regex: r"([A-Z]+-\d+|#\d+|issue|ticket)",
            },
            expected_registry_regex: r"//\s*(TODO|FIXME)\b(?!.*([A-Z]+-\d+|#\d+|issue|ticket))",
            expected_registry_flags: None,
        },
        "DD-002" => RewriteSpec {
            base_regex: r"//\s*(HACK|XXX)\b",
            filter: FilterSpec::Negative {
                escape_regex: r"([A-Z]+-\d+|#\d+|issue|ticket)",
            },
            expected_registry_regex: r"//\s*(HACK|XXX)\b(?!.*([A-Z]+-\d+|#\d+|issue|ticket))",
            expected_registry_flags: None,
        },
        "DD-003" => RewriteSpec {
            base_regex: r"//\s*(temporary|workaround|compat|shim|stopgap|interim)\b",
            filter: FilterSpec::Negative {
                escape_regex: r"(until|before|after|when|remove|drop|deadline|\d{4}-\d{2})",
            },
            expected_registry_regex: r"//\s*(temporary|workaround|compat|shim|stopgap|interim)\b(?!.*(until|before|after|when|remove|drop|deadline|\d{4}-\d{2}))",
            expected_registry_flags: None,
        },
        "GS-001" => RewriteSpec {
            base_regex: r"[\w.)\]]+!",
            filter: FilterSpec::TrailingByteOrEol { allowed: b".[(;,)" },
            expected_registry_regex: r"[\w.)\]]+!(?=[.\[(\s;,)]|$)",
            expected_registry_flags: None,
        },
        "RL-001" => RewriteSpec {
            base_regex: r"(?i)\bpre-existing\b",
            filter: FilterSpec::Negative {
                escape_regex: r"(?i)\b(run #|run id|also fails on|verified)",
            },
            expected_registry_regex: r"\bpre-existing\b(?!.*\b(run #|run id|also fails on|verified))",
            expected_registry_flags: Some("i"),
        },
        "RL-005" => RewriteSpec {
            base_regex: r"(?i)\b(defer(red)?|follow[\s-]?up|backlog(ged)?)\b",
            filter: FilterSpec::Negative {
                escape_regex: r"(?i)(issue\s*#|gh\s+issue|TODO|created\s+(issue|ticket))",
            },
            expected_registry_regex: r"\b(defer(red)?|follow[\s-]?up|backlog(ged)?)\b(?!.*(issue\s*#|gh\s+issue|TODO|created\s+(issue|ticket)))",
            expected_registry_flags: Some("i"),
        },
        _ => return None,
    })
}

/// Hand-coded Rust equivalents of the six PCRE-lookaround rules. Returns
/// `None` for any other rule so the caller falls back to the standard
/// compile path. Any compile failure on the hand-coded base or escape regex
/// routes through `compile_error` so `registry_compile_diagnostics()` can
/// surface the regression — there is no silent-drop path.
fn prepare_pcre_rewrite(pattern: &AntiPattern) -> Option<PreparedPattern> {
    let spec = rewrite_spec(pattern.id.as_str())?;

    let primary_regex = match Regex::new(spec.base_regex) {
        Ok(regex) => regex,
        Err(err) => return Some(rewrite_compile_error(pattern, "base regex", &err)),
    };

    let post_filter = match spec.filter {
        FilterSpec::Negative { escape_regex } => match Regex::new(escape_regex) {
            Ok(regex) => PostFilter::NegativeFromMatchEnd { escape: regex },
            Err(err) => return Some(rewrite_compile_error(pattern, "escape regex", &err)),
        },
        FilterSpec::TrailingByteOrEol { allowed } => {
            PostFilter::RequireTrailingByteOrEol { allowed }
        }
    };

    Some(PreparedPattern {
        primary_regex: Some(primary_regex),
        secondary_regex: None,
        compile_error: None,
        post_filter: Some(post_filter),
        allowlist_regexes: compile_allowlist(&pattern.allowlist),
        pattern: pattern.clone(),
    })
}

fn rewrite_compile_error(
    pattern: &AntiPattern,
    kind: &'static str,
    err: &regex::Error,
) -> PreparedPattern {
    PreparedPattern {
        primary_regex: None,
        secondary_regex: None,
        compile_error: Some(format!(
            "SPG-003 {kind} for {} failed to compile: {err}",
            pattern.id
        )),
        post_filter: None,
        allowlist_regexes: compile_allowlist(&pattern.allowlist),
        pattern: pattern.clone(),
    }
}

fn post_filter_accepts(filter: &PostFilter, line: &str, match_end: usize) -> bool {
    match filter {
        PostFilter::NegativeFromMatchEnd { escape } => {
            let remainder = line.get(match_end..).unwrap_or("");
            !escape.is_match(remainder)
        }
        PostFilter::RequireTrailingByteOrEol { allowed } => {
            if match_end >= line.len() {
                return true;
            }
            // `allowed` is ASCII punctuation; check the raw byte first to
            // avoid paying the char-decode cost on the hot path. Fall back
            // to Unicode whitespace (PCRE `\s` under V8 matches the full
            // Unicode whitespace class) so content with NBSP / ideographic
            // space / ZWSP after a non-null assertion stays in parity with
            // the TS scanner.
            let next_byte = line.as_bytes()[match_end];
            if allowed.contains(&next_byte) {
                return true;
            }
            line[match_end..]
                .chars()
                .next()
                .is_some_and(char::is_whitespace)
        }
    }
}

/// Return a diagnostic for every registry rule whose regex failed to compile
/// under the Rust engine. Empty when every enabled rule compiled cleanly —
/// which should be the steady state after the SPG-003 rewrites land.
///
/// `anvil doctor` uses this to flag silent-drop rules; the scanner's hot
/// path does not call it, so there is no per-scan overhead.
#[must_use]
pub fn registry_compile_diagnostics() -> Vec<CompileDiagnostic> {
    PREPARED_PATTERNS
        .iter()
        .filter_map(|prepared| {
            prepared
                .compile_error
                .as_ref()
                .map(|err| CompileDiagnostic {
                    pattern_id: prepared.pattern.id.clone(),
                    pattern_title: prepared.pattern.title.clone(),
                    error: err.clone(),
                })
        })
        .collect()
}

/// Number of preceding lines GS-001 inspects when checking for a
/// guarded Map.get / Map.set / Map.has idiom. 8 covers the typical
/// "ensure key exists, then push" pattern across reasonable nesting;
/// if the guard sits further away the code is suspect anyway.
const GS001_GUARD_LOOKBACK: usize = 8;

static GS001_GET_KEY: LazyLock<Regex> = LazyLock::new(|| {
    // Anchored to end-of-haystack so we extract the `.get(<key>)!`
    // immediately preceding the match. The GS-001 base regex's
    // character class excludes `(`, so the matched span often only
    // covers `pattern)!`; we look at the prefix of the line up to
    // match_end and find the `.get(<key>)!` that ends there.
    Regex::new(r"([A-Za-z_][A-Za-z0-9_.]*)\.get\(\s*([A-Za-z_][A-Za-z0-9_.]*)\s*\)!\s*$")
        .expect("GS-001 get-key extractor must compile")
});

static GS001_MAP_GUARD: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"([A-Za-z_][A-Za-z0-9_.]*)\.(?:has|set)\(\s*([A-Za-z_][A-Za-z0-9_.]*)\s*(?:[,\)])")
        .expect("GS-001 map guard extractor must compile")
});

/// Return `true` when the `<receiver>.get(<key>)!` match has a
/// preceding `.has(<key>)` or `.set(<key>, ...)` guard within the
/// look-back window — the canonical "lazily populate this Map then
/// dereference" idiom whose runtime guarantee is explicit but
/// invisible to the line-local regex.
fn gs001_is_guarded_map_get(
    line: &str,
    _match_start: usize,
    match_end: usize,
    lines: &[&str],
    line_index: usize,
) -> bool {
    let prefix = line.get(..match_end).unwrap_or("");
    let Some(captures) = GS001_GET_KEY.captures(prefix) else {
        return false;
    };
    let Some(receiver) = captures.get(1) else {
        return false;
    };
    let Some(key) = captures.get(2) else {
        return false;
    };
    let receiver = receiver.as_str();
    let key = key.as_str();

    let start = line_index.saturating_sub(GS001_GUARD_LOOKBACK);
    for prior in &lines[start..line_index] {
        if GS001_MAP_GUARD.captures_iter(prior).any(|guard| {
            guard
                .get(1)
                .is_some_and(|guard_receiver| guard_receiver.as_str() == receiver)
                && guard
                    .get(2)
                    .is_some_and(|guard_key| guard_key.as_str() == key)
        }) {
            return true;
        }
    }
    false
}

/// GH #1914: rules whose detection only makes sense in executable code — a
/// match inside a comment or string literal is always a false positive, so
/// the scanner runs them against a comment/string-masked view of the source.
///
/// Deliberately a small opt-in allowlist rather than the default: most other
/// rules legitimately target comments (AP-001 `// eslint-disable`, AP-004/-005
/// `@ts-ignore` / `@ts-expect-error`, DD-* `// TODO|HACK`) or prose (RL-*),
/// and masking would silence them. Extending this set — or promoting it to a
/// `lexical_scope` field on the compiled registry so each rule declares its
/// own scope — is tracked as a follow-up on #1914.
fn rule_is_code_scoped(rule_id: &str) -> bool {
    matches!(rule_id, "AP-003" | "GS-001")
}

fn find_match_columns(
    prepared: &PreparedPattern,
    line: &str,
    lines: &[&str],
    line_index: usize,
) -> Vec<usize> {
    if prepared.pattern.id == "AP-001" {
        let mut columns = Vec::new();
        if let Some(regex) = &prepared.primary_regex {
            columns.extend(regex.find_iter(line).map(|matched| matched.start()));
        }
        if let Some(regex) = &prepared.secondary_regex {
            columns.extend(regex.find_iter(line).map(|matched| matched.start()));
        }
        columns.sort_unstable();
        return columns;
    }

    let is_gs001 = prepared.pattern.id == "GS-001";

    prepared
        .primary_regex
        .as_ref()
        .map_or_else(Vec::new, |regex| {
            regex
                .find_iter(line)
                .filter(|matched| match &prepared.post_filter {
                    None => true,
                    Some(filter) => post_filter_accepts(filter, line, matched.end()),
                })
                .filter(|matched| {
                    !is_gs001
                        || !gs001_is_guarded_map_get(
                            line,
                            matched.start(),
                            matched.end(),
                            lines,
                            line_index,
                        )
                })
                .map(|matched| matched.start())
                .collect()
        })
}

fn pattern_runs_on_artifact(pattern: &AntiPattern, kind: ArtifactKind) -> bool {
    // Compiled `.anvil` patterns declare `targets`; skip if the artifact's
    // kind is not listed. Legacy patterns (hardcoded `PATTERN_DEFS`) have
    // `targets: None` and are treated as source-only, preserving
    // pre-ANVFMT-008 behaviour.
    match &pattern.targets {
        Some(targets) => targets.iter().any(|t| t == kind.as_str()),
        None => kind == ArtifactKind::Source,
    }
}

/// Scan an artifact for anti-patterns.
///
/// The scanner filters the pattern catalogue to the subset whose detection
/// is meaningful for the artifact's kind:
///   - Compiled `.anvil` patterns carry an explicit `targets` list —
///     artifacts with a kind outside that list are skipped.
///   - Legacy hardcoded patterns have no `targets` and are treated as
///     source-only.
///   - File-extension and allowlist filtering only applies to `source`
///     artifacts; for PR bodies / commit messages / agent output the
///     `reference` is not a path.
#[must_use]
pub fn scan_artifact(artifact: &Artifact, options: Option<&ScanOptions>) -> ScanResult {
    let scan_options = options.cloned().unwrap_or_default();
    let prepared_patterns = prepared_patterns_for(&scan_options);
    let lines = artifact.content.split('\n').collect::<Vec<_>>();
    let is_source = artifact.kind == ArtifactKind::Source;
    // GH #1914: for code-construct rules (see `rule_is_code_scoped`), mask
    // comment + string spans in source artifacts so the rule does not match
    // `!` / `any` / etc. that appear inside comments or string literals. The
    // masker preserves byte offsets, so match columns stay accurate.
    //
    // Masking is OPT-IN per rule, not global: many rules deliberately target
    // comments (AP-001 `// eslint-disable`, AP-004/-005 `@ts-ignore`, DD-*
    // `// TODO|HACK`) or prose (RL-*), and must keep seeing the raw text.
    // Suppression directives also live *inside* comments, so suppression
    // detection below always reads the ORIGINAL `lines`. Non-source artifacts
    // (PR bodies, commit messages, agent output) are prose — never masked.
    //
    // Masking is only built when this artifact will actually run a
    // code-scoped rule — masking is O(file) work, so skipping it when no
    // such rule is configured keeps the common prose / non-code-rule path
    // allocation-free (council ALLOC-001).
    let needs_mask = is_source
        && prepared_patterns
            .iter()
            .any(|prepared| rule_is_code_scoped(&prepared.pattern.id));
    let masked_lines: Vec<String> = if needs_mask {
        super::mask::mask_non_code_lines(&lines)
    } else {
        Vec::new()
    };
    let masked_view: Vec<&str> = masked_lines.iter().map(String::as_str).collect();
    let mut warnings = Vec::new();

    for prepared in &prepared_patterns {
        if !pattern_runs_on_artifact(&prepared.pattern, artifact.kind) {
            continue;
        }

        if is_source {
            let effective_extensions =
                if let Some(pattern_extensions) = &prepared.pattern.file_extensions {
                    Some(pattern_extensions.as_slice())
                } else if prepared.pattern.all_file_types {
                    None
                } else {
                    Some(LEGACY_JS_TS_EXTENSIONS_OWNED.as_slice())
                };

            if let Some(extensions) = effective_extensions
                && !matches_file_extension(&artifact.reference, extensions)
            {
                continue;
            }
            if is_file_allowlisted_compiled(&artifact.reference, &prepared.allowlist_regexes) {
                continue;
            }
        }

        // Choose the line view for this rule: masked (comments/strings
        // blanked) for code-construct rules on source, raw otherwise. The
        // same view feeds `find_match_columns`'s multi-line context (e.g.
        // GS-001's `.has()/.set()` map-guard lookback). For code-scoped
        // rules that context is the masked view by design — a guard that
        // only appears inside a comment must not suppress a real finding.
        let rule_lines: &[&str] = if is_source && rule_is_code_scoped(&prepared.pattern.id) {
            &masked_view
        } else {
            &lines
        };

        for line_index in 0..rule_lines.len() {
            let line_number = line_index + 1;
            let columns =
                find_match_columns(prepared, rule_lines[line_index], rule_lines, line_index);
            for column in columns {
                let suppressed = if is_source {
                    suppression_for_line(&lines, line_number, &prepared.pattern.id)
                } else {
                    None
                };
                warnings.push(create_warning_from_match(
                    &prepared.pattern,
                    &artifact.reference,
                    line_number,
                    column,
                    suppressed,
                ));
            }
        }
    }

    // Keep output deterministic — downstream consumers (JSON serialisers,
    // snapshot tests, the TUI results pane) rely on a stable order.
    warnings.sort_by(|a, b| {
        a.location
            .line
            .cmp(&b.location.line)
            .then_with(|| a.location.column.cmp(&b.location.column))
            .then_with(|| a.id.cmp(&b.id))
    });

    ScanResult {
        file: artifact.reference.clone(),
        artifact_type: artifact.kind,
        warnings,
        patterns_checked: prepared_patterns
            .iter()
            .map(|prepared| prepared.pattern.id.clone())
            .collect(),
    }
}

/// Scan a source file's content for anti-patterns. Backward-compatible
/// wrapper around `scan_artifact` with `kind: Source`.
#[must_use]
pub fn scan_file(file_path: &str, content: &str, options: Option<&ScanOptions>) -> ScanResult {
    scan_artifact(&Artifact::source(file_path, content), options)
}

/// Scan multiple artifacts for anti-patterns.
///
/// Artifacts are scanned concurrently on the rayon thread pool. The per-pattern
/// regex cache (`PREPARED_PATTERNS`) is `Send + Sync` and shared across worker
/// threads, so each artifact pays only its own matching cost. Output ordering
/// matches the input slice.
#[must_use]
pub fn scan_artifacts(artifacts: &[Artifact], options: Option<&ScanOptions>) -> Vec<ScanResult> {
    artifacts
        .par_iter()
        .map(|artifact| scan_artifact(artifact, options))
        .collect()
}

#[must_use]
pub fn scan_files(files: &[(&str, &str)], options: Option<&ScanOptions>) -> Vec<ScanResult> {
    files
        .par_iter()
        .map(|(path, content)| scan_file(path, content, options))
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::antipattern::scanner::{ScanOptions, scan_file};

    #[test]
    fn scans_default_patterns_only() {
        let content = "const value: any = input;\nconsole.log(value);";
        let result = scan_file("src/app.ts", content, None);

        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings.iter().any(|warning| warning.id == "AP-003"));
        assert!(!result.warnings.iter().any(|warning| warning.id == "AP-007"));
    }

    #[test]
    fn include_opt_in_detects_console_pattern() {
        let options = ScanOptions {
            patterns: None,
            include_opt_in: true,
        };
        let result = scan_file("src/app.ts", "console.log('x')", Some(&options));
        assert!(result.warnings.iter().any(|warning| warning.id == "AP-007"));
    }

    #[test]
    fn filters_by_requested_pattern_ids() {
        let options = ScanOptions {
            patterns: Some(vec!["AP-006".to_string()]),
            include_opt_in: true,
        };
        let content = "try { x(); } catch (e) {}\nconst v: any = x;";
        let result = scan_file("src/app.ts", content, Some(&options));

        assert_eq!(result.patterns_checked, vec!["AP-006"]);
        assert_eq!(result.warnings.len(), 1);
        assert_eq!(result.warnings[0].id, "AP-006");
    }

    #[test]
    fn applies_legacy_js_ts_extension_defaults() {
        let js_result = scan_file("src/a.ts", "const v: any = input;", None);
        let html_result = scan_file("src/a.html", "const v: any = input;", None);

        assert_eq!(js_result.warnings.len(), 1);
        assert!(html_result.warnings.is_empty());
    }

    #[test]
    fn allowlist_skips_paths_matching_glob_rules() {
        let result = scan_file("src/foo/__tests__/sample.ts", "const x: any = 1;", None);
        assert!(result.warnings.is_empty());
    }

    // V050F-006: pin the allowlist-cache shape and matcher behaviour
    // so a future refactor that moves regex compilation back into the
    // hot path is caught.

    #[test]
    fn prepare_pattern_caches_one_regex_per_allowlist_entry() {
        use crate::antipattern::types::{AntiPattern, AntiPatternCategory, Confidence};

        let pattern = AntiPattern {
            id: "T-CACHE".to_string(),
            name: "test".to_string(),
            category: AntiPatternCategory::CodeQuality,
            severity: crate::antipattern::types::WarningSeverity::Info,
            confidence: Confidence::Low,
            regex: "foo".to_string(),
            title: "Cache test".to_string(),
            explanation: String::new(),
            suggestion: String::new(),
            nudge: None,
            file_extensions: None,
            all_file_types: true,
            allowlist: vec![
                "**/__tests__/**".to_string(),
                "src/foo.ts".to_string(),
                "*.test.ts".to_string(),
            ],
            threshold: None,
            enabled: true,
            opt_in: false,
            family: None,
            definition_ref: None,
            spectrum_position: None,
            targets: None,
        };
        let prepared = super::prepare_pattern(pattern);
        assert_eq!(
            prepared.allowlist_regexes.len(),
            3,
            "every allowlist entry must produce exactly one cache slot"
        );
        for (entry, source) in prepared
            .allowlist_regexes
            .iter()
            .zip(prepared.pattern.allowlist.iter())
        {
            assert!(
                entry.regex.is_some(),
                "well-formed glob {source:?} must compile in prepare_pattern",
            );
        }
    }

    #[test]
    fn compiled_allowlist_matcher_preserves_match_base_semantics() {
        // Bare-name globs (no `/`) match against the basename, full-
        // path globs match against the full normalised path. This
        // mirrors the historical `glob_match(_, _, match_base=true)`
        // contract from the pre-cache implementation.
        let allowlist =
            super::compile_allowlist(&["*.test.ts".to_string(), "src/foo/**".to_string()]);
        assert!(
            super::is_file_allowlisted_compiled("src/foo/sample.test.ts", &allowlist),
            "bare *.test.ts must match basename"
        );
        assert!(
            super::is_file_allowlisted_compiled("src/foo/bar.ts", &allowlist),
            "src/foo/** must match full path"
        );
        assert!(
            !super::is_file_allowlisted_compiled("src/baz/bar.ts", &allowlist),
            "src/baz/bar.ts must not be allowlisted"
        );
    }

    #[test]
    fn compiled_allowlist_treats_uncompilable_entries_as_no_match() {
        // The legacy `glob_to_regex(...).is_some_and(...)` shape
        // returned `false` when compilation failed. Pin the
        // cache-side equivalent: an `AllowlistGlob` with `regex:
        // None` must never match, regardless of `is_path_glob`.
        let entries = vec![super::AllowlistGlob {
            is_path_glob: true,
            regex: None,
        }];
        assert!(
            !super::is_file_allowlisted_compiled("src/uncompilable.ts", &entries),
            "regex: None must short-circuit to no-match"
        );
    }

    #[test]
    fn suppression_on_previous_line_marks_warning_as_suppressed() {
        let content = "// @anvil-ignore AP-003 -- legacy contract\nconst value: any = input;";
        let result = scan_file("src/app.ts", content, None);

        assert_eq!(result.warnings.len(), 1);
        let warning = &result.warnings[0];
        assert!(warning.suppressed.is_some());
        if let Some(suppression) = &warning.suppressed {
            assert_eq!(suppression.reason, "legacy contract");
        }
    }

    #[test]
    fn suppression_does_not_apply_to_different_pattern() {
        let content = "// @anvil-ignore AP-001\nconst value: any = input;";
        let result = scan_file("src/app.ts", content, None);

        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].suppressed.is_none());
    }

    #[test]
    fn finds_multiple_matches_per_line() {
        let options = ScanOptions {
            patterns: Some(vec!["AP-002".to_string()]),
            include_opt_in: true,
        };
        let content = "/* eslint-disable foo */ // eslint-disable-next-line bar";
        let result = scan_file("src/app.ts", content, Some(&options));

        assert_eq!(result.warnings.len(), 2);
    }

    #[test]
    fn handles_ap001_negative_lookahead_semantics() {
        let options = ScanOptions {
            patterns: Some(vec!["AP-001".to_string()]),
            include_opt_in: true,
        };
        let content = "// eslint-disable-next-line no-console\n// eslint-disable";
        let result = scan_file("src/app.ts", content, Some(&options));

        assert_eq!(result.warnings.len(), 1);
        assert_eq!(result.warnings[0].location.line, 2);
    }

    #[test]
    fn suppression_requires_comment_syntax() {
        let content =
            "console.log('@anvil-ignore AP-003 -- not a comment');\nconst value: any = input;";
        let result = scan_file("src/app.ts", content, None);

        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].suppressed.is_none());
    }

    #[test]
    fn suppression_works_with_hash_comment() {
        let content = "# @anvil-ignore AP-003 -- legacy\nconst value: any = input;";
        let result = scan_file("src/app.ts", content, None);

        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].suppressed.is_some());
    }

    // GH #1914: code-construct rules (AP-003, GS-001) must not fire on
    // `!` / `any` that appear inside comments or string literals. The
    // scanner runs these rules against a comment/string-masked view.

    #[test]
    fn gs001_does_not_fire_on_bang_inside_string_literal() {
        // Reported false positive: user-facing copy like "Account created!".
        let content = r#"setSuccess("Account created! Please check your email");"#;
        let result = scan_file("src/AuthForms.tsx", content, None);
        assert!(
            result.warnings.iter().all(|w| w.id != "GS-001"),
            "GS-001 must not fire inside a string literal: {:?}",
            result.warnings
        );
    }

    #[test]
    fn gs001_does_not_fire_on_bang_inside_line_comment() {
        // Reported false positive: "// NOTE: ... they stay as both!".
        let content =
            "const members = both; // NOTE: keep members and syndicates, they stay as both!";
        let result = scan_file("src/route.ts", content, None);
        assert!(
            result.warnings.iter().all(|w| w.id != "GS-001"),
            "GS-001 must not fire inside a comment: {:?}",
            result.warnings
        );
    }

    #[test]
    fn ap003_does_not_fire_on_any_inside_comment() {
        // `as any` inside a comment is prose, not a real cast.
        let content = "// the value may be cast as any legacy shape here\nconst x = 1;";
        let result = scan_file("src/util.ts", content, None);
        assert!(
            result.warnings.iter().all(|w| w.id != "AP-003"),
            "AP-003 must not fire inside a comment: {:?}",
            result.warnings
        );
    }

    #[test]
    fn ap003_does_not_fire_on_any_inside_string_literal() {
        let content = r#"const label = "accepts any value";"#;
        let result = scan_file("src/util.ts", content, None);
        assert!(
            result.warnings.iter().all(|w| w.id != "AP-003"),
            "AP-003 must not fire inside a string: {:?}",
            result.warnings
        );
    }

    #[test]
    fn gs001_still_fires_on_real_non_null_assertion() {
        let content = "const name = user!.profile.name;";
        let result = scan_file("src/real.ts", content, None);
        assert!(
            result.warnings.iter().any(|w| w.id == "GS-001"),
            "GS-001 must still fire on a real non-null assertion: {:?}",
            result.warnings
        );
    }

    #[test]
    fn ap003_still_fires_after_a_masked_string_on_same_line() {
        // Column-accuracy guard: a masked string earlier on the line must
        // not shift the real `: any` match off its true byte column.
        let content = r#"log("done!"); const v: any = compute();"#;
        let result = scan_file("src/real.ts", content, None);
        let ap003: Vec<_> = result
            .warnings
            .iter()
            .filter(|w| w.id == "AP-003")
            .collect();
        assert_eq!(
            ap003.len(),
            1,
            "expected exactly one AP-003: {:?}",
            result.warnings
        );
        assert_eq!(
            ap003[0].location.column.expect("column"),
            content.find(": any").expect("offset"),
            "AP-003 reported at the wrong column after a masked string"
        );
    }

    #[test]
    fn ap003_still_fires_after_regex_literal() {
        // Regex literals must not mis-trigger comment/string masking that
        // would swallow the real `: any` later on the line (adversarial
        // F-1/F-2 false-negative guard).
        let content = r#"const re = /["']\/\//; const v: any = 1;"#;
        let result = scan_file("src/real.ts", content, None);
        assert!(
            result.warnings.iter().any(|w| w.id == "AP-003"),
            "AP-003 must still fire after a regex literal: {:?}",
            result.warnings
        );
    }

    #[test]
    fn gs001_does_not_fire_on_bang_inside_regex_literal() {
        let content = "const re = /user![A-Z]/;";
        let result = scan_file("src/real.ts", content, None);
        assert!(
            result.warnings.iter().all(|w| w.id != "GS-001"),
            "GS-001 must not fire on `!` inside a regex literal: {:?}",
            result.warnings
        );
    }

    #[test]
    fn ap003_template_literal_text_is_a_known_tradeoff() {
        // KNOWN-TRADEOFF (GH #1914): template-literal TEXT is left unmasked
        // so `${…}` interpolation code keeps being scanned. A consequence is
        // that `as any` in template prose still fires. This test pins the
        // deliberate behaviour so a future change to mask backtick text is a
        // conscious decision, not a silent regression.
        let content = "const msg = `cast as any value`;";
        let result = scan_file("src/real.ts", content, None);
        assert!(
            result.warnings.iter().any(|w| w.id == "AP-003"),
            "template-literal text trade-off changed: {:?}",
            result.warnings
        );
    }

    #[test]
    fn gs001_guard_inside_comment_does_not_suppress() {
        // A `.has(k)` guard that only appears in a comment must not suppress
        // a real `map.get(k)!` — the code-scoped rule sees masked context
        // (CORRECTNESS-001), so the commented guard is invisible.
        let content =
            "const m = new Map();\n// m.has(k) was checked elsewhere\nconst v = m.get(k)!;";
        let result = scan_file("src/real.ts", content, None);
        assert!(
            result
                .warnings
                .iter()
                .any(|w| w.id == "GS-001" && w.location.line == 3),
            "GS-001 must fire when the guard is only in a comment: {:?}",
            result.warnings
        );
    }

    // v0.5.0 ESLint-disable awareness — an explicit
    // `// eslint-disable-next-line` directive (with or without a
    // reason after `--`) is a documented suppression of the same
    // class of issue Anvil's AP-* family flags. Honouring it lets
    // existing TS/JS suppressions co-exist with Anvil without forcing
    // double-suppression via `@anvil-ignore`.

    #[test]
    fn eslint_disable_next_line_suppresses_ap003() {
        let content = "// eslint-disable-next-line @typescript-eslint/no-explicit-any -- EventEmitter base requires any[]\noverride on(event: string, listener: (...args: any[]) => void): this {";
        let result = scan_file("src/file-watcher.ts", content, None);
        let unsuppressed: Vec<_> = result
            .warnings
            .iter()
            .filter(|w| w.id == "AP-003" && w.suppressed.is_none())
            .collect();
        assert!(
            unsuppressed.is_empty(),
            "eslint-disable-next-line for no-explicit-any should suppress AP-003"
        );
    }

    #[test]
    fn eslint_disable_next_line_without_rule_suppresses_ap_family() {
        let content = "// eslint-disable-next-line\nconst v: any = input;";
        let result = scan_file("src/app.ts", content, None);
        let unsuppressed: Vec<_> = result
            .warnings
            .iter()
            .filter(|w| w.id == "AP-003" && w.suppressed.is_none())
            .collect();
        assert!(
            unsuppressed.is_empty(),
            "bare eslint-disable-next-line should suppress AP family"
        );
    }

    #[test]
    fn eslint_disable_block_suppresses_following_lines() {
        let content = "/* eslint-disable @typescript-eslint/no-explicit-any */\nconst v: any = input;\nconst w: any = other;";
        let result = scan_file("src/app.ts", content, None);
        let unsuppressed: Vec<_> = result
            .warnings
            .iter()
            .filter(|w| w.id == "AP-003" && w.suppressed.is_none())
            .collect();
        assert!(
            unsuppressed.is_empty(),
            "block eslint-disable should suppress AP-003 on subsequent lines"
        );
    }

    #[test]
    fn eslint_disable_does_not_suppress_unrelated_findings() {
        // An eslint-disable-next-line still on the SAME line as a
        // *different* issue should not over-reach. AP-006 (empty
        // catch) and AP-003 are independent.
        let content = "try { go() } catch (e) {}\n// eslint-disable-next-line @typescript-eslint/no-explicit-any\nconst v: any = input;";
        let result = scan_file("src/app.ts", content, None);
        let ap006_unsuppressed: Vec<_> = result
            .warnings
            .iter()
            .filter(|w| w.id == "AP-006" && w.suppressed.is_none())
            .collect();
        assert!(
            !ap006_unsuppressed.is_empty(),
            "AP-006 (empty catch) should still fire when only AP-003 was suppressed"
        );
    }

    // v0.5.0 GS-001 false positives — the Map.get-after-has-set idiom
    // is a canonical guarded-lookup pattern flagged by the regex despite
    // the runtime guarantee being explicit. Reproduces from
    // packages/adapters/src/base/file-discovery.ts:301 and
    // packages/anvil/runtime/src/export/formatters/llms-txt-formatter.ts:195.

    #[test]
    fn does_not_flag_guarded_map_get_after_has_set() {
        let content = "    if (!groups.has(pattern)) {\n      groups.set(pattern, []);\n    }\n    groups.get(pattern)!.push(file);";
        let result = scan_file("src/grouper.ts", content, None);
        let gs_warnings: Vec<_> = result
            .warnings
            .iter()
            .filter(|w| w.id == "GS-001" && w.suppressed.is_none())
            .collect();
        assert!(
            gs_warnings.is_empty(),
            "guarded Map.get should not trigger GS-001, got: {:?}",
            gs_warnings
                .iter()
                .map(|w| w.location.line)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn does_not_flag_guarded_map_get_with_indented_block() {
        // Wider indentation, deeper nesting — shape that matches the
        // actual `prompt-formatter.ts:198` site.
        let content = "      const byCategory = new Map();\n      for (const pattern of constraints.antiPatterns) {\n        const category = pattern.category;\n        if (!byCategory.has(category)) {\n          byCategory.set(category, []);\n        }\n        byCategory.get(category)!.push(pattern);\n      }";
        let result = scan_file("src/formatter.ts", content, None);
        let gs_warnings: Vec<_> = result
            .warnings
            .iter()
            .filter(|w| w.id == "GS-001" && w.suppressed.is_none())
            .collect();
        assert!(
            gs_warnings.is_empty(),
            "guarded Map.get inside a for loop should not trigger GS-001"
        );
    }

    #[test]
    fn still_flags_unguarded_non_null_assertion() {
        // Regression guard: a bare `obj!.prop` with no preceding guard
        // on the same key must still fire.
        let content = "function unsafe(maybe?: Foo) {\n  return maybe!.value;\n}";
        let result = scan_file("src/unsafe.ts", content, None);
        let has_gs001 = result
            .warnings
            .iter()
            .any(|w| w.id == "GS-001" && w.suppressed.is_none());
        assert!(has_gs001, "unguarded `maybe!.value` must still fire GS-001");
    }

    #[test]
    fn warning_carries_family_provenance_from_pattern() {
        use crate::antipattern::registry_loader::{
            LoadRegistryOptions, load_registry_patterns, reset_registry_cache,
        };
        use std::path::PathBuf;

        reset_registry_cache();
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let registry = manifest
            .parent()
            .and_then(std::path::Path::parent)
            .expect("workspace root")
            .join("patterns/compiled/registry.json");

        let registry_patterns = load_registry_patterns(&LoadRegistryOptions {
            registry_path: Some(registry),
        });
        let ap003 = registry_patterns
            .into_iter()
            .find(|p| p.id == "AP-003")
            .expect("AP-003 in registry");
        assert_eq!(ap003.family.as_deref(), Some("type-system-evasion"));

        let warning = super::create_warning_from_match(&ap003, "src/app.ts", 1, 0, None);
        assert_eq!(warning.family.as_deref(), Some("type-system-evasion"));
        assert!(
            warning.definition_ref.is_some(),
            "definition_ref should propagate"
        );
        assert_eq!(warning.spectrum_position, Some(1));
    }

    #[test]
    fn warning_from_pattern_without_provenance_carries_none() {
        use crate::antipattern::types::{AntiPattern, AntiPatternCategory, Confidence};

        let bare = AntiPattern {
            id: "TST-001".to_string(),
            name: "Synthetic".to_string(),
            category: AntiPatternCategory::CodeQuality,
            severity: crate::antipattern::types::WarningSeverity::Info,
            confidence: Confidence::Low,
            regex: "foo".to_string(),
            title: "Synthetic".to_string(),
            explanation: String::new(),
            suggestion: String::new(),
            nudge: None,
            file_extensions: None,
            all_file_types: true,
            allowlist: Vec::new(),
            threshold: None,
            enabled: true,
            opt_in: false,
            family: None,
            definition_ref: None,
            spectrum_position: None,
            targets: None,
        };
        let warning = super::create_warning_from_match(&bare, "src/app.ts", 1, 0, None);
        assert!(warning.family.is_none());
        assert!(warning.definition_ref.is_none());
        assert!(warning.spectrum_position.is_none());
    }

    // ---- scan_artifact: artifact-aware filtering ---------------------------

    #[test]
    fn scan_artifact_source_matches_legacy_scan_file() {
        use super::{Artifact, scan_artifact};
        use crate::antipattern::types::ArtifactKind;

        let content = "const v: any = input;";
        let via_file = scan_file("src/app.ts", content, None);
        let via_artifact = scan_artifact(
            &Artifact {
                kind: ArtifactKind::Source,
                reference: "src/app.ts".to_string(),
                content: content.to_string(),
            },
            None,
        );

        assert_eq!(via_file.warnings.len(), via_artifact.warnings.len());
        assert_eq!(via_artifact.artifact_type, ArtifactKind::Source);
    }

    #[test]
    fn pattern_with_no_targets_defaults_to_source_only() {
        use crate::antipattern::types::{
            AntiPattern, AntiPatternCategory, ArtifactKind, Confidence, WarningSeverity,
        };

        let untargeted = AntiPattern {
            id: "TST-002".to_string(),
            name: "No targets".to_string(),
            category: AntiPatternCategory::CodeQuality,
            severity: WarningSeverity::Info,
            confidence: Confidence::Low,
            regex: "x".to_string(),
            title: "No targets".to_string(),
            explanation: String::new(),
            suggestion: String::new(),
            nudge: None,
            file_extensions: None,
            all_file_types: true,
            allowlist: Vec::new(),
            threshold: None,
            enabled: true,
            opt_in: false,
            family: None,
            definition_ref: None,
            spectrum_position: None,
            targets: None,
        };

        assert!(super::pattern_runs_on_artifact(
            &untargeted,
            ArtifactKind::Source
        ));
        assert!(!super::pattern_runs_on_artifact(
            &untargeted,
            ArtifactKind::PrDescription
        ));
        assert!(!super::pattern_runs_on_artifact(
            &untargeted,
            ArtifactKind::CommitMessage
        ));
        assert!(!super::pattern_runs_on_artifact(
            &untargeted,
            ArtifactKind::AgentOutput
        ));
    }

    #[test]
    fn prepare_pattern_records_compile_error_on_broken_regex() {
        use crate::antipattern::types::{AntiPattern, AntiPatternCategory, Confidence};

        let broken = AntiPattern {
            id: "BROKEN-001".to_string(),
            name: "Broken".to_string(),
            category: AntiPatternCategory::CodeQuality,
            severity: crate::antipattern::types::WarningSeverity::Warning,
            confidence: Confidence::Low,
            // PCRE lookaround that the regex crate cannot compile.
            regex: r"foo(?!bar)".to_string(),
            title: "Broken rule".to_string(),
            explanation: String::new(),
            suggestion: String::new(),
            nudge: None,
            file_extensions: None,
            all_file_types: true,
            allowlist: Vec::new(),
            threshold: None,
            enabled: true,
            opt_in: false,
            family: None,
            definition_ref: None,
            spectrum_position: None,
            targets: None,
        };
        let prepared = super::prepare_pattern(broken);
        assert!(
            prepared.primary_regex.is_none(),
            "lookaround regex must not compile"
        );
        assert!(
            prepared.compile_error.is_some(),
            "compile error must be captured so doctor can surface it"
        );
    }

    // ---- SPG-003: hand-coded rewrites of PCRE-lookaround rules ------------

    fn registry_pattern(id: &str) -> crate::antipattern::types::AntiPattern {
        use crate::antipattern::registry_loader::{
            LoadRegistryOptions, load_registry_patterns, reset_registry_cache,
        };
        use std::path::PathBuf;

        reset_registry_cache();
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let registry = manifest
            .parent()
            .and_then(std::path::Path::parent)
            .expect("workspace root")
            .join("patterns/compiled/registry.json");
        load_registry_patterns(&LoadRegistryOptions {
            registry_path: Some(registry),
        })
        .into_iter()
        .find(|p| p.id == id)
        .unwrap_or_else(|| panic!("{id} missing from registry"))
    }

    fn scan_with(pattern_id: &str, reference: &str, content: &str) -> Vec<String> {
        let options = ScanOptions {
            patterns: Some(vec![pattern_id.to_string()]),
            include_opt_in: true,
        };
        scan_file(reference, content, Some(&options))
            .warnings
            .into_iter()
            .map(|w| format!("{}:{}", w.id, w.location.line))
            .collect()
    }

    fn scan_artifact_with(
        pattern_id: &str,
        kind: crate::antipattern::types::ArtifactKind,
        reference: &str,
        content: &str,
    ) -> Vec<String> {
        use super::{Artifact, scan_artifact};
        let options = ScanOptions {
            patterns: Some(vec![pattern_id.to_string()]),
            include_opt_in: true,
        };
        scan_artifact(
            &Artifact {
                kind,
                reference: reference.to_string(),
                content: content.to_string(),
            },
            Some(&options),
        )
        .warnings
        .into_iter()
        .map(|w| format!("{}:{}", w.id, w.location.line))
        .collect()
    }

    #[test]
    fn dd001_fires_on_untracked_todo_and_suppresses_when_tracked() {
        // Ensure the rule is live via registry (prepare_pattern is called
        // through the catalogue).
        let _ = registry_pattern("DD-001");

        // Positive: TODO with no tracking reference.
        assert_eq!(
            scan_with("DD-001", "src/a.ts", "// TODO refactor later\n"),
            vec!["DD-001:1"],
        );
        // Escape via ticket ID.
        assert!(
            scan_with("DD-001", "src/a.ts", "// TODO(PROJ-123): refactor\n").is_empty(),
            "ticket ID should suppress DD-001",
        );
        // Escape via #123.
        assert!(
            scan_with("DD-001", "src/a.ts", "// FIXME see #456 for details\n").is_empty(),
            "#\\d+ should suppress DD-001",
        );
        // Escape via 'issue' keyword.
        assert!(
            scan_with("DD-001", "src/a.ts", "// FIXME file issue later\n").is_empty(),
            "'issue' keyword should suppress DD-001",
        );
    }

    #[test]
    fn dd002_fires_on_untracked_hack_and_suppresses_when_tracked() {
        let _ = registry_pattern("DD-002");
        assert_eq!(
            scan_with("DD-002", "src/a.ts", "// HACK force auth for admins\n"),
            vec!["DD-002:1"],
        );
        assert!(scan_with("DD-002", "src/a.ts", "// HACK(#42) force auth\n").is_empty(),);
        assert!(scan_with("DD-002", "src/a.ts", "// XXX see ticket before ship\n").is_empty(),);
    }

    #[test]
    fn dd003_fires_on_temporary_without_timeline_escape() {
        let _ = registry_pattern("DD-003");
        assert_eq!(
            scan_with("DD-003", "src/a.ts", "// temporary fix\n"),
            vec!["DD-003:1"],
        );
        assert!(scan_with("DD-003", "src/a.ts", "// temporary until next release\n").is_empty(),);
        assert!(
            scan_with(
                "DD-003",
                "src/a.ts",
                "// workaround; remove after migration\n"
            )
            .is_empty(),
        );
    }

    #[test]
    fn gs001_fires_on_non_null_assertion_only_when_lookahead_holds() {
        let _ = registry_pattern("GS-001");

        // Positive: `value!;`
        assert_eq!(
            scan_with("GS-001", "src/a.ts", "const x = value!;\n"),
            vec!["GS-001:1"],
        );
        // Positive: `a!.b`
        assert_eq!(
            scan_with("GS-001", "src/a.ts", "const x = a!.b;\n"),
            vec!["GS-001:1"],
        );
        // Positive: `value!` at end of line.
        assert_eq!(
            scan_with("GS-001", "src/a.ts", "return value!\n"),
            vec!["GS-001:1"],
        );
        // Negative: logical NOT has no word char before `!`.
        assert!(scan_with("GS-001", "src/a.ts", "if (!value) return;\n").is_empty(),);
        // Negative: `value!!` double-bang is not a non-null assert under the
        // original positive-lookahead spec.
        assert!(scan_with("GS-001", "src/a.ts", "const x = value!!foo;\n").is_empty(),);
    }

    #[test]
    fn gs001_guarded_map_get_requires_same_receiver() {
        let _ = registry_pattern("GS-001");

        assert!(
            scan_with(
                "GS-001",
                "src/a.ts",
                "if (!cache.has(id)) cache.set(id, []);\ncache.get(id)!.push(value);\n",
            )
            .is_empty(),
        );
        assert_eq!(
            scan_with(
                "GS-001",
                "src/a.ts",
                "if (!other.has(id)) other.set(id, []);\ncache.get(id)!.push(value);\n",
            ),
            vec!["GS-001:2"],
        );
        assert_eq!(
            scan_with(
                "GS-001",
                "src/a.ts",
                "if (!other.cache.has(id)) other.cache.set(id, []);\ncache.get(id)!.push(value);\n",
            ),
            vec!["GS-001:2"],
        );
        assert_eq!(
            scan_with(
                "GS-001",
                "src/a.ts",
                "if (!cache.has(id2)) cache.set(id2, []);\ncache.get(id)!.push(value);\n",
            ),
            vec!["GS-001:2"],
        );
    }

    #[test]
    fn rl001_fires_case_insensitively_and_honours_verified_escape() {
        use crate::antipattern::types::ArtifactKind;
        let _ = registry_pattern("RL-001");

        assert_eq!(
            scan_artifact_with(
                "RL-001",
                ArtifactKind::PrDescription,
                "pr/100",
                "This is a pre-existing failure unrelated to my change.\n",
            ),
            vec!["RL-001:1"],
        );
        // Case-insensitive.
        assert_eq!(
            scan_artifact_with(
                "RL-001",
                ArtifactKind::PrDescription,
                "pr/101",
                "PRE-EXISTING failure noted.\n",
            ),
            vec!["RL-001:1"],
        );
        // Escape via `verified`.
        assert!(
            scan_artifact_with(
                "RL-001",
                ArtifactKind::PrDescription,
                "pr/102",
                "pre-existing failure, verified in run #123.\n",
            )
            .is_empty(),
        );
    }

    #[test]
    fn spg003_rewrite_matches_registry_snapshot() {
        // Adversarial-reviewer M-1: the hand-coded rewrite in `rewrite_spec`
        // is decoupled from the live registry. If the `.anvil` source is
        // edited and recompiled, the Rust scanner would silently keep using
        // the stale rewrite. This snapshot compares both the registry regex
        // and flags against the rewrite's recorded expectation and fires on
        // any drift so the rewrite gets revisited.
        use crate::antipattern::registry_loader::{
            CompiledPattern, Detection, LoadRegistryOptions, load_compiled_registry,
            reset_registry_cache,
        };
        use std::path::PathBuf;

        reset_registry_cache();
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let registry_path = manifest
            .parent()
            .and_then(std::path::Path::parent)
            .expect("workspace root")
            .join("patterns/compiled/registry.json");
        let registry = load_compiled_registry(&LoadRegistryOptions {
            registry_path: Some(registry_path),
        })
        .registry
        .expect("registry loads");

        for id in ["DD-001", "DD-002", "DD-003", "GS-001", "RL-001", "RL-005"] {
            let spec = super::rewrite_spec(id).unwrap_or_else(|| panic!("{id} spec missing"));
            let compiled: &CompiledPattern = registry
                .patterns
                .iter()
                .find(|p| p.id == id)
                .unwrap_or_else(|| panic!("{id} missing from registry"));
            let (actual_pattern, actual_flags) = match &compiled.detection {
                Detection::Regex { pattern, flags } => (pattern.as_str(), flags.as_deref()),
                Detection::Ast { .. } => panic!("{id} is AST, not regex"),
            };
            assert_eq!(
                actual_pattern, spec.expected_registry_regex,
                "{id}: registry detection.regex drifted from hand-coded rewrite. \
                 If this is intentional, update `expected_registry_regex` AND \
                 `base_regex`/`filter` in `rewrite_spec` together.",
            );
            assert_eq!(
                actual_flags, spec.expected_registry_flags,
                "{id}: registry flags drifted from rewrite expectation. \
                 If this is intentional, update `expected_registry_flags` in `rewrite_spec`.",
            );
        }
    }

    #[test]
    fn prepare_pcre_rewrite_surfaces_base_regex_compile_error() {
        // Synthetic pattern shaped like DD-001 but swapped with a deliberately
        // broken regex — covers the compile-error path even though none of the
        // shipping rules trigger it today.
        use crate::antipattern::types::{AntiPattern, AntiPatternCategory, Confidence};
        let bad = AntiPattern {
            id: "DD-001".to_string(),
            name: "Broken".to_string(),
            category: AntiPatternCategory::DeferredDebt,
            severity: crate::antipattern::types::WarningSeverity::Warning,
            confidence: Confidence::High,
            regex: r"[unmatched".to_string(),
            title: "Broken rewrite test".to_string(),
            explanation: String::new(),
            suggestion: String::new(),
            nudge: None,
            file_extensions: None,
            all_file_types: true,
            allowlist: Vec::new(),
            threshold: None,
            enabled: true,
            opt_in: false,
            family: None,
            definition_ref: None,
            spectrum_position: None,
            targets: None,
        };
        // The rewrite routes the *hand-coded* base regex through the compile
        // path, not the registry pattern, so DD-001's hand-coded regex
        // compiles cleanly here. To exercise the error path we construct the
        // error directly via `rewrite_compile_error`. Source the broken
        // pattern from a runtime string so clippy's regex-literal lint
        // doesn't reject the test.
        let broken = String::from(r"[unmatched");
        let err = regex::Regex::new(&broken).unwrap_err();
        let prepared = super::rewrite_compile_error(&bad, "base regex", &err);
        assert!(prepared.primary_regex.is_none());
        let msg = prepared.compile_error.expect("compile_error populated");
        assert!(msg.contains("SPG-003"));
        assert!(msg.contains("DD-001"));
        assert!(msg.contains("base regex"));
    }

    #[test]
    fn rl005_fires_case_insensitively_and_honours_issue_escape() {
        use crate::antipattern::types::ArtifactKind;
        let _ = registry_pattern("RL-005");

        assert_eq!(
            scan_artifact_with(
                "RL-005",
                ArtifactKind::PrDescription,
                "pr/200",
                "Will defer this work.\n",
            ),
            vec!["RL-005:1"],
        );
        // Case-insensitive.
        assert_eq!(
            scan_artifact_with(
                "RL-005",
                ArtifactKind::PrDescription,
                "pr/201",
                "DEFERRED to next cycle.\n",
            ),
            vec!["RL-005:1"],
        );
        // Escape via `issue #`.
        assert!(
            scan_artifact_with(
                "RL-005",
                ArtifactKind::PrDescription,
                "pr/202",
                "Will defer this work; tracked in issue #42.\n",
            )
            .is_empty(),
        );
    }

    #[test]
    fn prepare_pattern_leaves_compile_error_none_on_clean_regex() {
        use crate::antipattern::types::{AntiPattern, AntiPatternCategory, Confidence};

        let ok = AntiPattern {
            id: "OK-001".to_string(),
            name: "OK".to_string(),
            category: AntiPatternCategory::CodeQuality,
            severity: crate::antipattern::types::WarningSeverity::Warning,
            confidence: Confidence::Low,
            regex: r"\bfoo\b".to_string(),
            title: "OK rule".to_string(),
            explanation: String::new(),
            suggestion: String::new(),
            nudge: None,
            file_extensions: None,
            all_file_types: true,
            allowlist: Vec::new(),
            threshold: None,
            enabled: true,
            opt_in: false,
            family: None,
            definition_ref: None,
            spectrum_position: None,
            targets: None,
        };
        let prepared = super::prepare_pattern(ok);
        assert!(prepared.primary_regex.is_some());
        assert!(prepared.compile_error.is_none());
    }

    #[test]
    fn scan_artifact_respects_registry_pattern_targets() {
        use super::{Artifact, scan_artifact};
        use crate::antipattern::registry_loader::{
            LoadRegistryOptions, load_registry_patterns, reset_registry_cache,
        };
        use crate::antipattern::types::ArtifactKind;
        use std::path::PathBuf;

        reset_registry_cache();
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let registry_path = manifest
            .parent()
            .and_then(std::path::Path::parent)
            .expect("workspace root")
            .join("patterns/compiled/registry.json");

        let patterns = load_registry_patterns(&LoadRegistryOptions {
            registry_path: Some(registry_path),
        });
        // Pick an AP-* pattern with targets = ["source"] only — one such is
        // AP-003 (any type usage). We prove that a pr-description artifact
        // does not trigger AP-003 even with matching content.
        let ap003 = patterns
            .iter()
            .find(|p| p.id == "AP-003")
            .cloned()
            .expect("AP-003 registry pattern");
        assert_eq!(
            ap003.targets.as_deref(),
            Some(vec!["source".to_string()].as_slice()),
            "AP-003 should target source only"
        );

        let runs_on_pr = super::pattern_runs_on_artifact(&ap003, ArtifactKind::PrDescription);
        assert!(
            !runs_on_pr,
            "source-only pattern must not run on pr-description"
        );
        let runs_on_source = super::pattern_runs_on_artifact(&ap003, ArtifactKind::Source);
        assert!(runs_on_source, "source-only pattern must run on source");

        // End-to-end: scan_artifact returns no warnings for the source-only
        // pattern against a pr-description, regardless of content.
        let result = scan_artifact(
            &Artifact {
                kind: ArtifactKind::PrDescription,
                reference: "PR#1".to_string(),
                content: "const x: any = 1;".to_string(),
            },
            Some(&ScanOptions {
                patterns: Some(vec!["AP-003".to_string()]),
                include_opt_in: true,
            }),
        );
        assert!(result.warnings.is_empty());
    }
}
