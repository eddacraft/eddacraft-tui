# Intercept Rules

| ID   | Owner  | Status      | Progress |
| ---- | ------ | ----------- | -------- |
| INTR | @aneki | In Progress | 5/8      |

**Last reviewed:** 2026-05-28 — INTR-003 (antipattern wrapper), INTR-005
(regex-content rule), and INTR-007 (rule configuration) fleshed to **Ready**:
each now carries scope/files grounded in `crates/anvil-intercept-rules/` and the
existing `anvil-checks` / `anvil-config` APIs, dependencies, and a concrete unit
validation mirroring the shipped INTR-002 / INTR-004 / INTR-008 wrappers.
INTR-007 depends on INTR-003 and INTR-005 landing first. Module remains
**In Progress** (5/8 Done). Earlier (2026-05-13, Wave 0 G5): INTR-004 path-deny
promoted **Draft → Ready** so the carry-forward gate closed before Wave 1.

> **A1 launch slice (cherry-picked, not the whole module):** INTR-001 (rule
> trait), INTR-002 (secret-detection wrapper), INTR-006 (rule registry —
> required so the eventual daemon-backed validation path can compose multiple
> rules through one pipeline), INTR-008 (launch reasoning-pattern wrapper).
> INTR-003 (antipattern wrapper), INTR-004 (path-deny), INTR-005
> (regex-content), and INTR-007 (rule configuration from `.anvil.yaml`) are
> post-A1.
>
> *(If "INTR config" was meant to refer to INTR-007 rather than INTR-006,
> flag at A1 kickoff — the launch shim runs on the embedded fallback today
> without -007, but the daemon path cannot ship multi-rule validation
> without -006.)*
>
> RMCP-005's embedded fallback currently calls `anvil-checks` rules directly.
> INTR-002 and INTR-008 now provide the equivalent daemon-path adapters for the
> **daemon-backed** path (RTAI-002 / INTD-005). INTR-006 closes the A1 slice by
> composing those rules through the registry.

## Purpose

The intercept rules module defines the rule evaluation contract and the initial
set of deterministic, cheap rules that run on the daemon hot path. It bridges
existing anvil-checks (secret detection, antipattern scanning) into the intercept
pipeline via a thin trait, and adds new path-deny and regex-content rules specific
to the intercept use case. All rules must execute in microseconds to hundreds of
microseconds -- no graph recomputation, no network calls, no expensive AST
analysis.

## In Scope

- InterceptRule trait definition (input: change batch + file content; output:
  allow | interrupt with reason)
- Wrapper rules for existing anvil-checks secret detection
- Wrapper rules for existing anvil-checks antipattern scanning
- Minimum launch reasoning-pattern rule wrapper for A1 / RMCP pre-write
  validation
- PathDenyList rule (configurable list of forbidden path patterns)
- RegexContent rule (configurable regex patterns matched against changed file
  content)
- Rule registry (ordered evaluation, short-circuit on first interrupt)
- Rule configuration loading from `.anvil.yaml` enforcement block
- Observe-only mode (rules evaluate but decisions are logged, not enforced)

## Out of Scope

- Graph-assisted checks in this module's current scope. Future boundary
  membership, symbol ownership, known-edge, or architectural-index rules must
  wait for GV2's hot-read API and stay within ADR-031 latency budgets.
- OPA policy evaluation on the hot path
- Per-rule enforcement granularity (all rules share the project enforcement mode)
- Warn or block decisions (v1 is binary: allow | interrupt)
- Rule authoring UI or wizard
- Custom rule plugin loading
- Performance benchmarking infrastructure (covered by BENCH module)

## Interfaces

- **Depends on:** anvil-checks (secret detection, antipattern scanning crates)
- **Exposes:** InterceptRule trait and rule registry for consumption by
  intercept-daemon (INTD) enforcement pipeline

## Graph v2 Coordination

INTR intentionally remains cheap and deterministic. Graph v2 does not change the
current INTR scope: no graph recompute, transitive traversal, context slicing,
or explanation work belongs in hot-path rule evaluation.

After GV2-022 exposes bounded hot reads, a later INTR slice may add graph-backed
rules for boundary membership, symbol ownership, known-edge existence, or
precomputed architectural-index checks. Those rules must consume GV2 hot indexes
only; they must not query the general graph registry or perform traversal on the
daemon hot path.

