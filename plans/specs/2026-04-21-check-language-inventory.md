# Check Language Inventory

## Purpose

Discovery document for `CLAR-001` and `CLAR-002`. This captures the first
inventory pass across Anvil's quality-language surfaces so later work can
normalise terminology from evidence rather than preference.

Scope of this pass: Rust CLI/TUI/kernel crates, user docs, and APS plan text.
This includes both shipped surfaces and forward-looking plans/specs so emerging
commands, screens, routes, and quality nouns are inventoried before they ship.
This is intentionally broad and shallow first. The goal is to identify the
main noun systems, overlaps, and contradictions before deciding what the
canonical language should be.

## Questions

1. What user-visible nouns does Anvil use for quality and trust feedback?
2. Which nouns refer to product concepts versus implementation details?
3. Where do the same concepts appear under different names?
4. Which surfaces teach the product model well, and which leak internal drift?
5. Which new commands or surfaces are already planned, and what language are
   they likely to introduce?

## Initial Findings

The codebase currently uses several overlapping vocabularies for adjacent
concepts:

- `check` / `checks` for project configuration, doctor diagnostics, gate units,
  tutorial steps, and policy tests
- `gate` / `gates` for the overall quality run, gate profiles, docs access,
  and future enforcement concepts
- `scan` / `scanner` for secret and antipattern detection, discovery mode, and
  scanner implementation internals
- `architecture`, `boundary`, `layer`, and `graph` for structural analysis,
  often without a stable distinction between what the user sees and what the
  kernel computes
- `policy` / `policies` for OPA packs, builtins, architecture-adjacent rules,
  and governance features
- `warning` / `issue` / `violation` / `finding` for non-pass outcomes, with
  different nouns chosen by different surfaces
- `audit`, `doctor`, and `watch` as top-level commands that overlap with the
  same problem space but teach different mental models

The highest-friction inconsistency already confirmed in code is the split
between onboarding/config check names and gate-runner check names.

## Inventory Table

| Term / Phrase | Current Meaning | Surfaces | Classification | Notes |
| --- | --- | --- | --- | --- |
| `checks` | Enabled project checks in `.anvilrc` | `docs/public/anvil/operations/config.md`, `crates/anvil-cli/src/commands/defaults.rs` | user-facing concept | Config list uses `secret-detection`, `import-boundaries`, `antipattern-scan`, `architecture`, `policy` |
| `gate` | Aggregate quality run | `crates/anvil-cli/src/commands/gate.rs`, `welcome.rs`, TUI gate surface | user-facing concept | Also used for docs auth gate and future enforcement ideas |
| `AVAILABLE_CHECKS` | Executable gate units | `crates/anvil-cli/src/commands/gate.rs` | backend implementation | Hardcoded runner vocabulary: `lint`, `test`, `coverage`, `dependency`, `secret`, `architecture`, `policy` |
| `secret-detection` | Config/onboarding check name | `defaults.rs`, config docs | alias or candidate canonical term | More descriptive than runner term `secret` |
| `import-boundaries` | Config/onboarding check name | `defaults.rs`, config docs | alias or candidate canonical term | Appears to overlap with gate runner `architecture` |
| `antipattern-scan` | Config/onboarding check name | `defaults.rs`, config docs, welcome discovery scan | ambiguous | Present in config/tutorial language; not present in gate runner vocabulary |
| `architecture` | Architecture config validation and boundary checking | `architecture.rs`, `gate.rs`, docs | overloaded concept | Sometimes means config validity, sometimes boundary enforcement |
| `policy` | OPA policy list/explain/test/eval | `policy.rs`, `gate.rs`, docs | user-facing concept | Also absorbs architecture-like entries `ARCH-001` / `ARCH-002` in policy catalogue |
| `audit` | Repo audit for broad issues | `audit.rs`, TUI audit surface, welcome hub | user-facing concept | Uses `issue` vocabulary instead of `check` or `warning` |
| `doctor` | Environment/config diagnostics | `doctor.rs`, TUI doctor surface, welcome hub | user-facing concept | Uses `DiagnosticCheck` and `CheckStatus` |
| `warning` | Non-fatal problem / severity level | tutorial discovery, docs, plans, policy output | overloaded outcome term | Competes with `issue`, `violation`, `finding` |
| `violation` | Policy or boundary failure | policy docs, gate policy output, architecture validation | backend and user-facing outcome term | Stronger than `warning`; not consistently positioned |
| `finding` | Discovery/tutorial scan result | tutorial discovery code | user-facing concept | Tutorial uses this to explain scanning outcomes |
| `issue` | Audit output item | `audit.rs`, TUI audit surface | user-facing concept | Different outcome noun from warning/violation/finding |
| `graph` | Structural model of code and dependencies | plans, architecture docs, internal docs | product and implementation concept | Powerful differentiator, but not consistently part of onboarding |
| `boundary` / `boundaries` | Architecture constraints between layers/areas | config docs, architecture command, plans | user-facing concept | One of the clearest stable nouns in the repo |
| `watch` | File-watching mode with repeated checks | `watch.rs`, tutorial watch demo, TUI watch surface | user-facing mode | Related to gates/checks, but taught separately |
| `scan` | File/content analysis step | welcome discovery, secret scanner internals | overloaded action term | Sometimes first-run educational scan, sometimes engine behaviour |

