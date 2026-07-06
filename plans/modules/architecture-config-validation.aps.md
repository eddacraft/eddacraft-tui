<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Architecture Config Validation

| ID   | Owner | Priority | Status |
| ------- | ----- | -------- | ------ |
| ARCHCFG | —     | high     | In Progress |

**Last reviewed:** 2026-07-06 (ARCHCFG-006 design gate resolved via ADR-102:
build `init` + `visualise` (ARCHCFG-007/010 Ready), redirect `check`/`watch`/
baseline flags to existing gate/watch/baseline machinery (ARCHCFG-008 Ready as
guide reconciliation), reject `list` as a `show` synonym, defer
`impact`/`export`/`debug` behind the ARCHCFG-015 usage gate. Same-day ADR-102
amendment: `impact` reframed as a config dry-run diff (not a graph-tools
projection), `export` collides with the existing top-level `anvil export`,
`debug` if promoted is `show --layer --files`. Only `validate` and `show`
exist in `crates/anvil-cli/src/commands/architecture.rs` today; ARCHCFG-008
reconciles the guide `docs/guides/custom-architecture-policies.md` with
reality.)

## Purpose

Ensure `.anvil/architecture.yaml` is unambiguous, consistent, and safe to apply.
This protects teams who scaffold with AI by rejecting overlapping paths,
undefined layers, and incomplete definitions before analysis runs.

## In Scope

- Semantic validation beyond schema checks
- Detection of overlapping layer paths and ambiguous glob rules
- Validation of layer and module references
- Warnings for empty or unused layers and rules
- Diagnostics mapped to configuration sections
- CLI validation command and gate preflight option

## Out of Scope

- Dependency analysis and gate evaluation
- Auto-fixing configuration issues
- Cross-repo architecture baselines

## Interfaces

**Depends on:**

<!-- Audit 2026-04-26: opa-architecture-integration and architecture-safety archived; their work landed in crates/anvil-architecture and crates/anvil-policy. -->
- `crates/anvil-architecture` — Architecture YAML schema, parser, layer definitions, and baseline
- `crates/anvil-kernel` — kernel architecture config loading (KERN-030)
- `crates/anvil-cli` — Rust CLI commands

**Exposes:**

- `ArchitectureConfigValidator` — Validation API
- `anvil architecture validate` — CLI entry point
- Diagnostic report format for CI and AI tools

## Acceptance Criteria

- [ ] Overlapping layer paths are blocked with clear diagnostics
- [ ] Duplicate layer ids or names are blocked
- [ ] Rules referencing unknown layers are blocked
- [ ] Empty layers and unused rules emit warnings
- [ ] Validation report includes section or key identifiers
- [ ] Typical config validates in < 100ms
- [ ] Gate preflight can block architecture checks when validation fails

## Work Items

### ARCHCFG-001: Semantic validation rules

- **Intent:** Define semantic rules for architecture config integrity
- **Expected Outcome:** Validator detects overlaps, duplicates, and unknowns
- **Scope:** `crates/anvil-kernel/src/policy/config.rs` (extends KERN-030 loader)
- **Non-scope:** Gate evaluation
- **Files:**
  - `crates/anvil-kernel/src/policy/config_validator.rs` (including `#[cfg(test)]` unit tests)
- **Dependencies:** —
- **Validation:** `cargo test -p eddacraft-anvil-kernel -- architecture_config_validator`
- **Confidence:** high

### ARCHCFG-002: Diagnostic mapping

- **Intent:** Surface validation errors with clear configuration locations
- **Expected Outcome:** Errors map to section keys and rule ids
- **Scope:** `crates/anvil-kernel/src/policy/config_diagnostics.rs`
- **Non-scope:** CLI presentation
- **Files:**
  - `crates/anvil-kernel/src/policy/config_diagnostics.rs` (including `#[cfg(test)]` unit tests)
- **Dependencies:** ARCHCFG-001
- **Validation:** `cargo test -p eddacraft-anvil-kernel -- architecture_config_diagnostics`
- **Confidence:** medium