## Work Items

### INTR-001: InterceptRule Trait

- **Intent:** Define the contract that all hot-path rules implement, ensuring
  consistent input/output and composability
- **Expected Outcome:** A `crates/anvil-intercept-rules/` crate added to root
  workspace; a trait accepting a change batch reference and optional file
  content, returning an allow or interrupt decision with reason metadata; trait
  is object-safe for dynamic dispatch in the rule registry
- **Validation:** `cargo test -p eddacraft-anvil-intercept-rules --lib`
- **Status:** Done
- **Progress (2026-04-28):** Crate created at
  `crates/anvil-intercept-rules/` and added to the workspace. `InterceptRule`
  trait shipped with `rule_id`/`needs_content`/`evaluate(&RuleInput<'_>) ->
  RuleDecision`, bound `Send + Sync + 'static`, dyn-compatible
  (compile-time test asserts `Vec<Box<dyn InterceptRule>>` is constructible).
  `RuleInput` carries path + change kind + optional borrowed content so
  the daemon on-disk path and the RMCP/RTAI mid-edit path can both call
  rules without copying. `RuleDecision` is `Allow |
  Interrupt(InterruptReason{rule_id, message, line})`, serde-tagged
  `decision`; convenience constructors `RuleDecision::allow()`,
  `interrupt()`, and `interrupt_at()` cover the common cases. Unit tests
  cover dyn-dispatch round-trip, allow/interrupt/interrupt_at, RuleInput
  shape, serde shape, and `catch_unwind` panic-isolation contract for the
  registry.

### INTR-002: Secret Detection Wrapper

- **Intent:** Expose existing anvil-checks secret detection as an InterceptRule
  without duplicating the detection logic
- **Expected Outcome:** A thin adapter that calls anvil-checks secret scanning
  on changed file content and maps findings to interrupt decisions
- **Validation:** `cargo test -p eddacraft-anvil-intercept-rules --lib secret`
- **Status:** Done

### INTR-003: Antipattern Scanning Wrapper

- **Intent:** Expose existing anvil-checks antipattern scanning as an
  InterceptRule without duplicating detection logic, mirroring the INTR-002
  secret-detection and INTR-008 reasoning wrappers.
- **Expected Outcome:** A `crates/anvil-intercept-rules/src/antipattern.rs`
  adapter holds an `AntipatternCheckConfig`, declares `needs_content() == true`,
  runs antipattern detection over the changed file's borrowed content via
  `RuleInput`, and maps each finding to an interrupt decision carrying the
  antipattern id and line. Findings render through the canonical
  `Category`/`Diagnostic` envelope via a `diagnostics_with_limit` method
  consistent with the secret wrapper. `Removed` changes always `Allow`. The
  rule is registry-composable (object-safe `dyn InterceptRule`) so INTD-005 can
  short-circuit on the first interrupt.
- **Scopes:** `crates/anvil-intercept-rules/` only.
- **Non-scope:** New antipattern detections, false-positive tuning, opt-in
  pattern catalogue changes, or graph-assisted antipattern checks (GV2 boundary
  per the Graph v2 Coordination section).
- **Files:**
  - `crates/anvil-intercept-rules/src/antipattern.rs` (new)
  - `crates/anvil-intercept-rules/src/lib.rs` (module + re-export)
- **Dependencies:** INTR-001 (trait, Done), anvil-checks
  `antipattern::run_antipattern_check` (exists).
- **Confidence:** high
- **Validation:** `cargo test -p eddacraft-anvil-intercept-rules --lib antipattern`
  — unit tests cover an interrupting fixture, a clean-content allow, a
  `Removed`-change allow, the missing/binary-content allow, and
  canonical-diagnostic mapping (category, source module, line) mirroring the
  secret-wrapper test shape.
- **Status:** In Progress

### INTR-004: Path Deny List Rule

- **Intent:** Allow projects to declare file paths or glob patterns that should
  never be written by agent sessions
- **Expected Outcome:** A rule that evaluates changed file paths against a
  configurable deny list; matches produce an interrupt decision with the
  matching pattern and path
