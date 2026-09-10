use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretPatternDef {
    pub name: String,
    pub pattern: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretCheckConfig {
    pub enable_entropy: bool,
    pub entropy_threshold: f64,
    pub min_entropy_length: usize,
    pub scan_git_history: bool,
    pub git_history_depth: usize,
    pub skip_extensions: Vec<String>,
    pub custom_patterns: Vec<SecretPatternDef>,
    pub custom_allowlist: Vec<String>,
    /// SCAN-002: per-line length guard (in bytes). Lines longer than this
    /// threshold are skipped before any regex is applied so that a
    /// pathological minified/base64/concatenated line cannot trigger
    /// worst-case backtracking across the 18 built-in patterns or
    /// user-supplied custom regexes.
    ///
    /// Threshold default rationale: 4096 bytes is short enough to neutralise
    /// the realistic `ReDoS` blast radius (catastrophic backtracking on
    /// `a*a*` style regexes scales with line length) while remaining well
    /// above any line we expect to see in real source code — Prettier's
    /// `printWidth` default is 80, the longest commonly-formatted line in
    /// generated assets is sub-1 KB, and the 4 KB ceiling still leaves
    /// minified bundles excluded at the file-extension layer
    /// (`.min.js`, `.min.css`, `.map`).
    ///
    /// `serde(default)` keeps the field backward-compatible with on-disk
    /// configs written before SCAN-002.
    #[serde(default = "default_max_line_bytes")]
    pub max_line_bytes: usize,
}

/// SCAN-002 default — see `SecretCheckConfig::max_line_bytes`.
const fn default_max_line_bytes() -> usize {
    4096
}

/// Is `extension` usable as a `skip_extensions` entry: a dotted suffix of
/// ASCII alphanumerics and `. _ + -`.
///
/// Every skip path must agree on this, because every skip path narrows what
/// the secret scan covers. An **empty** entry is the dangerous case:
/// `str::ends_with("")` is true for every string, so a single `""` in
/// `skip_extensions` skips every non-lockfile file on disk and drops every
/// history line — while the check still reports "No secrets detected" and
/// passes. Rejecting whitespace and pathspec-magic characters (`:`, `(`,
/// `*`, `\`, …) additionally stops operator config from rewriting the git
/// pathspec that `build_log_args` constructs.
pub(crate) fn is_usable_skip_extension(extension: &str) -> bool {
    extension.len() > 1
        && extension.starts_with('.')
        && extension
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '+' | '-'))
}

/// Split `skip_extensions` into the entries that may narrow the scan and a
/// human-readable warning per rejected entry.
///
/// Invalid entries are reported rather than silently honoured: narrowing
/// coverage on the strength of a malformed config value is exactly the
/// false-clean this guards against.
pub(crate) fn partition_skip_extensions(skip_extensions: &[String]) -> (Vec<&str>, Vec<String>) {
    let mut usable = Vec::with_capacity(skip_extensions.len());
    let mut warnings = Vec::new();
    for extension in skip_extensions {
        if is_usable_skip_extension(extension) {
            usable.push(extension.as_str());
        } else {
            warnings.push(format!(
                "secret scan: ignoring invalid `skip_extensions` entry {extension:?} \
                 (expected a dotted suffix like \".min.js\"); it would have narrowed \
                 or disabled scanning"
            ));
        }
    }
    (usable, warnings)
}

