# anvil-checks Pipeline — As-Built

| Type     | Authority | Owner | Status | Freshness                                                                                            |
| -------- | --------- | ----- | ------ | ---------------------------------------------------------------------------------------------------- |
| As-built | Derived   | SCAN  | Live   | Last reviewed 2026-05-07 against `v0.6.0-beta` and `crates/anvil-checks`, `crates/anvil-checks-napi` |

| Upstream                                                                          | Downstream                                                                                                                                        |
| --------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| `crates/anvil-checks`, `crates/anvil-checks-napi`, `crates/anvil-intercept-rules` | anvil check / gate / audit / watch CLI, intercept daemon scan_buffer, MCP shim anvil_validate_write, activation baseline, welcome screen analyser |

> **Status:** Live (beta) **Last reviewed:** 2026-05-07 against `v0.6.0-beta`
> slate (HEAD `97b61fd0`) **Crate / location:** `crates/anvil-checks` (+
> `crates/anvil-checks-napi`, callers in
> `crates/anvil-cli/src/commands/check.rs|gate.rs|audit.rs`,
> `crates/anvil-intercept-rules/`) **Module owner (APS):** `scan-performance`
> (SCAN-NNN parallel walk + ReDoS hardening), with check-category subsets in
> `surface-env-files.aps.md` (SURFENV), `realtime-ai-validation.aps.md`
> (AI-001), `command-safety-surfaces.aps.md` (archived; runtime owned by
> `anvil-checks::command_safety`), and `ai-guardrail-profile.aps.md` (AIGUARD).
> The pre-`0.4` Rust port lives in archived modules `anvil-rust-scanner.aps.md`,
> `anvil-scanner-parity-gaps.aps.md`, and `anvil-ts-scanner-retirement.aps.md`.
> **Used by:** `anvil check`, `anvil gate`, `anvil audit`, `anvil watch` (kernel
> watch loop dispatches changes through the rule registry), the intercept
> daemon's `scan_buffer` JSON-RPC method, the embedded validation path
> (`anvil-intercept::embedded`), the MCP shim (`anvil_validate_write` routes
> through the same rule registry), the activation orchestrator's first-run
> baseline, and the welcome-screen sample analyser.

## 1. Overview

`anvil-checks` is the registry-backed check engine that produces findings
against parsed source artefacts. The same pipeline serves `anvil check`,
`anvil gate`, `anvil audit`, the watch loop, and the MCP shim — surfaces compose
check results into different decision shapes (gate verdict, audit report,
mid-edit interrupt, baseline diff) but the rule evaluation itself is
single-source.

The crate ships four check families plus a shared filter / suppression
substrate:

- **Antipattern (`AP-*`, `DD-*`, `GS-*`, `RL-*`)** — registry-backed regex rules
  from `patterns/compiled/registry.json`
  (`crates/anvil-checks/src/antipattern/`).
- **Secret detection** — pattern + entropy + `.env` parsing
  (`crates/anvil-checks/src/secret/`).
- **Reasoning (`AI-001`)** — comment-prose appeals to authority
  (`crates/anvil-checks/src/reasoning/`).
- **Surface — env files (`SURFENV-001..004`)** — `.env`/`.envrc` parse,
  gitignore hygiene, prod-shaped values, template drift
  (`crates/anvil-checks/src/surface/env/`).
- **Command safety** — script plan analysis against rule sets
  (`crates/anvil-checks/src/command_safety/`).

It does NOT own:

- File discovery / walking — callers compose `ignore::WalkBuilder` (see
  `crates/anvil-cli/src/commands/check.rs:592-620`, `gate.rs:397-455`).
- Architecture / boundary enforcement — that lives in `anvil-architecture`
  (called from `gate.rs::run_check_architecture`, `gate.rs:885`).
- OPA/Rego policy evaluation — `anvil-policy::evaluator::Evaluator` (called from
  `gate.rs::run_check_policy`, `gate.rs:1042-1118`).

## 2. Architecture diagram

```text
                    ┌──────────────────────────────────┐
   source files ───▶│ caller-owned discovery           │
   git index   ────▶│  ignore::WalkBuilder + filter    │
   in-memory   ────▶│  (commands/check.rs, gate.rs)    │
                    └──────────────┬───────────────────┘
                                   │ Vec<&str> paths
                                   ▼
                ┌────────────────────────────────────────┐
                │ activation::language_profile           │
                │  partition_for_language_specific_checks│
                │  (returns scannable + LanguageSkipLedger)│
                └─────┬───────────────────────┬──────────┘
                      │ language-specific      │ cross-language
                      │ (.ts/.tsx/.js/...)     │ (.env, secrets)
                      ▼                        ▼
       ┌──────────────────────────────────────────────────┐
       │            anvil-checks rule families             │
       │  ┌──────────────┐  ┌──────────────┐  ┌──────────┐│
       │  │ antipattern  │  │   secret     │  │reasoning ││
       │  │ (registry-   │  │ (18 patterns │  │ (AI-001) ││
       │  │  backed)     │  │  + entropy)  │  │          ││
       │  └──────┬───────┘  └──────┬───────┘  └────┬─────┘│
       │         │ surface/env (parse + scan)      │      │
       │         │ command_safety (script plans)   │      │
       └─────────┼──────────────────────────────────┼─────┘
                 │     Warning / SecretFinding /     │
                 │     Diagnostic (anvil.diagnostic.v1)
                 ▼                                  ▼
       ┌──────────────────────┐         ┌────────────────────┐
       │ suppression filter   │         │ baseline filter    │
       │ @anvil-ignore +      │ ──────▶│ .anvil/baseline.json│
       │ eslint-disable map   │         │ "new edges only"   │
       └──────────┬───────────┘         └─────────┬──────────┘
                  │                                │
                  ▼                                ▼
         ┌────────────────────────────────────────────────┐
         │  surfaces                                       │
         │  • anvil check  (CheckOutput JSON / human)      │
         │  • anvil gate   (GateResult, AIGUARD envelope)  │
         │  • anvil audit  (AuditData TUI, JSON)           │
         │  • anvil watch  (kernel→intercept-rules)        │
         │  • MCP shim     (validate_write correlation)    │
         │  • intercept    (scan_buffer mid-edit)          │
         └─────────────────────────────────────────────────┘
```