- **Validation:** `cargo test -p eddacraft-anvil-intercept-rules --lib path_deny`
- **Status:** Done
- **Progress (2026-05-13, `feat/INTR-004-path-deny`):** `PathDenyListRule`
  shipped in `crates/anvil-intercept-rules/src/path_deny.rs`. Compiles
  configured gitignore-flavoured globs (via `globset`) eagerly at
  construction so malformed patterns surface as `PathDenyError::InvalidGlob`
  rather than failing silently on the hot path. `needs_content()` is
  `false`, allowing the registry (INTR-006) to skip content reads when
  this is the only registered rule. `evaluate()` is a single
  `GlobSet::matches` call; on match it returns
  `RuleDecision::interrupt("path-deny", "Path matches deny pattern
  '<pattern>': <path>")`. `Removed` changes always `Allow` — a delete
  is not a write and the rule's intent is to prevent agent
  creation/modification of protected paths. `diagnostics_with_limit`
  emits a canonical `Category::Policy` diagnostic with no line number
  (path-only rule) and a remediation hint. Deterministic "first
  registered pattern wins" ordering keeps operator-visible output
  stable across runs. 14 unit tests pass:
  `cargo test -p eddacraft-anvil-intercept-rules --lib path_deny`
  (48 across the crate).

### INTR-005: Regex Content Rule

- **Intent:** Allow projects to declare content patterns that should trigger
  interruption when written by agent sessions, the content-matching counterpart
  to the INTR-004 path-deny rule.