## Surface Map

### CLI command layer

- `anvil gate`: aggregate quality execution with a hardcoded check list
- `anvil gate-config`: configuration for gate check definitions and thresholds
- `anvil doctor`: environment and setup diagnostics
- `anvil audit`: broad repository audit with issues and next steps
- `anvil architecture`: show/validate architecture definition
- `anvil policy`: list/explain/diff/validate/test policies
- `anvil watch`: rerun gate/check workflows on change
- `anvil tutorial`: interactive teaching flow with watcher-backed progress

### Onboarding and welcome

- `defaults.rs` defines onboarding check names for `.anvilrc`
- welcome flow runs discovery scans, gate, audit, doctor, tutorial, and watch
- onboarding language currently mixes checks, scans, findings, gate, and watch
  without one explicit concept hierarchy

### TUI surfaces

- gate surface models a gate result as a list of `GateCheck`
- doctor surface models setup diagnostics as `DiagnosticCheck`
- audit surface models repo concerns as `AuditIssue`
- tutorial discovery surface models scan results and findings
- watch surface models repeated gate/check runs over time

### Docs

- config docs teach `checks` in `.anvilrc`
- gate config docs teach a different check list for `.anvil/gate-config.json`
- OPA docs teach policy packs, policy tests, warning and violation outputs
- architecture docs teach layers, boundaries, and graph-adjacent concepts

### Plans and architecture writing

- plans use `graph`, `boundary`, `policy`, `warning`, `drift`, `scan`, and
  `gate` extensively, often as roadmap or architecture terms rather than user
  onboarding terms

### Forward-looking plans and specs

- planned modules and specs already introduce additional nouns and surfaces,
  including flag catalogues, dashboard gate views, policy governance packs,
  release skills, intercept enforcement, and graph-delivery concepts
- these future-facing documents are part of the inventory because language drift
  is cheaper to prevent before implementation than to rename after shipping

## Confirmed Contradictions

### 1. Config checks vs gate checks

- `.anvilrc` and onboarding default to `secret-detection` and
  `import-boundaries`
- gate execution uses `secret` and `architecture`
- docs present both lists as if they are part of one system

Evidence:

- `crates/anvil-cli/src/commands/defaults.rs`
- `crates/anvil-cli/src/commands/gate.rs`
- `docs/public/anvil/operations/config.md`

### 2. Architecture is doing two jobs linguistically

- `anvil architecture validate` checks the shape of `.anvil/architecture.yaml`
- the gate `architecture` check appears to mean boundary enforcement against
  source imports
- policy catalogue also exposes architecture rules as policy-like entries

This makes `architecture` both a config object, a gate, and a policy category.

### 3. Outcome nouns are not aligned

- tutorial discovery uses `finding`
- audit uses `issue`
- doctor uses `check` with pass/fail/warn/skip
- policy and architecture use `violation`
- broader product copy uses `warning`

These may be valid distinctions, but they are not yet explained as deliberate.

