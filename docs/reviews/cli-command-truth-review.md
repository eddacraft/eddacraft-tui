# CLI Command Truth Review

| Type  | Authority | Owner | Status | Freshness                                                                    |
| ----- | --------- | ----- | ------ | ---------------------------------------------------------------------------- |
| Guide | Advisory  | CLICT | Live   | Last updated 2026-07-07 — slices 4–6 reconciled; runtime registry re-checked |

| Upstream                                                                    | Downstream                                      |
| --------------------------------------------------------------------------- | ----------------------------------------------- |
| `crates/anvil-cli/src/commands/`, `docs/runbooks/cli-surface.md`, APS CLICT | Doc reconciliation, APS work items, build gates |

**Status:** Work in progress. This file is the running audit log for documented
CLI commands versus runtime behaviour. Fan-out from here command-family by
command-family.

**APS module:**
[`plans/modules/cli-command-truth.aps.md`](../../plans/modules/cli-command-truth.aps.md)

---

## Operating model

This review is a **living audit log**, not a one-off report. Each top-level CLI
command family (or tightly coupled family group) gets the same treatment
architecture received in slice 1.

### Per-family loop

| Step                | Owner                | Output                                                                                             |
| ------------------- | -------------------- | -------------------------------------------------------------------------------------------------- |
| 1. Audit            | Review doc           | New **slice** — doc inventory, runtime map, substitutes, tests, drift hotspots                     |
| 2. Plan             | CLICT APS            | New **CLICT-00N** work item (Ready) scoped to docs reconciliation only                             |
| 3. Reconcile        | CLICT-00N            | Docs aligned to `anvil <cmd> --help` _today_; redirects for behaviour that already ships elsewhere |
| 4. Build (optional) | Vertical APS         | Design gate + implementation (e.g. ARCHCFG-006..014 for architecture)                              |
| 5. Re-audit         | Review slice + CLICT | Refresh after each merge that adds or changes subcommands                                          |

### Division of labour

- **CLICT** — what documentation _claims_ versus what the CLI _registers and
  dispatches today_. Reconciliation work items correct guides, public docs, APS
  false-complete rows, and CHANGELOG drift.
- **Vertical modules** — what to _build_ and how (design gates, implementation,
  engine wiring). CLICT does not own code changes.

Reconciliation is **phase 1 of N** for any family where builds are planned: docs
fixed now stop agent/user pain; a follow-up CLICT pass (or slice refresh) is
expected after vertical work lands.

### Slice template

Each slice should include:

1. Documentation layers table (conflicting sources)
2. Command-by-command map (documented → code → engine → tests)
3. Wiring gaps and substitute surfaces
4. Test coverage gaps
5. Documentation drift hotspots
6. APS cross-links (CLICT reconciliation item + vertical build items)
7. Reconciliation checklist (open until CLICT-00N closes)

---

## Problem

Documentation, APS completion records, and CHANGELOG entries sometimes claim CLI
commands that are not registered in `crates/anvil-cli`, or describe behaviour
the implementation does not provide. Agents and users treat guides as executable
truth; drift causes failed runs, false confidence, and wasted implementation
planning.

**First case (2026-07-06):** `anvil architecture` — the guide documents ten
command surfaces; the CLI exposes two subcommands; neither subcommand calls the
real `anvil_architecture` enforcement engine.

---

## Method

For each command family:

1. **Inventory docs** — guides, runbooks, public tutorials, APS completed
   records, CHANGELOG claims.
2. **Inventory runtime** — `anvil <cmd> --help`,
   `crates/anvil-cli/src/commands/`, `main.rs` dispatch, feature-flag/auth
   gates.
3. **Map substitutes** — top-level commands or gate checks that already provide
   the described behaviour under a different name.
4. **Map tests** — unit, integration, E2E, crate-level.
5. **Record verdict** — exists / stub / redirect / missing / falsely marked
   complete.
6. **Open APS item** — docs reconciliation before build decisions where scope is
   unclear.

---

## Command families (runtime truth)

**Source of truth:** `cargo run --bin anvil -- --help` and per-command `--help`,
root registry re-verified 2026-07-07 on `main`. Registration lives in
`crates/anvil-cli/src/main.rs` (`Commands` enum); dispatch in
`crates/anvil-cli/src/commands/`.

**Count:** **45** top-level command families (plus built-in `help`). **20**
expose subcommands; **25** are flags/positional-only surfaces. Hidden
compatibility aliases (`login`, `logout`, `whoami`) dispatch to `auth` and are
tracked as auth-surface notes, not separate command families.

### Registry