- **Expected Outcome:** A `crates/anvil-intercept-rules/src/regex_content.rs`
  rule compiles a configured list of regex patterns eagerly at construction so
  malformed patterns surface as a typed `RegexContentError::InvalidPattern`
  (mirroring INTR-004's `PathDenyError::InvalidGlob`) rather than failing on the
  hot path. `needs_content() == true`. `evaluate()` matches compiled patterns
  against the borrowed `RuleInput` content and on first match returns
  `RuleDecision::interrupt_at` with the matching pattern and the 1-based line
  number. Deterministic "first registered pattern wins" ordering keeps output
  stable. `Removed` changes always `Allow`; missing/binary content always
  `Allow`. A `diagnostics_with_limit` method emits a canonical `Category::Policy`
  diagnostic with the matched line and a remediation hint.
- **Scopes:** `crates/anvil-intercept-rules/` only.
- **Non-scope:** Regex sourced from `.anvil.yaml` (that wiring is INTR-007),
  multiline/streaming matching, ReDoS-bounded execution budgets beyond compiling
  with the existing `regex` crate's linear-time guarantees, or capture-group
  reporting.
- **Files:**
  - `crates/anvil-intercept-rules/src/regex_content.rs` (new)
  - `crates/anvil-intercept-rules/src/lib.rs` (module + re-export)
- **Dependencies:** INTR-001 (trait, Done). Construction-time pattern compilation
  follows the INTR-004 eager-compile precedent.
- **Confidence:** high
- **Validation:** `cargo test -p eddacraft-anvil-intercept-rules --lib regex_content`
  — unit tests cover invalid-pattern rejection at construction, a single-pattern
  interrupt with correct line, first-pattern-wins ordering, clean-content allow,
  `Removed`-change allow, missing/binary-content allow, and canonical-diagnostic
  line mapping.
- **Status:** In Progress

### INTR-006: Rule Registry

- **Intent:** Compose multiple rules into an ordered evaluation pipeline with
  short-circuit semantics
- **Expected Outcome:** A registry that holds registered InterceptRule
  implementations, evaluates them in order, and returns the first interrupt
  decision (or allow if all pass); supports observe-only mode where interrupt
  decisions are logged but not enforced
- **Validation:** `cargo test -p eddacraft-anvil-intercept-rules --lib registry`
- **Status:** Done
- **Progress (2026-04-29, `feat/INTR-006`):** `RuleRegistry` landed in
  `crates/anvil-intercept-rules/src/registry.rs` with `RegistryDecision`
  (Allow / Interrupt) and `RegistryMode` (Enforce / ObserveOnly).
  Enforce mode short-circuits on first Interrupt; observe-only emits
  each would-be interrupt to stderr and keeps evaluating, returning
  Allow. (`tracing` is intentionally not a dep; the eprintln calls
  are the minimum-dep fallback until a wider observability story
  picks a logger.) Per-rule `catch_unwind` maps a panicking rule to
  Allow under `panic="unwind"` builds — best-effort safety net for
  dev / debug / test. The workspace's `[profile.release]` sets
  `panic="abort"`, so release-build rule panics still terminate the
  process; the long-term answer is rules that don't panic by
  construction, per the trait contract. Rule ids are sampled once at
  registration and cached — the hot path never calls `rule_id()`
  again, and `InterruptReason.rule_id` is normalised to the cached
  id before returning so a rule that emits a mismatched id can't
  break dedup or observability. Duplicate rule_ids rejected at
  register / with_rules via `RegistryError::DuplicateRuleId`.
  `any_needs_content` lets INTD-005 skip content reads when no
  content-bearing rule is registered. 15 registry tests pass:
  `cargo test -p eddacraft-anvil-intercept-rules --lib registry`.

### INTR-007: Rule Configuration

- **Intent:** Build a populated rule registry from the `.anvil.yaml` enforcement
  block so projects can declare deny lists, regex patterns, and which built-in
  rules are enabled without code changes.
- **Expected Outcome:** A `crates/anvil-intercept-rules/src/config.rs` module
  reads the `enforcement` section discovered by `anvil_config::discover`
  (`.anvil.<ext>`), parses an intercept-rules sub-block into typed config, and
  constructs a `RuleRegistry` (INTR-006) holding the enabled rule instances:
  secret detection (default on), antipattern (INTR-003), path-deny (INTR-004),
  and regex-content (INTR-005). Missing or absent config falls back to sensible
  defaults (secret detection enabled, no custom deny lists, no custom regex).
  Malformed config returns a typed `Result::Err` rather than silently degrading
  to defaults (operator-config no-silent-defaults rule). Regex and globs are
  compiled once at construction and cached for the rule instances' lifetime, so
  the hot path never recompiles.
- **Scopes:** `crates/anvil-intercept-rules/` only.
- **Non-scope:** New `.anvil.yaml` schema keys outside the existing
  `enforcement` block, per-rule enforcement granularity (all rules share the
  project enforcement mode per Out of Scope), or a config-authoring wizard.
- **Files:**
  - `crates/anvil-intercept-rules/src/config.rs` (new)
  - `crates/anvil-intercept-rules/src/lib.rs` (module + re-export)
- **Dependencies:** INTR-003 (antipattern wrapper), INTR-005 (regex-content
  rule), INTR-006 (registry, Done), `anvil_config::discover` (exists; the
  `enforcement` block is parsed today in `anvil-config/src/rule_modes.rs`).
- **Confidence:** medium — depends on INTR-003 and INTR-005 landing first so the
  config can construct those rule instances; the parse-and-construct shape
  itself is clear.
- **Validation:** `cargo test -p eddacraft-anvil-intercept-rules --lib config`
  — unit tests cover an absent-config default registry, a populated config
  constructing deny-list + regex + antipattern rules, a malformed-config typed
  error (no silent default), and a round-trip that the constructed registry
  rejects a content payload its configured rules should interrupt.
- **Status:** In Progress

### INTR-008: Launch reasoning-pattern rule wrapper

- **Intent:** Give the A1 Rust MCP pre-write path at least one AI-output
  reasoning-pattern rule beyond secret detection, without expanding INTR into a
  full reasoning engine.
- **Expected Outcome:** A minimum-viable rule from `anvil-checks` reasoning
  patterns, such as AI-001 appeal-to-authority or unjustified precision, is
  exposed as an `InterceptRule`. The rule evaluates proposed content supplied by
  RMCP/RTAI the same way it evaluates file-backed content from INTD. Findings map
  to the canonical diagnostic envelope and obey the project enforcement mode.
- **Non-scope:** Full AI-001..AI-007 catalogue, false-positive tuning beyond the
  launch fixture, or LLM-based classification.
- **Validation:** Unit test triggers the rule on a fixture planning/code comment
  payload and asserts the RMCP/RTAI response contains a canonical diagnostic with
  no dependency on Node.js or the archived TS MCP server
  (`archive/anvil-mcp-server/`)
- **Status:** Done