## Candidate Taxonomy Directions

These are hypotheses, not decisions yet.

- `check` may need to become the smallest user-facing executable unit across
  gate, doctor, and possibly audit
- `gate` may work best as the aggregate judgement over a selected set of checks 
//aneki note - gates have a blocking and workflow connotation... you have to go through the gate to advance
- `scan` may need to be reserved for discovery/analysis actions, not top-level
  user taxonomy
- `graph` and `boundary` likely belong in the explanatory model because they are
  core to what makes Anvil distinct
- `warning`, `violation`, `issue`, and `finding` may need an explicit hierarchy
  or a reduction to fewer outcome nouns

## Gaps To Investigate Next

1. How `.anvilrc` checks are consumed outside onboarding and docs
2. Whether `gate-config` is authoritative anywhere, or mostly a parallel model
3. Where `anvil check` still exists historically versus current Rust CLI shape
4. Which tutorial steps teach concepts versus merely demonstrate commands
5. Whether the docs tree already contains a hidden canonical explanation we can
   adopt rather than rewrite from scratch
6. What other commands, checks are already planned

## Initial Forward-Looking Inventory

This pass is intentionally selective. It captures the main emerging surfaces
that are likely to shape user-visible language if left uncoordinated.

| Emerging Surface | Source | Potential Language Risk | Notes |
| --- | --- | --- | --- |
| Flag catalogue and manifest migration | `plans/modules/feature-flag-catalogue.aps.md` | `gate`, `flag`, `catalogue`, `definition`, `surface` overlap | Uses `gate` in the feature-flag sense, not the quality-run sense |
| Welcome/onboarding and discovery scan flow | `plans/modules/restore-welcome-screen.aps.md` | mixes `scan`, `warning`, `finding`, `check`, `tutorial`, `watch demo` | Strong source for future user mental model because it is first-run UX |
| Rust CLI Tier 2 | `plans/modules/rust-cli-tier2.aps.md` | introduces `check`, `validate`, `drift`, `gate-config`, `policy-debug`, `policy-watch`, `pr-comment`, `exception` without a shared concept frame | High forward risk because it expands the command set around the same quality model |
| Rust CLI Tier 3 | `plans/modules/rust-cli-tier3.aps.md` | introduces plan/governance command language (`plan validate`, `plan load`, `plan status`) that may drift away from the core quality model or overload `status`, `validate`, and governance nouns | High forward risk because it broadens the binary's workflow vocabulary beyond quality commands |
| Dashboard gate and architecture views | `plans/index.aps.md` dashboard modules | `gate detail`, `check tree`, `dependency graph` may harden nouns in UI | Dashboard will likely become a second major teaching surface after CLI/TUI |
| Policy governance modules | `plans/index.aps.md` policy governance section | `policy`, `assertion`, `control`, `bundle`, `compliance`, `gate` overlap | Governance language may pull policy away from developer UX language |
| Intercept loop modules | `plans/index.aps.md` intercept loop section | `enforcement`, `interrupt`, `rules`, `daemon`, `violation` | Could introduce stronger blocking terminology than current warning-first model |
| Release skill redesign | `plans/specs/2026-04-20-relmgmt-agent-driven-release-design.md` | uses `gate` as preflight or publication-control term | Another overload of `gate` with workflow-approval meaning |
| Graph delivery and weave plans | `plans/index.aps.md`, `plans/specs/2026-04-17-weave-rs-standalone-design.md` | `graph`, `policy_eval`, `graph_query`, `tool` may stay internal or leak into product copy | Important because graph is part of Anvil's differentiator |

### Forward-Looking Implication

The inventory problem is not limited to today's Rust CLI. Plans are already
creating additional command and UI language around gates, graphs, policy,
warnings, and enforcement. `CLAR-003` therefore needs to define rules for both
shipped and planned surfaces, and `CLAR-005` should update any active plans
whose wording would otherwise introduce further drift.

## Recommended Next Step

Proceed to `CLAR-003` with a short design doc that defines:

- canonical concepts
- concept relationships
- approved and deprecated wording
- the onboarding learning sequence for new users

That doc should use this inventory as input, not replace it.