| #   | Command        | Shape       | Runtime surface (2026-07-06)                                                                                                            | Runbook section | CLICT slice |
| --- | -------------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------- | --------------- | ----------- |
| 1   | `audit`        | Flat        | `[--format auto\|tui\|plain\|json\|sarif] [--json] [--no-tui] [-v]` — full-project audit                                                | Yes             | Tier 2      |
| 2   | `audit-chain`  | Flat        | Witness-chain walk for bypassed protection                                                                                              | Yes             | Tier 2      |
| 3   | `check`        | Flat        | `[FILES…] [--changed] [--staged] [--since <ref>] [--all]` — planless `antipattern-scan` + `secret-detection` only                       | Yes             | Tier 2      |
| 4   | `report-fp`    | Flat        | `[--list] <check-id> <file:line> [--include-snippet]`                                                                                   | Yes             | Tier 2      |
| 5   | `doctor`       | Flat        | `[--fix] [--json] [--no-tui] [-v]`                                                                                                      | Yes             | Tier 2      |
| 6   | `config`       | Subcommands | `show`, `set`, `convert`                                                                                                                | Yes             | Tier 2      |
| 7   | `drift`        | Subcommands | `snapshot`, `compare`, `report`, `list`, `migrate`                                                                                      | Yes             | **Slice 3** |
| 8   | `edda`         | Subcommands | `list`, `show`                                                                                                                          | Yes             | Tier 2      |
| 9   | `ember`        | Subcommands | `list`                                                                                                                                  | Yes             | Tier 2      |
| 10  | `exception`    | Subcommands | `grant`, `revoke`, `list`, `show`, `verify`, `migrate`                                                                                  | Yes             | Slice 2\*   |
| 11  | `status`       | Flat        | Project health snapshot (`--verify` read-only probe)                                                                                    | Yes             | Tier 2      |
| 12  | `start`        | Flat        | Activation orchestrator: `[--verify] [--watch] [--format yaml\|yml\|json\|toml] [--new-identity] [--why]`                               | Yes             | Tier 2      |
| 13  | `tutorial`     | Flat        | Interactive guided tutorial                                                                                                             | Yes             | Tier 2      |
| 14  | `welcome`      | Flat        | Welcome / quick-start screen                                                                                                            | Yes             | Tier 2      |
| 15  | `init`         | Flat        | `[--force]` — writes project config                                                                                                     | Yes             | Tier 2      |
| 16  | `insights`     | Flat        | Local-only weekly activity views                                                                                                        | Yes             | Tier 2      |
| 17  | `kindling`     | Subcommands | `usage top\|unused\|flags\|principals`                                                                                                  | Yes             | Tier 2      |
| 18  | `migrate`      | Subcommands | `format`, `schema` (bare `anvil migrate` → `format`)                                                                                    | Yes             | Tier 2      |
| 19  | `intercept`    | Subcommands | `start`, `status`, `unblock`, `stop`                                                                                                    | Partial†        | **Slice 6** |
| 20  | `workspace`    | Subcommands | `mode`, `allow`, `deny`, `register`, `unregister`, `list`, `install-hook`                                                               | Partial†        | **Slice 6** |
| 21  | `l4-validate`  | Flat        | Explicit commit-range L4 policy validation (CI lane)                                                                                    | Yes             | Tier 2      |
| 22  | `licenses`     | Flat        | Third-party licence attribution                                                                                                         | Yes             | Tier 2      |
| 23  | `mcp-config`   | Flat        | Generate editor MCP config (claude-code, cursor, windsurf, vscode)                                                                      | Yes             | Tier 2      |
| 24  | `mcp`          | Subcommands | `install`, `serve`                                                                                                                      | Yes             | Tier 2      |
| 25  | `plan`         | Subcommands | `dashboard`                                                                                                                             | Yes             | Tier 2      |
| 26  | `dashboard`    | Positional  | `[NAME]` where `NAME` ∈ `architecture`, `drift`, `suppressions` (omit → picker)                                                         | Yes             | Tier 2      |
| 27  | `new`          | Flat        | Scaffold from template                                                                                                                  | Yes             | Tier 2      |
| 28  | `wizard`       | Flat        | Guided project setup                                                                                                                    | Yes             | Tier 2      |
| 29  | `admin`        | Subcommands | `list`, `show`, `revoke`, `audit`, `send-migration`, `email-update`, `approve`, `invite`, `auth`                                        | Yes             | Tier 2      |
| 30  | `gate`         | Flat        | `[PLAN] [-p\|--profile dev\|ci\|production\|ai] [--only-checks …] [--skip-checks …] [--fail-fast] [--progress] [--format …]`            | Yes             | **Slice 5** |
| 31  | `gate-config`  | Flat        | `[-l\|--list] [-e\|--enable <check>] [-d\|--disable <check>]`                                                                           | Yes             | Slice 5\*   |
| 32  | `watch`        | Flat        | `[-f\|--file …] [-a\|--action check\|gate\|none] [--plans] [--source] [--all] [--patterns …] [--exclude …] [--debounce <ms>]`           | Yes             | **Slice 4** |
| 33  | `export`       | Flat        | `[SOURCE] [--to aps\|json\|yaml] [--format llms.txt\|mcp-resource\|prompt-fragment] [-o OUTPUT] [--compact]`                            | Yes             | Tier 2      |
| 34  | `hooks`        | Subcommands | `install`, `uninstall`, `status`                                                                                                        | Yes             | Tier 2      |
| 35  | `hook`         | Subcommands | `pre-commit`, `pre-push`, `post-commit`, `post-merge`, `post-rewrite`, `bootstrap`                                                      | Yes             | Tier 2      |
| 36  | `baseline`     | Subcommands | `verify`                                                                                                                                | Yes             | Tier 2      |
| 37  | `capsule`      | Subcommands | `create`, `verify`, `explain`, `prune`                                                                                                  | Yes             | Tier 2      |
| 38  | `architecture` | Subcommands | `validate`, `show`                                                                                                                      | Yes             | **Slice 1** |
| 39  | `auth`         | Subcommands | `login`, `logout`, `whoami`, `refresh`                                                                                                  | Yes             | Tier 2      |
| 40  | `policy`       | Subcommands | `eval`, `eval-regression`, `attack-regression`, `probe-trends`, `list`, `explain`, `diff`, `validate`, `install`, `show`, `test` (stub) | Partial‡        | **Slice 2** |
| 41  | `gctx`         | Subcommands | `egress enable\|disable\|status`                                                                                                        | Yes             | Tier 2      |
| 42  | `update`       | Flat        | `[--check] [--version <ver>] [--force]`                                                                                                 | Yes             | Tier 2      |
| 43  | `uninstall`    | Flat        | `[--global]` — project state or user state + daemon                                                                                     | Yes             | Tier 2      |
| 44  | `validate`     | Positional  | `<PLAN> [--format …] [--no-validate-hash]` — APS plan file validation                                                                   | Yes             | Tier 2      |
| 45  | `version`      | Flat        | Install-method-aware version + upgrade guidance                                                                                         | Yes             | Tier 2      |