The language-profile gate sits between file discovery and per-language check
selection. The hardcoded extension allowlist on
`AntipatternCheckConfig::default()` is the fallback for repos with no profile
(and for legacy callers that have not yet adopted
`partition_for_language_specific_checks`). Cross-language checks (secrets,
env-template) bypass the partition — they run on every candidate file.

## 3. Conceptual model

The five-layer hierarchy (graph / structure → checks → findings → gates →
surfaces) is fully described in
[`docs/architecture/quality-model.md`](./quality-model.md). This doc focuses on
what's actually implemented in `v0.6.0-beta`. Cross-link notes:

- **Check** — the smallest evaluative unit. In code, each rule family exposes a
  `run_*_check` entry point with a `*CheckResult` shape
  (`run_antipattern_check`, `run_secret_check`, `run_reasoning_check`,
  `run_surfenv_check`, `run_command_safety_check`). All four mirror a
  `passed / score / message / findings` quartet so surfaces compose them
  uniformly.
- **Finding** — the antipattern family carries `Warning`
  (`crates/anvil-checks/src/antipattern/types.rs:92-120`); the secret family
  carries `SecretFinding` (`secret/types.rs:76-84`); the reasoning family emits
  the canonical `Diagnostic`
  (`crates/anvil-kernel-types/src/diagnostics.rs:155-172`,
  `schema_version = "anvil.diagnostic.v1"`); SURFENV emits four family-specific
  finding shapes (`EnvFinding`, `GitignoreFinding`, `ProdValueFinding`,
  `DriftFinding`) that wrap or parallel `SecretFinding`.
- **Gate** — the workflow judgement is `gate.rs::GateResult`
  (`crates/anvil-cli/src/commands/gate.rs:131-138`); it aggregates `CheckResult`
  rows over the canonical names declared in `AI_GUARDRAIL_CHECKS`
  (`gate.rs:104-110`) plus lint / test / coverage / dependency.
- **Notification** — gate, check, and audit all emit
  `anvil_kernel_types::Notification` rows alongside the structured payload so
  the TUI and the JSON consumers can drive UI without parsing rule output
  (`gate.rs::notifications_for_gate_result`, `gate.rs:148-206`).

## 4. Check registry

### 4.1 Antipattern registry

The compiled `.anvil` registry at `patterns/compiled/registry.json` is the
single source of truth. Eighteen rules ship in `v0.6.0-beta` across five
families (counts confirmed at `patterns/compiled/registry.json`, loaded by
`crates/anvil-checks/src/antipattern/registry_loader.rs`):

| Family                    | IDs                                              | Default severity (sample) |
| ------------------------- | ------------------------------------------------ | ------------------------- |
| guardrail-suppression     | `AP-001`, `AP-002`, `AP-004`, `AP-005`, `GS-001` | warning / info / warning  |
| type-system-evasion       | `AP-003`                                         | warning                   |
| error-visibility          | `AP-006`, `AP-007`                               | warning / info            |
| deferred-debt             | `DD-001`..`DD-004`                               | warning / info            |
| responsibility-laundering | `RL-001`..`RL-006`                               | warning / error / info    |

Notable rules (one-line summaries; full body in the registry):

- `AP-001` — broad `// eslint-disable` (or `/* eslint-disable */`) with no rule
  list. Hand-rewritten in
  `crates/anvil-checks/src/antipattern/scanner.rs:441-454` to split the PCRE
  negative-lookahead the `regex` crate cannot compile.
- `AP-003` — explicit `: any` type annotation
  (`scanner.rs::tests::scans_default_patterns_only`, `scanner.rs:904-912`).
- `AP-006` — empty catch block (`registry.json` family `error-visibility`).
- `AP-007` — `console.*` in production code (opt-in, `registry.json`
  `opt_in: true`).
- `GS-001` — non-null assertion (`expr!`) overriding nullability; carved out in
  `0.5.1-beta` via the GS-001 guarded-Map.get/has/set defence
  (`scanner.rs:672-728`, `tests::does_not_flag_guarded_map_get_after_has_set`,
  `scanner.rs:1095-1112`).
- `RL-003` — blanket "unrelated" dismissal in PR / commit prose (severity error;
  runs on `pr-description` / `commit-message` artifacts via the registry's
  `targets:` field).

Six rules carry PCRE lookarounds the Rust `regex` crate cannot compile
(`AP-001`, `DD-001..3`, `RL-001`, `RL-005`, `GS-001`); the scanner splits each
into a base regex plus a hand-coded post-filter (`scanner.rs:441-616`). A
snapshot drift-guard (`spg003_rewrite_matches_registry_snapshot`) and the
TS-parity fixture suite pin the rewrites to the registry's canonical pattern.

Registry resolution order
(`crates/anvil-checks/src/antipattern/registry_loader.rs:147-164`):

1. Explicit `LoadRegistryOptions::registry_path` (tests).
2. `ANVIL_REGISTRY_PATH` env var.
3. Upward walk from CWD.
4. Upward walk from the executable's directory (handles installed binaries run
   outside the monorepo).

If no registry is found, the loader returns an empty catalogue plus a warning
diagnostic; callers can ask for a structured report via
`anvil_checks::antipattern::registry_compile_diagnostics()`
(`scanner.rs:651-666`). `anvil doctor` consumes this so a silent silent-drop
rule surfaces as a configuration check.

### 4.2 Secret detection

Eighteen built-in patterns (`crates/anvil-checks/src/secret/patterns.rs:12-85`)
covering API keys, JWT, AWS, RSA / PGP private-key shape, database URLs, generic
secret, credit card, GitHub, Slack, Stripe (live + test), Google API, Heroku,
SendGrid, Twilio, NPM token. Plus user-supplied custom patterns
(`SecretCheckConfig::custom_patterns`, `secret/types.rs:17`).

Entropy detection (`secret/entropy.rs`) uses Shannon entropy with threshold
`4.5` and `min_entropy_length = 16` (`secret/types.rs:48-50`). Quoted runs and
`KEY=VALUE` assignments are the only shapes considered — bare prose (Tailwind
class strings, JSDoc, `pnpm-lock.yaml` peer-dep keys) is filtered out by the
`assignment_pattern`-style char class (`secret/entropy.rs:60-72`).

Per-rule false-positive carve-outs in `secret/scanner.rs`:

- `is_credit_card_false_positive` (`secret/scanner.rs:14-29`) — rejects
  credit-card matches that are a fragment of a UUID.
- `is_generic_secret_false_positive` (`secret/scanner.rs:63-127`) — rejects RHS
  values that are TS type closures, `process.env.X` accesses, template
  substitutions, function calls, or pure identifiers (the `0.5.1-beta`
  carve-out).

Performance + DoS guards:

- 1 MiB hard skip per file (`secret/check.rs:15-16`, `MAX_FILE_SIZE`).
- 4 KiB per-line guard (`secret/types.rs:26-43`, `default_max_line_bytes`) —
  SCAN-002 ReDoS hardening; lines longer than the cap are skipped before any
  regex runs and counted in `SecretCheckResult::lines_skipped_oversize`
  (`secret/types.rs:115-117`).
- `catch_unwind` per file (`secret/check.rs:45-54`) so a custom regex panic
  cannot tear down the run.

Skip-by-extension defaults (`secret/types.rs:52-69`): `.lock`, `.min.js`,
`.min.css`, `.map`, `.svg`, `.png`, `.jpg`, `.jpeg`, `.gif`, `.ico`, `.woff`,
`.woff2`, `.ttf`, `.eot`. Custom patterns that fail to compile surface in
`SecretCheckResult::pattern_errors` rather than silently disappearing
(`secret/check.rs:23, 105`).

### 4.3 Reasoning (AI-001)

`crates/anvil-checks/src/reasoning/appeal_to_authority.rs` flags comments that
justify code with appeals to authority, social proof, or deflection. Eight
phrase patterns ship at launch (`appeal_to_authority.rs:69-86`): role-as-source
("the lead said"), "as discussed with", "the manager wants", "trust me", "just
do it", "we've always done it this way", "we've done this for years", "don't
worry about (tests | edge cases | safety)".

Scope:

- Comment regions only — `//`, `/* … */`, `#`, `<!-- … -->`. String content with
  the same prose does not match (`reasoning/appeal_to_authority.rs:11-16`).
- One emission per matching line (multiple matches collapse).
- Suppression honours `@anvil-ignore AI-001` via
  `crate::antipattern::parse_suppression`
  (`reasoning/appeal_to_authority.rs:38`).

Findings emit canonical `Diagnostic` values
(`schema_version = "anvil.diagnostic.v1"`, `Category::Reasoning`,
`Severity::Info`, `source.source_module = "anvil-checks::reasoning"`). The check
ships as info-only and the heuristics are deliberately broad (false positives
acceptable, false negatives the failure mode); precision tightens after
telemetry (`reasoning/appeal_to_authority.rs:18-23`).

Shipped in `0.5.0-beta` per `CHANGELOG.md:183-184`.

### 4.4 Surface — env files (SURFENV)

Four rules under `crates/anvil-checks/src/surface/env/` (one per SURFENV slice):

- **SURFENV-001** — secrets in parsed `.env` values
  (`surface/env/scanner.rs::scan_env_file`, `surface/env/scanner.rs:82-100`).
  Parses with the dotenv subset (`surface/env/parser.rs::parse_env`,
  `surface/env/parser.rs:60-82`), applies the secret pattern set to each value,
  skips the standalone scanner's `looks_like_code` filter because a parsed
  `.env` value position is exactly where AWS-shaped keys belong.
- **SURFENV-002** — `.gitignore` hygiene
  (`surface/env/gitignore.rs::check_gitignore_hygiene`).
- **SURFENV-003** — production-shaped values in non-prod env files
  (`surface/env/prod_value.rs::scan_prod_values`).
- **SURFENV-004** — drift between sibling template (`.env.example`,
  `.env.local.example`, `.env.template`, `.env.sample`) and concrete `.env*`
  files in the same directory (`surface/env/drift.rs::check_env_drift`, paired
  by `pair_template_with_concrete` in `surface/env/check.rs:168-203`).

Aggregator entry point: `run_surfenv_check` (`surface/env/check.rs:88-131`);
discovery is the caller's job (the aggregator works against in-memory snapshots,
git index, or real working tree).

`.env` key/value parsing shipped in `0.5.0-beta` per `CHANGELOG.md:185-186`.

### 4.5 Command safety

`crates/anvil-checks/src/command_safety/` analyses script plans extracted from
`.aps.md` fenced code blocks against a default rule set
(`default_filesystem_rules`, `default_git_rules`). Entry point:
`run_command_safety_check(&CommandSafetyCheckContext)`
(`command_safety/check.rs:1-30`). The CLI seam lives at
`crates/anvil-cli/src/commands/gate.rs:1196-1245` (`run_check_command_safety`).

### 4.6 Architecture and policy (out-of-crate)

Two further check categories surface through `anvil gate` but are NOT owned by
`anvil-checks`:

- **Architecture** — `anvil_architecture::validate_with_files_and_edges`
  (`gate.rs:885-942`). Driven by `.anvil/architecture.yaml`; absent config
  returns `passed=true` with a `"Skipping"` message under `--profile ai`'s
  strict-config gate (`gate.rs:1149-1194`).
- **Policy (OPA / Rego)** — `anvil_policy::evaluator::Evaluator`
  (`gate.rs:1042-1118`). Requires the external `opa` binary; `OpaNotAvailable`
  is treated as a host-tooling problem and does NOT elevate to a strict-mode
  block (`gate.rs:1180-1184`).

## 5. Finding model

The antipattern family's `Warning` shape
(`crates/anvil-checks/src/antipattern/types.rs:92-120`):

| Field                                                    | Type                  | Notes                                        |
| -------------------------------------------------------- | --------------------- | -------------------------------------------- |
| `id`                                                     | `String`              | Rule id (`AP-003`, `GS-001`, `RL-005`, …)    |
| `fingerprint`                                            | `Option<String>`      | `id:file:line:pattern` — baseline key        |
| `category`                                               | `WarningCategory`     | `AntiPattern` / `Boundary` / `Architecture`  |
| `severity`                                               | `WarningSeverity`     | `Error` / `Warning` / `Info`                 |
| `confidence`                                             | `Confidence`          | `High` / `Medium` / `Low`                    |
| `title`, `message`, `explanation`, `suggestion`, `nudge` | prose                 | populated from registry entry                |
| `location`                                               | `Location`            | file + 1-based line + optional column / span |
| `pattern`                                                | `Option<String>`      | producing pattern id (legacy field)          |
| `suppressed`                                             | `Option<Suppression>` | reason + author + scope when suppressed      |
| `family`                                                 | `Option<String>`      | registry family (`type-system-evasion`, …)   |
| `definition_ref`                                         | `Option<String>`      | provenance back into `.anvil` source         |
| `spectrum_position`                                      | `Option<u32>`         | family-relative ordering                     |