### ARCHCFG-003: CLI validation command

- **Intent:** Provide a direct validation entry point for users and CI
- **Expected Outcome:** `anvil architecture validate` returns structured output
- **Scope:** `crates/anvil-cli/src/commands/architecture.rs`
- **Non-scope:** IDE integration
- **Files:**
  - `crates/anvil-cli/src/commands/architecture.rs` (validate subcommand, including colocated tests)
- **Dependencies:** ARCHCFG-001, ARCHCFG-002
- **Validation:** `cargo test -p eddacraft-anvil -- architecture_validate`
- **Confidence:** medium

### ARCHCFG-004: Gate preflight integration

- **Intent:** Prevent architecture checks from running on invalid config
- **Expected Outcome:** Gate preflight blocks with validation report
- **Scope:** `crates/anvil-cli/src/commands/gate.rs`
- **Non-scope:** Architecture analysis logic
- **Files:**
  - `crates/anvil-cli/src/commands/gate.rs` (preflight integration, including colocated tests)
- **Dependencies:** ARCHCFG-001
- **Validation:** `cargo test -p eddacraft-anvil -- architecture_config_preflight`
- **Confidence:** medium

### ARCHCFG-005: Documentation and examples

- **Intent:** Explain config validation rules and remediation steps
- **Expected Outcome:** Guide and examples for common failures
- **Scope:** `docs/guides/`
- **Non-scope:** Marketing content
- **Files:**
  - `docs/guides/architecture-config-validation.md`
- **Dependencies:** ARCHCFG-003
- **Validation:** Manual doc review
- **Confidence:** medium

### ARCHCFG-006: Design gate — resolve the missing-command surface

- **Intent:** Determine, before building anything, which of the eight
  CLI commands documented in `docs/guides/custom-architecture-policies.md`
  (`init`, `check`, `watch`, `visualise`, `list`, `impact`, `export`, `debug`)
  are genuinely correct to build, in what shape, and how each relates to
  capability that may already exist elsewhere in Anvil. The guide's frontmatter
  claims these are "Live" and reviewed against this module's CLI file; they are
  not — only `validate` and `show` exist. Ship a decision, not more drift.
- **Expected Outcome:** A decision record (an ADR if the shape is genuinely new
  surface, otherwise a documented scope update to this module) resolves each
  candidate command as build / defer / reject / redirect-to-existing-command,
  with a one-line rationale citing the specific overlap or gap found. At
  minimum it must answer:
  - **init** — no scaffold exists today (`resolve_arch_config` tells users to
    "create `.anvil/architecture.yaml` manually"); the guide's quickstart and
    every tutorial step downstream assume it exists. Highest-confidence yes;
    the open question is shape only (`--template` support, which templates).
  - **check** — the guide describes running real dependency analysis against
    the codebase, but that already exists as the `import-boundaries` gate
    check (legacy alias `architecture`, `crates/anvil-cli/src/commands/check_catalog.rs`,
    run via `anvil gate --only-checks architecture`). This module's own Out of
    Scope excludes "Dependency analysis and gate evaluation." Decide: redirect
    the guide to the existing gate check, or build `check` as a thin
    convenience wrapper over it — do not build a second analysis engine.
  - **watch** — `anvil watch` already exists as a general daemon (DSV
    modules). Decide whether `architecture watch` is a new standalone loop or
    an architecture-scoped view/filter registered with the existing daemon.
  - **visualise** / **export** — `anvil architecture show --json` already
    returns the full structured definition (layers, patterns, depends_on,
    rules count). Decide whether these are genuinely new data paths or just
    new renderers (mermaid, markdown) over `show`'s existing output.
  - **list** — overlaps with the existing `show` command; decide whether it is
    a distinct view (e.g. layer names only, or violations) or the guide
    inventing a synonym that should be corrected to `show`.
  - **impact** — Anvil already has graph-based impact analysis
    (`anvil_impact_of_change`, `anvil_find_dependents`). Decide whether
    `architecture impact --rule` is a thin, architecture-scoped wrapper over
    that existing capability rather than a new engine.
  - **debug** — no existing equivalent found; the least-justified candidate.
    Requires the clearest standalone case before it proceeds past Draft.
  - **check --fix / --baseline-all** — baseline-accepting flags align with
    Anvil's own "new edges only — baseline existing state, warn on new
    violations" principle (`.claude/rules/architecture.md`), so this is
    plausibly legitimate despite this module's "Auto-fixing configuration
    issues" Out of Scope line (that line is about auto-fixing the config
    file's own structure, a different concern). Decide where a baseline
    mechanism should live: `validate`, a new `check`, or `gate`.