† **Runbook partial:** `docs/runbooks/cli-surface.md` omits subcommands that
exist in `--help` (see slice 6).

‡ **Runbook partial:** policy section lists six subcommands; runtime registers
eleven (`install`, `show`, `eval-regression`, `attack-regression`,
`probe-trends` missing from runbook synopsis).

\* Bundled into the parent slice's CLICT reconciliation item (exception with
policy; `gate-config` with gate).

### Gate check names (runtime catalog)

Canonical names from `crates/anvil-cli/src/commands/check_catalog.rs`
(`CHECK_DEFINITIONS`, 2026-07-06):

| Stable ID    | Canonical name      | Aliases        | Gate-supported |
| ------------ | ------------------- | -------------- | -------------- |
| ANV-CORE-001 | `secret-detection`  | `secret`       | Yes            |
| ANV-CORE-002 | `import-boundaries` | `architecture` | Yes            |
| ANV-CORE-003 | `antipattern-scan`  | —              | Yes            |
| ANV-CORE-004 | `policy`            | —              | Yes            |
| ANV-CORE-005 | `lint`              | —              | Yes            |
| ANV-CORE-006 | `test`              | —              | Yes            |
| ANV-CORE-007 | `coverage`          | —              | Yes            |
| ANV-CORE-008 | `dependency`        | —              | Yes            |
| ANV-CORE-009 | `command-safety`    | —              | Yes            |

`anvil check` honours only the planless subset (`antipattern-scan`,
`secret-detection`). All gate-supported checks run under `anvil gate`.

### Priority tiers

| Tier        | Families                                                           | CLICT work     |
| ----------- | ------------------------------------------------------------------ | -------------- |
| **Tier 1**  | `architecture`, `policy`, `drift`, `watch`, `gate` (+ `exception`) | CLICT-001..005 |
| **Tier 2**  | Remaining 36 families — runbook-first spot-check                   | CLICT-007      |
| **Tier 1½** | `intercept`, `workspace` — runbook subcommands reconciled          | CLICT-006      |

---

## Slice 1: `anvil architecture` (2026-07-06)

### Documentation layers (conflicting claims)

| Source                                              | Authority             | Commands claimed                                                                                                                       |
| --------------------------------------------------- | --------------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| `docs/guides/custom-architecture-policies.md`       | Live guide (stale)    | **10** — `init`, `validate`, `check`, `watch`, `visualise`, `list`, `impact`, `export`, `debug`, plus `check --fix` / `--baseline-all` |
| `docs/runbooks/cli-surface.md`                      | Authoritative runbook | **2** — `validate`, `show`                                                                                                             |
| `docs/public/anvil/tutorials/architecture.md`       | Public tutorial       | **2** + gate substitute — `validate`, `show`, `gate --only-checks import-boundaries`                                                   |
| `docs/public/anvil/operations/config.md`            | Public ops            | `validate`, `show`                                                                                                                     |
| `plans/completed.aps.md` / `completed-index.aps.md` | APS (wrong)           | `init` (OPA-004), `visualise` (TUI-015) marked **Complete**                                                                            |
| `CHANGELOG.md`                                      | Release notes (wrong) | Claims `anvil architecture visualise` shipped                                                                                          |

**Runtime truth** (`anvil architecture --help`, 2026-07-06): only **`validate`**
and **`show`**.

### Command-by-command map