`WarningSummary` (`types.rs:171-178`) is the
`total / errors / warnings / info / suppressed` rollup; `WarningResult` bundles
`warnings + summary + patterns_checked` (`types.rs:181-185`). Warning
fingerprint construction is centralised in `create_warning_fingerprint`
(`types.rs:231-237`) so the baseline reader and the scanner agree on keys.

The reasoning and intercept-rules paths emit the canonical `anvil.diagnostic.v1`
`Diagnostic` (`crates/anvil-kernel-types/src/diagnostics.rs:155-172`):
`schema_version`, `id`, `severity`, `summary`, `location`, `category`,
`source: { rule_id, source_module }`, `remediation_hint`, `mode`. The same shape
backs the AI guardrail profile envelope, the RTAI / INTD telemetry mirror, and
the DRVR JSON-RPC notification — none of those modules redefines the type
(`diagnostics.rs:155-160`).

Secret findings (`secret/types.rs:76-84`) carry `file`, `line`, `finding_type`
(`Pattern` / `Entropy`), `pattern_name`, `redacted_match`, `redacted_line`. The
redacted excerpt is the only content the consumer sees; raw matches never leave
the scanner.

## 6. Suppressions

The authoritative suppression parser is `crate::antipattern::parse_suppression`
(`crates/anvil-checks/src/antipattern/scanner.rs:228-257`,
[ADR-029](../../plans/decisions/029-suppression-parser-authority.md)). Every
Track 3 surface (SURFENV, SURFSQL, SURFCI, …) reuses this parser rather than
rolling its own. The directive shape is:

```
// @anvil-ignore <RULE-ID> -- <reason>
# @anvil-ignore <RULE-ID> -- <reason>
/* @anvil-ignore <RULE-ID> -- <reason> */
<!-- @anvil-ignore <RULE-ID> -- <reason> -->
-- @anvil-ignore <RULE-ID> -- <reason>
```

The directive must appear on the line immediately above the offending finding; a
different rule id does not suppress
(`scanner.rs::tests::suppression_does_not_apply_to_different_pattern`,
`scanner.rs:967-973`). Suppressed warnings still appear in `WarningResult` with
`suppressed: Some(...)` so surfaces can render "N suppressed" counts honestly
(`types.rs::count_by_severity`, `types.rs:240-259`).

`0.5.1-beta` added `eslint-disable` honesty (`scanner.rs:259-440`,
`CHANGELOG.md:153-155`). Three forms are recognised:

- `// eslint-disable-next-line <rule>` (with optional `-- <reason>`).
- `// eslint-disable-next-line` (bare — broad opt-out across the AP / GS family;
  reasoning-rule scope is unaffected).
- `/* eslint-disable <rule> */` … `/* eslint-enable */` block form.

The TypeScript-eslint → Anvil rule mapping is hard-coded (`scanner.rs:264-280`):
`@typescript-eslint/no-explicit-any → AP-003`,
`@typescript-eslint/ban-ts-comment → AP-004 | AP-005`,
`@typescript-eslint/no-non-null-assertion → GS-001`, `no-empty → AP-006`,
`no-console → AP-007`. Bare directives suppress the whole AP family plus
`GS-001`.

SURFENV reuses the same parser (`surface/env/scanner.rs:7-11`,
`surface/env/suppression.rs::resolve_line_suppression`). `SURFENV-001` accepts
`# @anvil-ignore SURFENV-001 -- <reason>` on the line above the offending entry.

## 7. Language-profile gate (LAUNCH-016)

The canonical contract is `partition_for_language_specific_checks`
(`crates/anvil-cli/src/activation/language_profile.rs:284-320`). Returns
`(scannable: Vec<&str>, LanguageSkipLedger)` for any candidate file list. Files
whose extension belongs to an `Unsupported` registry entry are dropped from the
scannable list and tallied in the ledger keyed by language name with
`reason: "unsupported"`.

The fallback for repos without a profile is the hardcoded extension allowlist on
`AntipatternCheckConfig::default()`
(`crates/anvil-checks/src/antipattern/types.rs:197-218`):
`.ts .tsx .js .jsx .mjs .cjs .html .htm .css .scss .less`. The CLI's
`commands/check.rs::resolve_extensions` honours `--extensions` first and falls
back to that default (`crates/anvil-cli/src/commands/check.rs:629-645`).

LAUNCH-016 hand-off status (per
`crates/anvil-cli/src/activation/language_profile.rs:280-282` and
`docs/architecture/activation-as-built.md` G-01):

| Acceptance criterion                                                                                                              | Status                                                                                                                                                                                                         |
| --------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| (a) Default scans `.ts`, skips `.py` for language-specific checks                                                                 | **landed** — `AntipatternCheckConfig::default().extensions` carries the contract; activation orchestrator's `services::sample_analyser` walks through the partition helper before scanning                     |
| (b) Secret scanner runs on both supported + unsupported languages (cross-language)                                                | **landed** — secret-scan call sites do NOT route through the partition helper; gate's `run_check_secret` walks all `.env*`, `.ts`, `.js`, `.rs`, `.json`, `.yaml`, `.yml`, `.toml`, `.env` (`gate.rs:438-454`) |
| (c) Run summary records the skip with language and count                                                                          | **landed** — surfaced via `AnalysisOutcome.skipped_unsupported_languages` and the `repo_languages` array in `anvil status --verify --json`                                                                     |
| (d) Explicit `extensions:` opt-in to scan unsupported languages through `commands::check` / `commands::watch` / `commands::audit` | **hand-off** — see §12 G-01                                                                                                                                                                                    |

Cross-language checks (secrets, env-template) MUST NOT use the partition helper;
the doc-comment at `language_profile.rs:280-282` makes this explicit. The seam
is in place — downstream consumers compose the user-config decision before
invoking `partition_for_language_specific_checks`.