- **Scope:** Decision-making and scope resolution only
- **Non-scope:** Implementation of any candidate command
- **Dependencies:** —
- **Validation:** Decision record exists (ADR or module-scope update);
  `pnpm aps:active-lint` passes with ARCHCFG-007..014 statuses reflecting the
  verdict (Draft retained where deferred/rejected, Ready where the gate
  approves proceeding)
- **Confidence:** high
- **Status:** Complete: 2026-07-06
- **Resolution:** ADR-102 (`plans/decisions/102-architecture-cli-surface.md`).
  Verdicts: build `init` (007) and `visualise` (010); redirect `check` →
  `anvil gate --only-checks architecture`, `watch` → `anvil watch --action
  none|gate`, baseline flags → `anvil baseline`/ADR-039; reject `list`
  (synonym of `show`); defer `impact`, `export`, `debug` behind the
  ARCHCFG-015 usage gate (added by the 2026-07-06 ADR-102 amendment).
  ARCHCFG-008 rescoped to guide reconciliation and carries all guide
  corrections (check/watch/list/quickstart/freshness claim).

### ARCHCFG-007: `anvil architecture init`

- **Intent:** Scaffold `.anvil/architecture.yaml` so a project can adopt
  architecture validation without hand-writing the schema from scratch
- **Expected Outcome:** `anvil architecture init` (optionally `--template
  <name>`) writes a valid starter `architecture.yaml` that passes
  `anvil architecture validate` unmodified
- **Scope:** `crates/anvil-cli/src/commands/architecture.rs`
- **Non-scope:** Template authoring beyond one or two starter shapes; auto-detecting an existing project's real layers; an interactive wizard (dropped per ADR-102 — the scaffold is non-interactive)
- **Dependencies:** ARCHCFG-006
- **Validation:** `cargo test -p eddacraft-anvil -- architecture_init`
- **Confidence:** high
- **Status:** Ready
- **Gate verdict (ADR-102):** Build. Non-interactive scaffold; optional
  `--template <layered|hexagonal>`, default `layered`; output must pass
  `anvil architecture validate` unmodified.

### ARCHCFG-008: Guide reconciliation — redirect `check`, `watch`, `list`

- **Intent:** Reconcile `docs/guides/custom-architecture-policies.md` with the
  ADR-102 verdicts so the guide documents only shipped commands
- **Expected Outcome:** The guide's quickstart, CLI Commands section, and
  troubleshooting are corrected: `check` → `anvil gate --only-checks
  architecture`; `watch` → `anvil watch --action none|gate`; `list` → `anvil
  architecture show`; `check --fix`/`--baseline-all` → the existing baseline
  machinery (`anvil baseline`, ADR-039); commands still deferred by ADR-102
  (`impact`, `export`, `debug`) are removed or marked planned; the frontmatter
  freshness claim reflects the actual review date. No new command ships — a
  wrapper was rejected as a second entry point whose semantics would have to
  track gate forever
- **Scope:** `docs/guides/custom-architecture-policies.md`
- **Non-scope:** New dependency-analysis logic; any change under `crates/`
- **Dependencies:** ARCHCFG-006
- **Validation:** Manual doc review — every command in the guide exists in
  `anvil --help` output (or is explicitly marked planned)
- **Confidence:** high
- **Status:** Ready
- **Gate verdict (ADR-102):** Redirect. Rescoped from "build or redirect
  `architecture check`" to the docs-only reconciliation carrying all
  redirect/reject corrections.