| #   | Documented command | In `anvil architecture` CLI?          | Code location                                                                                                                                                | Wired to `anvil_architecture`?                                        | Tests                              |
| --- | ------------------ | ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------- | ---------------------------------- |
| 1   | `init`             | No                                    | `crates/anvil-architecture/src/yaml_parser.rs` (`create_definition_from_template`); legacy TS templates in `packages/anvil/core/src/architecture/templates/` | Crate API only                                                        | Crate + TS template tests          |
| 2   | `validate`         | **Yes**                               | `crates/anvil-cli/src/commands/architecture.rs`                                                                                                              | **No** — local `serde_yaml::Value` parse; `depends_on` ref check only | 14 unit tests in `architecture.rs` |
| 3   | `show`             | **Yes**                               | same file                                                                                                                                                    | **No** — same shallow parser                                          | Covered in the 14 tests            |
| 4   | `check`            | No                                    | `crates/anvil-cli/src/commands/gate.rs` → `run_check_architecture()`                                                                                         | **Yes** — full boundary analysis                                      | 7+ gate tests                      |
| 5   | `watch`            | No (doc: `anvil architecture watch`)  | `crates/anvil-cli/src/commands/watch.rs`                                                                                                                     | **Yes** (kernel watcher)                                              | watch + kernel tests               |
| 6   | `visualise`        | No                                    | **None**                                                                                                                                                     | —                                                                     | **None**                           |
| 7   | `list`             | No                                    | **None** (partial overlap with `show`)                                                                                                                       | —                                                                     | **None**                           |
| 8   | `impact`           | No                                    | **None**                                                                                                                                                     | —                                                                     | **None**                           |
| 9   | `export`           | No (doc: `anvil architecture export`) | `crates/anvil-cli/src/commands/export.rs` (`anvil export`)                                                                                                   | Partial — baseline read                                               | 37 export tests                    |
| 10  | `debug`            | No                                    | **None**                                                                                                                                                     | —                                                                     | **None**                           |

Variants documented but absent: `check --fix`, `check --baseline-all`.

### Wiring gap (the “2 in code, 1 engine” finding)

Both `validate` and `show` are registered and dispatched from `main.rs`, but
neither imports `anvil_architecture`. The real enforcement path is:

```text
anvil gate --only-checks import-boundaries   (alias: architecture check in gate)
  → gate.rs::run_check_architecture
  → anvil_architecture::parse_architecture_definition
  → anvil_architecture::validate_with_files_and_edges
```

`anvil architecture validate` only checks that `depends_on` layer names exist in
the YAML — it does not run semantic validation (`validate_definition`), scan
imports, or produce violation records. Council review captured this in
`plans/reviews/COUNCIL_REVIEW.md` (architecture-health vs `architecture.rs`).

### Related surfaces (real behaviour, different command names)

| Surface                              | Role                         | Code                                 |
| ------------------------------------ | ---------------------------- | ------------------------------------ |
| `anvil gate` / `import-boundaries`   | Boundary enforcement         | `gate.rs`                            |
| `anvil watch`                        | Live boundary monitoring     | `watch.rs`                           |
| `anvil drift *`                      | Baseline drift               | `drift.rs`                           |
| `anvil dashboard architecture`       | Architecture-health TUI      | `commands/dashboard/architecture.rs` |
| `anvil export`                       | Agent context incl. baseline | `export.rs`                          |
| MCP `anvil_query_boundary`           | Pre-write import check       | `mcp/tools/query_boundary.rs`        |
| `eddacraft-anvil-architecture` crate | Core engine                  | 138 crate tests                      |

### Test coverage gaps

| Layer                            | Covers                        | Gap                                                  |
| -------------------------------- | ----------------------------- | ---------------------------------------------------- |
| `architecture.rs` (14 tests)     | Arg parsing, shallow YAML     | No `run()` E2E; no `anvil_architecture` integration  |
| `gate.rs` (7 architecture tests) | Real boundary enforcement     | No subprocess test for `anvil architecture validate` |
| E2E (`apps/e2e`)                 | Violation fixture helper only | No `anvil architecture *` suite                      |
| TCOV-004 “Complete”              | —                             | Marks stub-path tests as full command coverage       |

### Documentation drift hotspots (architecture)

1. **`docs/guides/custom-architecture-policies.md`** — highest priority;
   frontmatter claims “Live” and review against `architecture.rs` but lists
   eight commands that do not exist.
2. **`plans/completed.aps.md`** — `OPA-004` (`init`), `TUI-015` (`visualise`)
   falsely Complete.
3. **`CHANGELOG.md`** — `visualise` claim with no implementation.
4. **`plans/specs/2026-03-18-rust-cli-design.md`** — planned `watch` under
   `architecture.rs`; lives under top-level `anvil watch` instead.

Accurate today: `docs/runbooks/cli-surface.md`,
`docs/public/anvil/operations/config.md`, public architecture tutorial
(validate + show + gate redirect).

### APS cross-links (architecture)