The post-init activation path's first-scan analyser
(`crates/anvil-cli/src/services/sample_analyser.rs:108-130`) demonstrates the
canonical adoption shape: walk → partition → scan language-specific →
still-scan-all for secrets.

## 8. Baseline + drift

`crates/anvil-cli/src/activation/baseline.rs` records the set of non-suppressed
antipattern + secret findings present on first `anvil start` and persists them
to `.anvil/baseline.json` (`baseline.rs:46-47, 192-213`). Schema is
`Baseline { schema_version, created_at, fingerprints: BTreeSet<String>, counts: BaselineCounts }`
— readers reject unknown `schema_version`s rather than silently accepting
partial data (`baseline.rs:42-44, 234-238`).

Fingerprint format (`baseline.rs:288-305`):

- Antipattern: `antipattern:{id}:{file}:{line}:{pattern}` — wraps
  `anvil_checks::antipattern::create_warning_fingerprint` with a namespace
  prefix.
- Secret: `secret:{file}:{line}:{pattern_name}` — pattern name + file + line;
  the redacted match is intentionally not part of the fingerprint because
  redaction lengths are non-deterministic.

Backslashes are normalised to forward slashes so the same finding on Windows and
POSIX produces the same fingerprint (`baseline.rs:296-303`).

The "new edges only" semantics ([ADR-003](../../plans/decisions/) framing,
restated at `baseline.rs:1-30`): a future scan diffs its findings against the
baseline; fingerprints in the baseline are legacy / inherited and are treated as
silent. Findings outside the baseline are NEW and surface.

**v1 wiring caveat.** The contract shipped (file shape + reader +
`Baseline::contains_warning` / `contains_secret` methods at
`baseline.rs:170-184`); active filtering through `anvil watch` / `anvil check`
is the follow-up work item described at `baseline.rs:18-25`. In `v0.6.0-beta`
the baseline is written on first activation and surfaced via the `baseline:`
summary line in the activation render and the `baseline_present` /
`baseline_summary` fields in `anvil status --verify --json`; subsequent scans in
`check` / `watch` do not yet filter against it.

The baseline is written-once-on-first-activation; users who want to refresh
delete `.anvil/baseline.json` and re-run `anvil start`.

## 9. CLI surfaces (the four entry points)

### 9.1 `anvil check`

Targeted source-analysis (`crates/anvil-cli/src/commands/check.rs:154-276`).
Default mode is the antipattern check;
`--artifact pr-description | commit-message | agent-output` routes to
`scan_artifact` for non-source artefacts (`check.rs:289`). File selection:
explicit paths, `--changed [--staged | --since <ref>]`, or `--all`
(`check.rs:443-621`); the JSON envelope is `CheckOutput` (version `1.0.0`,
`check.rs:106-129`) and is intentionally a different shape from the napi binding
(per-artifact `ScanResultOutput`) — see
`crates/anvil-checks-napi/src/lib.rs:7-16`.

`--severity error|warning|info` (default `error`) sets the blocking threshold
(`check.rs:649-655`). `registry_compile_diagnostics()` is read on every run and
surfaced in the JSON envelope (`check.rs:705-712`) so silent-drop rules are
observable.

### 9.2 `anvil gate`

Workflow judgement (`crates/anvil-cli/src/commands/gate.rs:1865+`). Runs the
configured check set (`run_single_check`, `gate.rs:1121-1164`): `lint`, `test`,
`antipattern-scan`, `secret`, `coverage`, `dependency`, `architecture`,
`policy`, `command-safety`. The `--profile ai` flag selects from
`AI_GUARDRAIL_CHECKS` as an **allow-list** (`secret-detection`,
`import-boundaries`, `antipattern-scan`, `policy`, `command-safety` —
`gate.rs:104-110`), pins `strict_config = true` (`gate.rs:88-90`) so missing
project config for architecture / policy / command-safety becomes a blocking
diagnostic, and pins `json_output_default = true` (`gate.rs:91-92`) so AI
consumers always parse a stable JSON envelope. The envelope is the canonical
`anvil.diagnostic.v1` shape published by AIGUARD-002.

Lint, test, coverage, and dependency are deliberately excluded from the AI
profile — they are language-toolchain concerns the host project already enforces
and would push the profile past its 5 s budget (`gate.rs:96-103`).

Shipped in `0.5.0-beta` per `CHANGELOG.md:180-182`.

### 9.3 `anvil audit`

Broad repository review (`crates/anvil-cli/src/commands/audit.rs:21-49`). Walks
the tree with `ignore::WalkBuilder` configured `standard_filters(false)` (so
`.gitignore` does NOT prune the audit — a security scan must see every file)
plus an explicit prune list `.git`, `node_modules`, `.anvil`, `target`
(`audit.rs:55-106`). Per-file scans run on the rayon pool with `catch_unwind`
panic containment and a deterministic post-sort on
`(severity, file, line, message)` (`audit.rs:131-154`).

`check_env_file` (`audit.rs:198-222`) flags any `.env` or `.env.*` file as
`IssueSeverity::High`, with one carve-out: filenames ending in `.example`,
`.sample`, `.template`, `.dist` are committed templates and are skipped
(`audit.rs:181-196`). This is the `0.5.1-beta` audit-noise fix
(`CHANGELOG.md:155-157`): the audit reports real `.env`s regardless of
directory, but no longer flags `.env.example` and friends.

`SOURCE_EXTS = ts | js | rs | py` (`audit.rs:59`); `MAX_FILE_LINES = 500`
(`audit.rs:62`). Audit's source-scan rules are coarser than the antipattern
check by design — it surfaces bare-line-count and TODO-density issues in
addition to `.env` discovery.

### 9.4 `anvil watch`

Continuous mode (`crates/anvil-cli/src/commands/watch.rs:678+`). The file-change
loop is owned by `anvil_kernel::watch::run_watch` (`watch.rs:777`); change
events flow into the kernel's filter
(`anvil_kernel::watcher::filter::FileFilter`) and out as
`anvil_kernel_types::EngineEvent`s. Watch does NOT call `anvil_checks::*`
directly — the check evaluation is deferred to the intercept daemon
(`anvil-intercept-rules`) when wired, or to a manual `anvil check` re-run when
not.