### ARCHCFG-009: `anvil architecture watch`

- **Intent:** Surface architecture violations live as code changes, per the
  ARCHCFG-006 verdict on standalone-loop vs existing-daemon integration
- **Expected Outcome:** Architecture violations appear in the chosen watch
  surface without a second independent file-watching loop
- **Scope:** `crates/anvil-cli/src/commands/architecture.rs`, or the existing
  watch daemon integration point per the ARCHCFG-006 decision
- **Non-scope:** A new file-watching mechanism if the existing daemon can host this
- **Dependencies:** ARCHCFG-006, ARCHCFG-008
- **Validation:** `cargo test -p eddacraft-anvil -- architecture_watch`
- **Confidence:** low
- **Status:** Draft
- **Gate verdict (ADR-102):** Redirect — no new command. `anvil watch --action
  none` already is the architecture/dependency-only watch and `--action gate`
  includes the import-boundaries check; the guide correction lands via
  ARCHCFG-008. Re-open only with an ADR-102 amendment and demand evidence.

### ARCHCFG-010: `anvil architecture visualise`

- **Intent:** Render the architecture definition as a diagram (starting with
  Mermaid, per the guide)
- **Expected Outcome:** `anvil architecture visualise --format mermaid`
  produces a diagram from the same definition `show` already parses
- **Scope:** `crates/anvil-cli/src/commands/architecture.rs`
- **Non-scope:** A general-purpose diagramming engine; formats beyond Mermaid
  in the first pass
- **Dependencies:** ARCHCFG-006
- **Validation:** `cargo test -p eddacraft-anvil -- architecture_visualise`
- **Confidence:** medium
- **Status:** Ready
- **Gate verdict (ADR-102):** Build. A renderer over the definition `show`
  already parses — a new renderer, not a new data path. `--format mermaid`
  only in the first pass; follow ADR-056 `--format` conventions.

### ARCHCFG-011: `anvil architecture list`

- **Intent:** Resolve whether `list` is a distinct view or a guide-invented
  synonym for `show`, per the ARCHCFG-006 verdict
- **Expected Outcome:** Either the guide is corrected to use `show`, or `list`
  ships as a genuinely distinct, narrower view (e.g. layer names only)
- **Scope:** `crates/anvil-cli/src/commands/architecture.rs`,
  `docs/guides/custom-architecture-policies.md`
- **Non-scope:** Duplicating `show`'s full output under a new name
- **Dependencies:** ARCHCFG-006
- **Validation:** `cargo test -p eddacraft-anvil -- architecture_list`
- **Confidence:** low
- **Status:** Draft
- **Gate verdict (ADR-102):** Reject — guide-invented synonym. `show` already
  lists layers, patterns, dependencies, and rule count; the guide correction
  lands via ARCHCFG-008. Re-open only with an ADR-102 amendment.

### ARCHCFG-012: `anvil architecture impact`

- **Intent:** Preview which existing imports a proposed architecture config
  change would newly violate (or newly resolve), as a dry-run diff of the
  import-boundaries analysis
- **Expected Outcome:** `anvil architecture impact --file <proposed.yaml>`
  runs the import-boundaries check under the current and the proposed config
  and reports the violation-set diff
- **Scope:** `crates/anvil-cli/src/commands/architecture.rs`
- **Non-scope:** A new analysis engine; a `--rule "<string>"` DSL (dropped per
  the ADR-102 2026-07-06 amendment); the graph impact tools
  (`anvil_impact_of_change` answers the inverse question — changed paths →
  dependents)
- **Dependencies:** ARCHCFG-006, ARCHCFG-015
- **Validation:** `cargo test -p eddacraft-anvil -- architecture_impact`
- **Confidence:** low
- **Status:** Draft
- **Gate verdict (ADR-102, amended 2026-07-06):** Defer behind the
  ARCHCFG-015 usage gate. Reframed from graph-tools projection to config
  dry-run diff; needs the import-boundaries check to accept an injected
  config, and its consumer persona cannot exist before `init` ships.