impl Default for SecretCheckConfig {
    fn default() -> Self {
        Self {
            enable_entropy: true,
            entropy_threshold: 4.5,
            min_entropy_length: 16,
            scan_git_history: false,
            git_history_depth: 10,
            skip_extensions: vec![
                ".lock".to_string(),
                ".min.js".to_string(),
                ".min.css".to_string(),
                ".map".to_string(),
                ".svg".to_string(),
                ".png".to_string(),
                ".jpg".to_string(),
                ".jpeg".to_string(),
                ".gif".to_string(),
                ".ico".to_string(),
                ".woff".to_string(),
                ".woff2".to_string(),
                ".ttf".to_string(),
                ".eot".to_string(),
            ],
            custom_patterns: Vec::new(),
            custom_allowlist: Vec::new(),
            max_line_bytes: default_max_line_bytes(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretFinding {
    pub file: String,
    pub line: usize,
    pub finding_type: FindingType,
    pub pattern_name: String,
    pub redacted_match: String,
    pub redacted_line: String,
    /// Byte offset of the matched token within the source line (0-based).
    /// Present so machine consumers can re-read the source span without
    /// the secret being echoed in the redacted payload (Dave SEC-FP-2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub match_start: Option<usize>,
    /// Exclusive end byte offset of the matched token within the source line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub match_end: Option<usize>,
    /// Non-secret classifier for the matched token shape. Lets consumers
    /// triage without seeing the raw token (e.g. `"path"` vs `"opaque"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_shape: Option<TokenShape>,
    /// SDT-004: the vendored ruleset version that produced this finding
    /// (e.g. `"gitleaks@v8.30.1 tier1"`), or `None` for a built-in rule or the
    /// entropy heuristic.
    ///
    /// Required by the governing work item: "ruleset version appears in finding
    /// provenance". Without it a vendored detection is indistinguishable from a
    /// built-in one, so a reader triaging a finding cannot tell which catalogue
    /// version to blame — or re-check after a refresh. `serde(default)` keeps
    /// the field backward-compatible with pre-SDT-004 wire consumers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ruleset_version: Option<String>,
}

/// Coarse, non-secret token classification for machine-readable triage.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TokenShape {
    /// Path-shaped token (separators and/or document extension).
    Path,
    /// Opaque credential-like token.
    Opaque,
}

impl Default for SecretFinding {
    fn default() -> Self {
        Self {
            file: String::new(),
            line: 0,
            finding_type: FindingType::Pattern,
            pattern_name: String::new(),
            redacted_match: String::new(),
            redacted_line: String::new(),
            match_start: None,
            match_end: None,
            token_shape: None,
            ruleset_version: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FindingType {
    Pattern,
    Entropy,
}

/// Why a candidate that matched a secret pattern (or the entropy heuristic)
/// was withheld from the findings. Recorded so suppression is never silent:
/// the scanner reports *what* it chose not to flag and *which* allowlist tier
/// made that call. The security-relevant tier is [`AllowlistProvenance::Custom`]
/// — a user-supplied opt-out (today `SecretCheckConfig::custom_allowlist`, and
/// the entry point through which a future project-config allowlist surface will
/// flow). Surfacing those means a `.anvilrc` entry can never silently hide a
/// real credential without the operator seeing it called out at scan time.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AllowlistProvenance {
    /// A built-in shape-anchored entry (hex hash, `0x…`, data URI, ULID,
    /// content-addressed record id).
    BuiltinShape,
    /// A built-in documentation/test keyword (`example`, `test`, `dummy`, …).
    BuiltinKeyword,
    /// A built-in, context-bound non-secret fixture/test-vector rule.
    BuiltinBenignFixture,
    /// A user-supplied `custom_allowlist` entry. `pattern` is the source
    /// regex so the operator can trace the suppression back to the exact
    /// opt-out they configured.
    Custom { pattern: String },
    /// An in-source `@anvil-ignore` (or `-until`) directive on the line
    /// above an *ignorable* (non-high-confidence) match. High-confidence
    /// shapes never take this arm — they stay findings. `rule_id` is the
    /// directive id (`SECRET-HIGH-ENTROPY-STRING`, `SECRET-DETECTION`, …)
    /// so a withheld FP stays visible for scanner tightening.
    InlineIgnore { rule_id: String, reason: String },
}

impl AllowlistProvenance {
    /// `true` for operator-configured opt-outs — the tier worth calling out
    /// in detail, because it can mask a genuine credential.
    #[must_use]
    pub fn is_operator_configured(&self) -> bool {
        matches!(
            self,
            AllowlistProvenance::Custom { .. } | AllowlistProvenance::InlineIgnore { .. }
        )
    }
}

/// Canonical printed finding id for a secret pattern name.
///
/// `"High Entropy String"` → `SECRET-HIGH-ENTROPY-STRING`. Shared by
/// `anvil check`, SARIF, and the inline-ignore matcher so a suppression
/// written for the id `check` prints actually matches.
#[must_use]
pub fn finding_id(pattern_name: &str) -> String {
    format!(
        "SECRET-{}",
        pattern_name.to_ascii_uppercase().replace(' ', "-")
    )
}

/// Check-level ids Codex copies from MCP `rule_id` after secret redaction.
pub const CHECK_RULE_ID: &str = "secret-detection";
const CHECK_RULE_ID_UPPER: &str = "SECRET-DETECTION";

/// `true` when an `@anvil-ignore` id should withhold this non-high-confidence
/// match. High-confidence callers must not use this — they stay findings.
#[must_use]
pub fn inline_ignore_matches(parsed_id: &str, pattern_name: &str) -> bool {
    let canonical = finding_id(pattern_name);
    parsed_id == canonical
        || parsed_id.eq_ignore_ascii_case(CHECK_RULE_ID)
        || parsed_id == CHECK_RULE_ID_UPPER
        || parsed_id == "ANV-CORE-001"
}

/// Honour a previous-line `@anvil-ignore` for an ignorable secret match.
#[must_use]
pub fn inline_ignore_from_previous_line(
    previous: &str,
    pattern_name: &str,
) -> Option<AllowlistProvenance> {
    let (id, reason) = crate::antipattern::parse_suppression(previous)?;
    inline_ignore_matches(&id, pattern_name).then_some(AllowlistProvenance::InlineIgnore {
        rule_id: id,
        reason,
    })
}

/// `true` when `id` is the printed finding id of a high-confidence built-in
/// pattern (`SECRET-AWS-KEY`, `SECRET-GITHUB-TOKEN`, …). Used by
/// `anvil_suppress` to refuse inline ignore of real credential shapes.
#[must_use]
pub fn is_high_confidence_finding_id(id: &str) -> bool {
    let upper = id.to_ascii_uppercase();
    crate::secret::patterns::SECRET_PATTERNS
        .iter()
        .any(|pattern| pattern.high_confidence && finding_id(pattern.name) == upper)
}

/// A secret candidate that matched a pattern/entropy rule but was withheld
/// because it also matched an allowlist entry. Carries enough context to
/// surface "we suppressed a would-be `AWS Key` at foo.rs:12 via your
/// `.anvilrc` allowlist" without leaking the raw value (`redacted_match`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Suppression {
    pub file: String,
    pub line: usize,
    /// The rule that *would* have fired (e.g. `"AWS Key"`, `"High Entropy String"`).
    pub rule_name: String,
    pub redacted_match: String,
    pub provenance: AllowlistProvenance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntropyFinding {
    pub file: String,
    pub line: usize,
    pub entropy: f64,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretCheckResult {
    pub passed: bool,
    pub score: u8,
    pub message: String,
    pub findings: Vec<SecretFinding>,
    /// Configuration errors surfaced during the run — currently populated when
    /// a `custom_patterns` regex fails to compile. Wire-compatible with
    /// pre-EAMIG-003 consumers via `serde(default)`.
    #[serde(default)]
    pub pattern_errors: Vec<String>,
    /// SCAN-002: total number of lines skipped because they exceeded
    /// `SecretCheckConfig::max_line_bytes`. Surfaced so reviewers can tell
    /// "0 findings" from "we skipped a 4 MB minified bundle line that
    /// might have held a secret". `serde(default)` keeps the field
    /// backward-compatible.
    #[serde(default)]
    pub lines_skipped_oversize: usize,
    /// Candidates that matched a secret rule but were withheld because they
    /// also matched an allowlist entry. Recorded so suppression is observable
    /// rather than silent — operators can see what the scanner chose not to
    /// flag and why. `serde(default)` keeps the field backward-compatible.
    #[serde(default)]
    pub suppressions: Vec<Suppression>,
    /// Failures while running the optional git-history secret scan. Present
    /// only when `scan_git_history` was requested and the scan could not
    /// complete (I/O, missing git, non-zero `git log`, …). A non-empty list
    /// prevents a clean pass: requested history coverage that never ran must
    /// not be reported as "No secrets detected". `serde(default)` keeps the
    /// field backward-compatible with pre-fix wire consumers.
    #[serde(default)]
    pub history_scan_errors: Vec<String>,
    /// SDT-001/SDT-006: every reason this result cannot claim it saw all of
    /// its input — a failed history scan, oversize lines, and the blocking
    /// file-level skips below — in the order `message` renders them.
    ///
    /// Non-empty whenever the scan could not read all of its input, which is
    /// **independent of `findings`**: a result can carry real findings *and*
    /// coverage notes at the same time. It forces `passed` to false on its own
    /// when there are no findings. Do not read it as "the reason this failed
    /// instead of a finding" — read it as "what this result could not see".
    ///
    /// This is the data behind the coverage half of `message`. Consumers that
    /// build their own output need it structured rather than as prose: `anvil
    /// gate` renders findings as locations, so without this a coverage-only
    /// failure came out as "Potential secrets found in 0 location(s):" above
    /// an empty list — a red naming nothing to act on. `serde(default)` keeps
    /// the field backward-compatible.
    #[serde(default)]
    pub coverage_notes: Vec<String>,
    /// SDT-006: files at or over `MAX_FILE_SIZE`, so no byte of them was
    /// scanned. **Blocking** — the file-level twin of `lines_skipped_oversize`,
    /// and measured on the anvil repository itself this is the path that hides
    /// `pnpm-lock.yaml` from the GH #2584 URL-credential scan.
    ///
    /// Paths, not a bare count: "3 files were not scanned" is not something an
    /// operator can act on, and a path is no more sensitive than the `file` a
    /// `SecretFinding` already reports unredacted. Rendered through the same
    /// `normalise_file_path` as findings, and sorted, so the value is stable
    /// under the parallel scan. `serde(default)` for wire compatibility.
    #[serde(default)]
    pub files_skipped_oversize: Vec<String>,
    /// SDT-006: files that exist but could not be read as UTF-8 text — the
    /// `fs::read_to_string(..).ok()?` path. **Blocking**: a failed read is not
    /// an operator choice, and a credential in a file nobody opened is exactly
    /// the false-clean this module exists to remove.
    #[serde(default)]
    pub files_skipped_unreadable: Vec<String>,
    /// SDT-006: files whose scan panicked and was contained by the SCAN-001
    /// `catch_unwind`. **Blocking, and the sharpest of the four**: containing
    /// the panic is correct, but before SDT-006 the whole file's scan was
    /// discarded and the result still read "No secrets detected", passed,
    /// score 100.
    #[serde(default)]
    pub files_skipped_panicked: Vec<String>,
    /// SDT-006: how many files a configured `skip_extensions` entry excluded.
    /// **Never blocking** — a deliberate operator exclusion, not a failure.
    /// The default list carries `.png`, `.jpg` and `.lock`, which every
    /// repository has, so blocking here would redden every clean pass
    /// everywhere (operator decision, 2026-08-28).
    ///
    /// A count rather than paths, and deliberately absent from `message`: this
    /// population is unbounded (a repo can hold thousands of images) and
    /// reciting the operator's own configuration back at them on every clean
    /// run would train them to skip the coverage clause — which is the clause
    /// the blocking notes above need them to read. Carrying it here keeps the
    /// exclusion observable without spending the message channel on it.
    #[serde(default)]
    pub files_skipped_extension: usize,
}