The daemon-side rule registry the watch path dispatches into is
`anvil_intercept::enforcement::default_rule_registry`
(`crates/anvil-intercept/src/enforcement.rs:96-102`): two rules in v1 —
`SecretDetectionRule` and `LaunchReasoningPatternRule` — both of which are thin
adaptors over the `anvil-checks` family entry points
(`crates/anvil-intercept-rules/src/secret.rs:1-5`,
`crates/anvil-intercept-rules/src/reasoning.rs:1-8`). The same registry serves
the daemon's `scan_buffer` JSON-RPC method and the embedded fallback path.

### 9.5 MCP shim — `anvil_validate_write`

Routes through the same rule-evaluation pipeline. The validation client
(`crates/anvil-cli/src/mcp/validation.rs::LocalDaemonValidationClient`) prefers
the daemon's `scan_buffer` and falls back to
`anvil-intercept::embedded::embedded_evaluate` on Unix when the daemon is
unavailable; on Windows the path is `cfg(unix)`-gated and returns
`DaemonValidationOutcome::Unavailable` unconditionally. The result envelope is
byte-identical between daemon and embedded (the intercept-as-built §12 doc pins
this with
`local_daemon_client_returns_scan_buffer_diagnostics_with_embedded_parity`).

For the daemon-side detail see
[`docs/architecture/intercept-as-built.md`](./intercept-as-built.md) §12.

## 10. Performance posture

The four primary callers (`check`, `gate`, `audit`, the activation sample
analyser) share one parallel walk pattern, rolled out in `0.5.0-beta`:

- File discovery via `ignore::WalkBuilder` with explicit prune lists.
- Per-file scan on the rayon thread pool
  (`per_file: Vec<_> = candidates.par_iter().filter_map(...).collect()`, see
  `secret/check.rs:32-56`, `commands/audit.rs:131-142`,
  `antipattern/check.rs:112-130`).
- `catch_unwind` panic containment per file so a single bad regex / file cannot
  tear down the run.
- Deterministic post-sort on `(severity, file, line, …)` so the parallel collect
  order does not leak thread scheduling into user-visible output
  (`audit.rs:148-154`, `antipattern/scanner.rs:852-858`).

ReDoS hardening (SCAN-002, `plans/modules/scan-performance.aps.md` §82+):

- Secret scan: 4 KiB per-line guard (`SecretCheckConfig::max_line_bytes`,
  `secret/types.rs:36-43`); oversized lines are dropped before any regex runs
  and counted in `SecretCheckResult::lines_skipped_oversize`.
- Antipattern scan: registry rules carry RE2-compatible expressions (or
  hand-coded post-filters for the six PCRE-lookaround rules); there is no PCRE
  backtracking surface.

Size guards:

- 1 MiB hard skip per file in the secret scan (`secret/check.rs:15-16`).
- 5 MiB cap on a single artefact in `anvil check --artifact`
  (`crates/anvil-cli/src/commands/check.rs:30`).
- Per-pattern regex compilation is cached behind a `LazyLock`
  (`antipattern/scanner.rs:101-103`, `secret/patterns.rs:126-142`) —
  `Send + Sync`, shared across rayon worker threads, no per-scan recompilation
  cost.

The single source of truth for the kernel-side performance budget is
[`docs/architecture/kernel-benchmarking-spec.md`](./kernel-benchmarking-spec.md).

## 11. Cross-cutting concerns

### 11.1 Determinism

Same input + same anvil version → same findings. Enforced by:

- Per-pattern regex cache (`LazyLock`) so regex compilation cannot observe
  runtime state.
- Deterministic post-sort on every parallel collect path (see §10).
- Stable fingerprint format (`§5`) so the baseline reader and the scanner agree
  on keys across runs.
- Drift-guard test on the SPG-003 PCRE rewrites
  (`antipattern/scanner.rs:497-507`) — any drift between the registry pattern
  string and the hand-coded base regex fails CI rather than producing different
  match sets at runtime.

### 11.2 Suppression honesty

- Local `eslint-disable` honoured (`0.5.1-beta`, `CHANGELOG.md:153-155`).
- Audit skip filter for env templates while still reporting real `.env`s
  (`0.5.1-beta`, `audit.rs:181-196`).
- Suppressed findings still ship in `WarningResult` so the `summary.suppressed`
  count is reported honestly (no silent drops).

### 11.3 The TypeScript scanner stack is archived

Per `CHANGELOG.md` `0.5.1-beta` entry (line 140) the Rust CLI surfaces are now
authoritative for antipattern, suppression, drift, gate, and export flows. The
TS scanner crate is archived (see
`plans/archive/modules/anvil-ts-scanner-retirement.aps.md`); the
`crates/anvil-checks-napi` binding is the remaining seam for the Node-side
integration tests, and its JSON shape is intentionally distinct from
`anvil check --json` (`crates/anvil-checks-napi/src/lib.rs:7-16`). Both shapes
derive from the same `scan_artifact` call, so warning content is parity by
construction; the envelope is different because the binding operates one
artefact at a time and exposes the richer `Warning` fields (~17) the CLI's
narrow `JsonWarning` (~9) projects through.

### 11.4 JSON envelope stability

The `anvil.diagnostic.v1` shape
(`crates/anvil-kernel-types/src/diagnostics.rs:23-25, 155-172`) backs three
outer envelopes:

- AIGUARD-002 gate result (consumed by `anvil gate --profile ai`).
- RTAI-007 / INTD-013 telemetry mirror
  (`crates/anvil-intercept/src/telemetry.rs`).
- DRVR-002 JSON-RPC notification
  (`anvil-intercept-proto/src/protocol.rs::ANVIL_PUBLISH_DIAGNOSTICS`).

None of those modules redefines the type. Adding a field is backward-compatible
(consumers tolerate unknown fields); renaming or removing a field is breaking
and requires a `schema_version` bump.

### 11.5 Provenance

Registry-sourced patterns carry `family` / `definition_ref` /
`spectrum_position` provenance fields all the way through to the emitted
`Warning` (`antipattern/scanner.rs::create_warning_from_match`,
`scanner.rs:200-225`, test pinned by
`warning_carries_family_provenance_from_pattern`, `scanner.rs:1144-1175`).
Legacy patterns without provenance keep the fields `None` rather than
synthesising fake ones.

## 12. Known gaps (dated 2026-05-07)

### G-01: LAUNCH-016 hand-off — `extensions:` user-config opt-in deferred