### ARCHCFG-013: `anvil architecture export`

- **Intent:** Export the architecture definition and/or a validation report
  in a shareable format (starting with Markdown, per the guide)
- **Expected Outcome:** `anvil architecture export --format markdown`
  produces a document from the same definition `show --json` already exposes
- **Scope:** `crates/anvil-cli/src/commands/architecture.rs`
- **Non-scope:** Formats beyond Markdown in the first pass
- **Dependencies:** ARCHCFG-006, ARCHCFG-010, ARCHCFG-015
- **Validation:** `cargo test -p eddacraft-anvil -- architecture_export`
- **Confidence:** medium
- **Status:** Draft
- **Gate verdict (ADR-102, amended 2026-07-06):** Defer behind the
  ARCHCFG-015 usage gate. `show --json` already serves machine consumers, and
  a substantial top-level `anvil export` already exists
  (`crates/anvil-cli/src/commands/export.rs` — plan conversion + constraint
  export), so the open question is surface ownership: extend `anvil export`,
  ship a namespace-local `architecture export`, or fold into
  `visualise --format markdown`. Decide after ARCHCFG-010 ships the renderer
  plumbing the latter two options would share.

### ARCHCFG-014: `anvil architecture debug`

- **Intent:** Explain, for one layer, why it does or does not pass validation
  or the import-boundaries check
- **Expected Outcome:** `anvil architecture debug --layer <name>` reproduces
  the reasoning behind that layer's current pass/fail state
- **Scope:** `crates/anvil-cli/src/commands/architecture.rs`
- **Non-scope:** A general debugger; interactive/REPL tooling; a standalone
  `debug` subcommand (per the ADR-102 2026-07-06 amendment the shape, if
  promoted, is a flag on an existing command)
- **Dependencies:** ARCHCFG-006, ARCHCFG-008, ARCHCFG-015
- **Validation:** `cargo test -p eddacraft-anvil -- architecture_debug`
- **Confidence:** low
- **Status:** Draft
- **Gate verdict (ADR-102, amended 2026-07-06):** Defer behind the
  ARCHCFG-015 usage gate. The underlying need is real — nothing today shows
  which files a layer's globs actually capture, the first confusion once
  `init` ships — but it is ~a flag's worth of behaviour:
  `show --layer <name> --files` (preferred) or `validate --explain`, never a
  subcommand.

### ARCHCFG-015: Usage gate — re-verdict the deferred architecture commands

- **Intent:** Re-evaluate the ADR-102 deferred commands (`impact`, `export`,
  `debug`) — and sweep docs for any other documented-but-missing architecture
  surface — against real usage signal, so deferrals get a dated verdict
  instead of rotting as undated Drafts
- **Expected Outcome:** A dated amendment to ADR-102 giving each deferred
  candidate a fresh build / defer / reject verdict, each citing the specific
  usage evidence (issue, support question, telemetry, explicit user request)
  or its absence; ARCHCFG-012/013/014 statuses and shapes updated to match.
  Working shapes going in: `impact` as a config dry-run diff
  (`--file <proposed.yaml>`, current-vs-proposed violation sets); `export`
  resolving ownership against the existing top-level `anvil export` (extend
  it, namespace-local, or `visualise --format markdown`); `debug` as
  `show --layer <name> --files` rather than a subcommand
- **Trigger (usage gate, not time-based):** Open only when ARCHCFG-007
  (`init`) and ARCHCFG-010 (`visualise`) have shipped in a release **and** at
  least one concrete usage signal names a deferred capability; until then this
  item stays Draft by design
- **Scope:** Decision-making only — ADR-102 amendment plus this module's item
  statuses
- **Non-scope:** Implementation of any candidate command
- **Dependencies:** ARCHCFG-007, ARCHCFG-010
- **Validation:** ADR-102 carries the dated amendment;
  `pnpm aps:active-lint` passes with ARCHCFG-012..014 statuses reflecting the
  new verdicts
- **Confidence:** high
- **Status:** Draft