| Item                 | Module  | Role                                                                                   |
| -------------------- | ------- | -------------------------------------------------------------------------------------- |
| **CLICT-001**        | CLICT   | Docs reconciliation — correct guides, public docs, false Complete/CHANGELOG claims     |
| **ARCHCFG-006**      | ARCHCFG | Design gate — per missing command: build / defer / reject / redirect (merged PR #3203) |
| **ARCHCFG-007..014** | ARCHCFG | Implementation candidates, each blocked on ARCHCFG-006                                 |
| **ARCHCFG-001..005** | ARCHCFG | Wire `validate` to real semantic validation + gate preflight                           |

**Sequencing:** CLICT-001 (docs truth) can land before ARCHCFG-006 (build
decisions). Docs should redirect to existing surfaces where overlap is already
known (`check` → gate, `watch` → `anvil watch`, etc.) even while ARCHCFG-006 is
open.

### PR #3203 (merged 2026-07-05)

[PR #3203](https://github.com/eddacraft/anvil-001/pull/3203) added
**ARCHCFG-006..014** to `plans/modules/architecture-config-validation.aps.md`:

- **ARCHCFG-006** — design gate with evidence-backed overlap analysis per
  candidate command (init highest-confidence yes; check/watch/visualise/export/
  list/impact/debug each mapped to existing or missing capability).
- **ARCHCFG-007..014** — one Draft work item per candidate command, all blocked
  on ARCHCFG-006.

No code or doc content changed in that PR — planning only. CLICT-001 complements
it by fixing user-facing documentation now rather than waiting for build
verdicts.

### Reconciliation checklist (architecture)

- [ ] `docs/guides/custom-architecture-policies.md` — remove or redirect eight
      absent subcommands
- [ ] `plans/completed.aps.md` / `completed-index.aps.md` — correct OPA-004,
      TUI-015 false-complete rows
- [ ] `CHANGELOG.md` — correct or supersede `visualise` claim
- [ ] Review doc slice 1 marked reconciled

---

## Slice 2: `anvil policy` + `anvil exception` (2026-07-06)

**Context:** POLRESET completed 2026-07-05 (10/10). Internal guides were
reviewed 2026-07-04; public tutorial was not. DOCSYNC-012 tracks the public
tutorial rewrite; CLICT-002 owns the broader reconciliation.

### Documentation layers (conflicting claims)

| Source                                       | Authority             | Claims vs runtime (2026-07-06)                                                                                                                   |
| -------------------------------------------- | --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| `docs/public/anvil/tutorials/policies.md`    | Public tutorial       | **Stale** — standalone `opa` binary required; loose `.rego` drop-in; `anvil policy test` as primary workflow; no `install`/`validate`/pack model |
| `docs/runbooks/cli-surface.md` §policy       | Authoritative runbook | **Partial** — six subcommands documented; missing `install`, `show`, `eval-regression`, `attack-regression`, `probe-trends`                      |
| `docs/guides/policy-validation.md`           | Authoritative guide   | **Current** — pack admission, `pack.yaml`, `anvil policy validate` (reviewed 2026-07-04)                                                         |
| `docs/guides/opa-policy-testing.md`          | Authoritative guide   | **Current** — regorus path, fixture layout, Go OPA as reference-only (reviewed 2026-07-04)                                                       |
| `docs/guides/policy-exceptions.md`           | Authoritative guide   | **Current** — `anvil exception *` contract (reviewed 2026-07-04)                                                                                 |
| `docs/public/anvil/beta-testing-guide.md`    | Public ops            | **Partial** — only `policy list` / `explain`; no POLRESET workflow                                                                               |
| `docs/public/anvil/releases/changelog.md`    | Release notes         | **Partial** — frames `anvil policy` as experimental; incomplete subcommand list                                                                  |
| `docs/archive/planning/opa-policy-engine.md` | Archived              | **Historical** — `anvil policy init`, loose-file workflow (must stay archived; agents may still cite)                                            |
| `plans/modules/documentation-sync.aps.md`    | APS                   | **DOCSYNC-012** Draft — public tutorial rewrite scoped to POLRESET pack model                                                                    |

**Runtime truth** (`anvil policy --help`): **11** subcommands — `eval`,
`eval-regression`, `attack-regression`, `probe-trends`, `list`, `explain`,
`diff`, `validate`, `install`, `show`, `test`.

### Command-by-command map (`anvil policy`)

| #   | Subcommand          | In CLI? | Code location                           | Engine / behaviour                                         | Tests                         |
| --- | ------------------- | ------- | --------------------------------------- | ---------------------------------------------------------- | ----------------------------- |
| 1   | `eval`              | Yes     | `commands/policy/eval.rs`               | Regorus via `anvil-policy-engine`                          | Unit + schema snapshot        |
| 2   | `eval-regression`   | Yes     | `commands/policy/eval_regression.rs`    | Report-only CI harness (`ci/eval/`)                        | Unit + CI                     |
| 3   | `attack-regression` | Yes     | `commands/policy/attack_regression.rs`  | Prompt-attack pack gate                                    | Unit                          |
| 4   | `probe-trends`      | Yes     | `commands/policy/adversarial_trends.rs` | ATC eval-history trends                                    | Unit                          |
| 5   | `list`              | Yes     | `commands/policy/mod.rs`                | Policy metadata listing                                    | Arg-parse tests               |
| 6   | `explain`           | Yes     | same                                    | Policy explanation                                         | Arg-parse tests               |
| 7   | `diff`              | Yes     | same                                    | Policy file diff                                           | Arg-parse tests               |
| 8   | `validate`          | Yes     | `commands/policy/validate.rs`           | Pack admission (`anvil-policy-engine/src/pack/`)           | Unit + starter_proof          |
| 9   | `install`           | Yes     | `commands/policy/install.rs`            | Bundled `anvil-baseline` starter pack                      | starter_proof + install tests |
| 10  | `show`              | Yes     | same                                    | Preview starter pack without install                       | install tests                 |
| 11  | `test`              | Yes     | `commands/policy/mod.rs`                | **Stub** — discovers `*_test.rego`, prints not implemented | Arg-parse only                |

Documented but **absent:** `anvil policy init` (archive only; use `install`).

### Related surfaces (policy ecosystem)

| Surface                           | Role                                    | Doc freshness                                 |
| --------------------------------- | --------------------------------------- | --------------------------------------------- |
| `anvil gate --only-checks policy` | Regorus gate evaluation                 | Public ops mentions; thin on packs            |
| `anvil exception grant\|…`        | Scoped exceptions before blocking modes | Guide + runbook + `operations/config` current |
| `ANVIL_POLICY_ENFORCEMENT`        | Opt-in `warn`/`fence`/`interrupt`       | Internal guides only; not public              |
| MCP `anvil_validate_write`        | Pre-write policy enforcement            | `integrations/mcp.md` — separate audit        |
| `anvil policy eval --json`        | Machine contract v1                     | `docs/specs/policy-eval-output-v1.md` current |

### Test coverage gaps (policy)

| Layer                      | Covers                        | Gap                                                |
| -------------------------- | ----------------------------- | -------------------------------------------------- |
| `policy/mod.rs` (19 tests) | Arg parsing, stub `test` path | No E2E for `install` → `gate --only-checks policy` |
| `starter_proof`            | End-to-end starter pack       | Not wired to public tutorial                       |
| E2E (`apps/e2e`)           | Gate workflow                 | No `anvil policy *` suite                          |

### Documentation drift hotspots (policy)

1. **`docs/public/anvil/tutorials/policies.md`** — highest priority; entire
   workflow is pre-POLRESET (DOCSYNC-012).
2. **`docs/runbooks/cli-surface.md` §policy** — synopsis and subcommand table
   missing five shipped commands.
3. **`docs/public/anvil/beta-testing-guide.md`** — policy section incomplete.
4. **`docs/architecture/tutorial-as-built.md`** — still centres on stub
   `anvil policy test` as tutorial truth.
5. **Archive false-complete** — `docs/archive/planning/TODO.md` marks
   `policy init` complete; redirect to `install`.

Accurate today: internal guides (`policy-validation`, `opa-policy-testing`,
`policy-exceptions`), specs (`policy-input-v1`, `policy-eval-output-v1`).

### APS cross-links (policy)

| Item            | Module  | Role                                                              |
| --------------- | ------- | ----------------------------------------------------------------- |
| **CLICT-002**   | CLICT   | Docs reconciliation — runbook, public surfaces, archive redirects |
| **DOCSYNC-012** | DOCSYNC | Public tutorial end-to-end rewrite (coordinate with CLICT-002)    |
| **POLRESET**    | (Done)  | Product truth source; no further build items in CLICT scope       |

### Reconciliation checklist (policy)

- [ ] Runbook policy section lists all 11 subcommands; notes `test` is stub
- [ ] Public tutorial rewritten around `install` → `validate` → `gate` flow
- [ ] Beta guide + changelog updated for post-POLRESET surface
- [ ] `tutorial-as-built.md` references pack workflow
- [ ] Archive/TODO false-complete rows corrected or marked superseded

---

## Slice 3: `anvil drift` (2026-07-06)

### Documentation layers

| Source                                    | Authority       | Claims vs runtime                                                        |
| ----------------------------------------- | --------------- | ------------------------------------------------------------------------ |
| `docs/runbooks/cli-surface.md` §drift     | Runbook         | **Aligned** — five subcommands match `--help`                            |
| `docs/public/anvil/tutorials/drift.md`    | Public tutorial | **Drift** — wrong snapshot filename; documents absent `--overwrite` flag |
| `docs/public/anvil/beta-testing-guide.md` | Public ops      | Mentions `drift report`, `drift migrate` — aligned                       |
| `docs/public/anvil/guides/dashboard.md`   | Public guide    | `anvil dashboard drift` — aligned                                        |

**Runtime truth:** `snapshot`, `compare`, `report`, `list`, `migrate`.

### Command-by-command map

| #   | Subcommand | In CLI? | Code                | Behaviour notes                                                 | Tests          |
| --- | ---------- | ------- | ------------------- | --------------------------------------------------------------- | -------------- |
| 1   | `snapshot` | Yes     | `commands/drift.rs` | Saves `.anvil/snapshots/snapshot-<name>.json` (prefix required) | Extensive unit |
| 2   | `compare`  | Yes     | same                | Resolves name with/without `snapshot-` prefix                   | Unit           |
| 3   | `report`   | Yes     | same                | Compares latest two valid snapshots                             | Unit           |
| 4   | `list`     | Yes     | same                | Lists snapshots in `.anvil/snapshots/`                          | Unit           |
| 5   | `migrate`  | Yes     | same                | Schema upgrade + optional `--prune-backups`                     | Unit           |

### Documentation drift hotspots (drift)

1. **Public tutorial** shows `Snapshot saved to .anvil/snapshots/baseline.json`
   — runtime writes `snapshot-baseline.json` (`SNAPSHOT_PREFIX` in `drift.rs`).
2. **Public tutorial** documents
   `anvil drift snapshot --name baseline --overwrite` — **no `--overwrite`
   flag** exists; named snapshots fail if the file already exists.
3. **Prerequisite** says `anvil check --all` — drift is architecture-graph
   based; gate/import-boundaries is the closer prerequisite (wording fix).

Runbook and beta guide are otherwise aligned.

### APS cross-links (drift)

| Item          | Module | Role                                                  |
| ------------- | ------ | ----------------------------------------------------- |
| **CLICT-003** | CLICT  | Fix public tutorial paths/flags; prerequisite wording |

### Reconciliation checklist (drift)

- [ ] `docs/public/anvil/tutorials/drift.md` — `snapshot-<name>.json` paths
- [ ] Remove `--overwrite` example; document rename/delete workflow instead
- [ ] Prerequisite references `gate` / import-boundaries where appropriate

---

## Slice 4: `anvil watch` (2026-07-06)

### Documentation layers

| Source                                             | Authority     | Claims vs runtime                                                                                   |
| -------------------------------------------------- | ------------- | --------------------------------------------------------------------------------------------------- |
| `docs/runbooks/cli-surface.md` §watch              | Runbook       | **Aligned** — flags match `--help`                                                                  |
| `docs/public/anvil/integrations/watch-output.md`   | Public spec   | **Current** — NDJSON lifecycle, daemon default, `--json` purity (DSV-era)                           |
| `docs/guides/custom-architecture-policies.md`      | Guide         | **Aligned** — substitute table points to `anvil watch` without teaching a non-registered subcommand |
| `docs/public/anvil/releases/changelog.md`          | Release notes | Documents `--action` default change — aligned with runtime                                          |
| `docs/public/anvil/guides/save-time-validation.md` | Public guide  | Thin on flags; no false commands                                                                    |

**Runtime truth:** flat command —
`[--file] [--action check|gate|none] [--plans] [--source] [--all] [--patterns] [--exclude] [--debounce]`.

### Wiring and substitutes

| Documented surface         | Runtime substitute                     | Notes                                      |
| -------------------------- | -------------------------------------- | ------------------------------------------ |
| `anvil architecture watch` | `anvil watch` (default action `check`) | Architecture boundaries via kernel watcher |
| `anvil start --watch`      | `anvil start --watch`                  | Honest fallback when MCP pre-write absent  |
| Daemon-backed save-time    | `anvil intercept start` + MCP          | Documented in `watch-output.md`            |

### Test coverage gaps (watch)

| Layer             | Covers                    | Gap                              |
| ----------------- | ------------------------- | -------------------------------- |
| `watch.rs` (many) | Arg parsing, filter logic | —                                |
| E2E save-time     | `save-time-driver.e2e`    | No full `anvil watch --json` E2E |

### Documentation drift hotspots (watch)

1. **`custom-architecture-policies.md`** — closed by CLICT-004: substitute table
   now names the capability and points to `anvil watch` without presenting a
   non-registered subcommand as executable.
2. **Default `--action` narrative** — closed by CLICT-004: runbook, public
   quickstart/ops, and save-time validation docs describe `check` as the default
   action and `--action none` as the architecture/dependency-only mode.

### APS cross-links (watch)

| Item          | Module | Role                                                          |
| ------------- | ------ | ------------------------------------------------------------- |
| **CLICT-004** | CLICT  | Architecture-watch redirects; default-action copy alignment   |
| **CLICT-001** | CLICT  | Overlap on `architecture watch` wording in architecture guide |

### Reconciliation checklist (watch)

- [x] No remaining `anvil architecture watch` in live guides
- [x] Public docs state default `--action check` and `--action none` for
      arch-only
- [x] `watch-output.md` cross-linked from public quickstart/ops where relevant

---

## Slice 5: `anvil gate` + `gate-config` + check vocabulary (2026-07-06)

### Documentation layers

| Source                                   | Authority             | Claims vs runtime                                                                |
| ---------------------------------------- | --------------------- | -------------------------------------------------------------------------------- |
| `docs/runbooks/cli-surface.md` §gate     | Runbook               | **Aligned** — flags and profiles match                                           |
| `docs/architecture/quality-model.md`     | Authoritative concept | **Aligned** — teaches `CHECK_DEFINITIONS` canonical names and accepted aliases   |
| `docs/public/anvil/concepts/gates.md`    | Public concept        | **Mostly aligned** — documents `import-boundaries` + legacy `architecture` alias |
| `docs/public/anvil/operations/config.md` | Public ops            | Check table uses canonical names — aligned                                       |
| `docs/public/anvil/concepts/sessions.md` | Public concept        | **Aligned** — sample output uses canonical check names with alias callout        |
| `crates/anvil-cli/src/commands/check.rs` | Runtime help text     | Correctly directs profile checks to `anvil gate`                                 |

**Runtime truth (`gate`):** flat — optional `[PLAN]` positional;
`--profile dev|ci|production|ai`; `--only-checks` / `--skip-checks` accept
canonical names and aliases per `check_catalog.rs`.

**Runtime truth (`gate-config`):** flat — `--list`, `--enable`, `--disable`.

### Check-name drift map

| Doc term            | Runtime canonical   | Alias accepted | Verdict  |
| ------------------- | ------------------- | -------------- | -------- |
| `architecture`      | `import-boundaries` | `architecture` | Redirect |
| `secret`            | `secret-detection`  | `secret`       | Redirect |
| `antipattern-scan`  | `antipattern-scan`  | —              | Aligned  |
| `policy`            | `policy`            | —              | Aligned  |
| `import-boundaries` | `import-boundaries` | —              | Aligned  |

### Documentation drift hotspots (gate)

1. **`quality-model.md`** — closed by CLICT-005: canonical-name table added from
   `CHECK_DEFINITIONS`, including `secret` → `secret-detection` and
   `architecture` → `import-boundaries`.
2. **`sessions.md` / sample JSON** — closed by CLICT-005: sample output and JSON
   use canonical names and note legacy aliases.
3. **`gates.md` / tutorials** — closed by CLICT-005: public examples use
   canonical check names while retaining alias explanation.
4. **Planned rename note** in runbook (`gate-config` → `anvil gate config`) —
   remains clearly future tense; no doc implies the rename exists today.

### APS cross-links (gate)

| Item          | Module | Role                                                          |
| ------------- | ------ | ------------------------------------------------------------- |
| **CLICT-005** | CLICT  | Quality-model + public copy alignment; alias callouts         |
| **CLICT-001** | CLICT  | `architecture check` → `gate --only-checks import-boundaries` |

### Reconciliation checklist (gate)

- [x] `quality-model.md` teaches canonical names with alias table
- [x] Public sample output uses `import-boundaries` (note `architecture` alias)
- [x] Runbook `gate-config` planned-rename stays clearly future tense

---

## Slice 6: `anvil intercept` + `anvil workspace` (2026-07-06)

**Tier 1½** — runbook gaps on shipped subcommands (not yet in original slice
queue).

### Runbook vs runtime gaps

| Command     | Status after CLICT-006 reconciliation                                                            |
| ----------- | ------------------------------------------------------------------------------------------------ |
| `workspace` | Runbook now lists `mode`, `allow`, `deny`, `register`, `unregister`, `list`, and `install-hook`. |
| `intercept` | Runbook now lists `start`, `status`, `unblock`, and `stop`.                                      |

`workspace` registration remains distinct from admission: `register` gives a
worktree durable daemon protection, while `allow` controls whether that root is
served in `allowlist` mode. The public operations/config guide now cross-links
the registration and persistence details for operators.

### APS cross-links

| Item          | Module | Role                                      |
| ------------- | ------ | ----------------------------------------- |
| **CLICT-006** | CLICT  | Runbook + public ops for daemon/workspace |

### Reconciliation checklist (intercept/workspace)

- [x] `docs/runbooks/cli-surface.md` §workspace documents all seven subcommands
- [x] `docs/runbooks/cli-surface.md` §intercept includes shipped `stop`
- [x] Public ops/config mentions worktree registration and durable persistence

---

## Slice queue

| #   | Command family                         | CLICT item | Status          | Notes                                                     |
| --- | -------------------------------------- | ---------- | --------------- | --------------------------------------------------------- |
| 1   | `anvil architecture`                   | CLICT-001  | **Reconciling** | PR #3209 — guide redirects, completed-index fixes         |
| 2   | `anvil policy` + `anvil exception`     | CLICT-002  | **Reconciling** | PR #3209 — public tutorial, runbook, beta guide           |
| 3   | `anvil drift`                          | CLICT-003  | **Reconciling** | PR #3209 — tutorial snapshot paths, `--overwrite` removed |
| 4   | `anvil watch`                          | CLICT-004  | **Done**        | Watch command/default-action docs reconciled              |
| 5   | `anvil gate` + `gate-config`           | CLICT-005  | **Done**        | Canonical check-name vocabulary reconciled                |
| 6   | `anvil intercept` + `anvil workspace`  | CLICT-006  | **Done**        | Runbook daemon/workspace subcommands reconciled           |
| 7   | Tier 2 runbook alignment (36 families) | CLICT-007  | **Proposed**    | Spot-check remaining families; fix runbook-only gaps      |