`partition_for_language_specific_checks` ships the contract; the `extensions:`
user-config opt-in for re-enabling unsupported-language scanning through
`commands::check`, `commands::watch`, and `commands::audit` is hand-off to a
follow-up PR. The seam is in place — downstream consumers compose the
user-config decision before invoking the partition helper. Tracked at
`plans/modules/launch-flow-readiness.aps.md` LAUNCH-016 (d). Cross-link
`docs/architecture/activation-as-built.md` §G-01.

### G-02: SQL coverage is partial

`SURFSQL` (`plans/modules/surface-sql-migrations.aps.md`) Phase 1 is not yet
shipped. SQL files (`.sql`) are classified `partial` in `LANGUAGE_REGISTRY`
(`activation-as-built.md` §"Language profile"); the secret scanner runs on
`.sql` (cross-language) but no structural governance is wired.

### G-03: Markdown coverage is partial

`MDGOV` (`plans/modules/markdown-governance.aps.md`) is not yet shipped.
Markdown (`.md .mdx`) is classified `partial` — secret checks ship; structural
governance pending.

### G-04: Python and Rust unsupported

`PYLAN` (`plans/modules/lang-python.aps.md`) and `RSTLAN`
(`plans/modules/lang-rust.aps.md`) anchors not yet shipped. `.py` and `.rs`
files are filtered out by `AntipatternCheckConfig::default().extensions`; the
language profile classifies them `unsupported`. Reasoning rule (`AI-001`) does
fire on `.rs` source comments via the registry's source-only default, but the
antipattern family does not.

### G-05: Kernel-import incremental quirks fixed in `0.5.1-beta` — known-historically-fragile

The kernel's import-graph incremental path was the source of two
silent-staleness bugs fixed at `CHANGELOG.md:144-148`. Pin tests live in
`crates/anvil-kernel/tests/`; flag this area as known-fragile if incremental
scan results disagree with full re-scan. Cross-link
[`kernel-benchmarking-spec.md`](./kernel-benchmarking-spec.md).

### G-06: OPA / Rego gate requires external tooling

The policy check shells out to the `opa` binary (`gate.rs::run_check_policy`,
`gate.rs:1042-1118`). `OpaNotAvailable` returns `passed=true` with a
`"Skipping"` message; under `--profile ai`'s `strict_config = true` this is
deliberately NOT elevated to a blocking diagnostic (`gate.rs:1180-1184`) — host
tooling availability is an environment problem, not a project posture problem.
Operators install OPA per
[`docs/guides/opa-policy-testing.md`](../guides/opa-policy-testing.md).

### G-07: Baseline filtering not yet wired into watch / check

The contract shipped (`baseline.rs:18-25`), but `anvil watch` and `anvil check`
do not yet filter their findings against the baseline. First-run findings are
recorded; subsequent scans see and report the same findings as new. The baseline
summary in `anvil status --verify --json` is honest about the recorded count;
the diff semantics ride a follow-up.

### G-08: AI-001 precision is broad-by-design

`Severity::Info`, false positives explicitly accepted
(`reasoning/appeal_to_authority.rs:18-23`). AI-002 onward will tighten once
telemetry on the AI-001 trigger rate exists. Treat AI-001 findings as a coaching
signal, not a blocking judgement.

### G-09: MCP `daemonStatus` always `not-wired` on Windows

`crates/anvil-cli/src/mcp/validation.rs::validate_pre_write` is
`#[cfg(unix)]`-gated; the `cfg(not(unix))` arm returns
`DaemonValidationOutcome::Unavailable` unconditionally. The MCP
`anvil_validate_write` correlation envelope cannot distinguish daemon-up from
daemon-down on Windows in v1. Cross-link
[`intercept-as-built.md`](./intercept-as-built.md) §16 gap 9.

## 13. Source references

`crates/anvil-checks/src/`:

| File                                             | Role                                                                                                                                    |
| ------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------- |
| `lib.rs`                                         | Module surface — re-exports `antipattern`, `command_safety`, `filter`, `reasoning`, `secret`, `surface`.                                |
| `filter.rs`                                      | `ScanFilter`, `DEFAULT_DIR_EXCLUDES`, `BUILD_ARTEFACT_DIRS`, `ALWAYS_SCAN_FILENAMES`, `is_binary_path`. The shared discovery substrate. |
| `antipattern/mod.rs`                             | Family surface — `run_antipattern_check`, `scan_artifact`, `scan_files`, `parse_suppression`, `registry_compile_diagnostics`.           |
| `antipattern/types.rs`                           | `Warning`, `WarningResult`, `WarningSummary`, `AntipatternCheckConfig`, `Suppression`, `create_warning_fingerprint`.                    |
| `antipattern/registry_loader.rs`                 | Compiled-registry decoder + path resolution + cache. The single source of truth for the AP / DD / GS / RL catalogue.                    |
| `antipattern/patterns.rs`                        | `LazyLock<Vec<AntiPattern>>` over the loaded registry.                                                                                  |
| `antipattern/scanner.rs`                         | `scan_artifact`, PCRE-rewrite shims, `@anvil-ignore` parser, eslint-disable mapping, GS-001 guarded-Map.get carve-out.                  |
| `antipattern/check.rs`                           | `run_antipattern_check` (rayon-parallelised file walk, severity scoring).                                                               |
| `secret/mod.rs`                                  | Family surface — `run_secret_check`, `scan_content`, `scan_git_history`, pattern matchers.                                              |
| `secret/types.rs`                                | `SecretFinding`, `SecretCheckConfig` (`max_line_bytes` SCAN-002 guard, skip extensions).                                                |
| `secret/patterns.rs`                             | 18 built-in patterns + default allowlist + `compile_custom_patterns`.                                                                   |
| `secret/scanner.rs`                              | Per-rule false-positive carve-outs (UUID-credit-card, generic-secret-code-shape).                                                       |
| `secret/entropy.rs`                              | Shannon entropy + quoted/assignment shape filter.                                                                                       |
| `secret/check.rs`                                | `run_secret_check` (rayon walk, 1 MiB skip, dedupe, scoring).                                                                           |
| `secret/git_scanner.rs`                          | Optional git-history scan (`config.scan_git_history`).                                                                                  |
| `reasoning/mod.rs`                               | Family surface — `run_reasoning_check`, `run_reasoning_check_with_limit`.                                                               |
| `reasoning/types.rs`                             | `ReasoningCheckConfig`, `ReasoningCheckResult`.                                                                                         |
| `reasoning/appeal_to_authority.rs`               | AI-001 — appeal-to-authority phrase patterns + comment-region scanner.                                                                  |
| `surface/env/mod.rs`                             | SURFENV family surface.                                                                                                                 |
| `surface/env/check.rs`                           | `run_surfenv_check` aggregator + template/concrete pairing.                                                                             |
| `surface/env/parser.rs`                          | `parse_env` — dotenv subset (single + double-quoted, `export`, comments).                                                               |
| `surface/env/scanner.rs`                         | SURFENV-001 — secret scan over parsed `.env` values.                                                                                    |
| `surface/env/gitignore.rs`                       | SURFENV-002 — `.gitignore` hygiene.                                                                                                     |
| `surface/env/prod_value.rs`                      | SURFENV-003 — production-shaped values in non-prod files.                                                                               |
| `surface/env/drift.rs`                           | SURFENV-004 — template ↔ concrete drift.                                                                                                |
| `surface/env/suppression.rs`                     | SURFENV suppression resolver (reuses `parse_suppression`).                                                                              |
| `command_safety/{check,matcher,parser}.rs`       | Script-plan extraction, rule matcher, parsed-command analysis.                                                                          |
| `command_safety/rules/{filesystem,git}_rules.rs` | Default rule sets shipped with the binary.                                                                                              |

Adjacent crates:

- `crates/anvil-checks-napi/src/lib.rs` — Node-API binding for the CLI
  acceleration path per ADR-030. Internal CLI seam, not published to npm.
- `crates/anvil-kernel-types/src/diagnostics.rs` — canonical `Diagnostic` +
  `DiagnosticSource` + `Location` shapes that back `anvil.diagnostic.v1`.
- `crates/anvil-intercept-rules/src/{secret,reasoning}.rs` — daemon- side
  adaptors over `anvil_checks::secret::scan_content_with_limit` and
  `anvil_checks::reasoning::run_reasoning_check_with_limit`.

CLI seams (`crates/anvil-cli/src/`):

- `commands/check.rs` — `anvil check` (antipattern + non-source artefacts).
- `commands/gate.rs` — `anvil gate` (every check category; `--profile ai`
  allow-list path).
- `commands/audit.rs` — `anvil audit` (env-file detection + source audit).
- `commands/watch.rs` — `anvil watch` (kernel watcher; dispatches through
  `anvil-intercept-rules` when the daemon is wired).
- `activation/baseline.rs` — `.anvil/baseline.json` schema + atomic writer +
  `contains_warning` / `contains_secret` contract.
- `activation/language_profile.rs` — `partition_for_language_specific_checks`,
  `LANGUAGE_REGISTRY`, `LanguageSkipLedger`.
- `services/sample_analyser.rs` — first-run scan that builds the baseline at
  activation time.
- `mcp/tools/validate_write.rs` — `anvil_validate_write` MCP tool; reuses
  `DEFAULT_COMPILED_PATTERNS` for the redaction filter (`§4.4` of the intercept
  doc).
- `mcp/validation.rs` — `LocalDaemonValidationClient` that routes validate-write
  calls between daemon-backed and embedded modes.

## 14. Related docs

- [`docs/architecture/quality-model.md`](./quality-model.md) — the conceptual
  model for `check` / `gate` / `watch` / `audit` / `doctor`. This doc describes
  the implementation; the quality model describes the intent.
- [`docs/architecture/activation-as-built.md`](./activation-as-built.md) —
  language profile, baseline write, `partition_for_language_specific_checks` use
  site.
- [`docs/architecture/intercept-as-built.md`](./intercept-as-built.md) —
  daemon-side rule-evaluation pipeline, embedded fallback, `scan_buffer`
  JSON-RPC, the AI guardrail correlation envelope.
- [`docs/architecture/kernel-benchmarking-spec.md`](./kernel-benchmarking-spec.md)
  — performance budget that the parallel-walk pattern is sized against.
- [`docs/public/anvil/concepts/gates.md`](../public/anvil/concepts/gates.md) —
  the public-side "gates" concept doc that this implementation backs up.
- [`plans/decisions/029-suppression-parser-authority.md`](../../plans/decisions/029-suppression-parser-authority.md)
  — ADR-029 establishing `crate::antipattern::parse_suppression` as the
  authoritative directive parser for every Track 3 surface.
- [`RELEASE-PLAN.md`](../../RELEASE-PLAN.md) — current release slate; per-tag
  check shipping history (`0.5.0-beta` AI-001 + `--profile ai` + `.env` parsing,
  `0.5.1-beta` secret-FP fixes + eslint-disable + GS-001 guarded-Map.get + audit
  env-template filtering).
- [`CHANGELOG.md`](../../CHANGELOG.md) — customer-facing checks summary across
  releases.
- [`plans/modules/scan-performance.aps.md`](../../plans/modules/scan-performance.aps.md)
  — SCAN-001 parallel walk + SCAN-002 ReDoS hardening.
- [`plans/modules/surface-env-files.aps.md`](../../plans/modules/surface-env-files.aps.md)
  — SURFENV-001..004 plan + acceptance.
- [`plans/modules/realtime-ai-validation.aps.md`](../../plans/modules/realtime-ai-validation.aps.md)
  — AI-001 family open question 3 (rule lives in `anvil-checks`, not in a
  standalone reasoning crate).
- [`plans/archive/modules/ai-guardrail-profile.aps.md`](../../plans/archive/modules/ai-guardrail-profile.aps.md)
  — AIGUARD-001 (`--profile ai` profile), AIGUARD-002 (canonical JSON envelope),
  AIGUARD-003 (end-to-end wiring).
- [`plans/archive/modules/anvil-rust-scanner.aps.md`](../../plans/archive/modules/anvil-rust-scanner.aps.md)
  — historical context: the original Rust port that landed the registry-backed
  catalogue. Archived; the Rust scanner is now authoritative.
- [`plans/archive/modules/anvil-ts-scanner-retirement.aps.md`](../../plans/archive/modules/anvil-ts-scanner-retirement.aps.md)
  — TS-scanner cascade retirement (completed in `0.5.1-beta`).
