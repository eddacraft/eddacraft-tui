# CLI Command Truth Review

| Type   | Authority | Owner  | Status | Freshness                                      |
| ------ | --------- | ------ | ------ | ---------------------------------------------- |
| Review | Advisory  | CLICT  | WIP    | Opened 2026-07-06 — architecture slice first |

| Upstream                                                                 | Downstream                                      |
| ------------------------------------------------------------------------ | ----------------------------------------------- |
| `crates/anvil-cli/src/commands/`, `docs/runbooks/cli-surface.md`, APS CLICT | Doc reconciliation, APS work items, build gates |

**Status:** Work in progress. This file is the running audit log for documented
CLI commands versus runtime behaviour. Fan-out from here command-family by
command-family.

**APS module:** [`plans/modules/cli-command-truth.aps.md`](../../plans/modules/cli-command-truth.aps.md)

---

## Operating model

This review is a **living audit log**, not a one-off report. Each top-level CLI
command family (or tightly coupled family group) gets the same treatment
architecture received in slice 1.

### Per-family loop

| Step | Owner | Output |
| ---- | ----- | ------ |
| 1. Audit | Review doc | New **slice** — doc inventory, runtime map, substitutes, tests, drift hotspots |
| 2. Plan | CLICT APS | New **CLICT-00N** work item (Ready) scoped to docs reconciliation only |
| 3. Reconcile | CLICT-00N | Docs aligned to `anvil <cmd> --help` *today*; redirects for behaviour that already ships elsewhere |
| 4. Build (optional) | Vertical APS | Design gate + implementation (e.g. ARCHCFG-006..014 for architecture) |
| 5. Re-audit | Review slice + CLICT | Refresh after each merge that adds or changes subcommands |

### Division of labour

- **CLICT** — what documentation *claims* versus what the CLI *registers and
  dispatches today*. Reconciliation work items correct guides, public docs, APS
  false-complete rows, and CHANGELOG drift.
- **Vertical modules** — what to *build* and how (design gates, implementation,
  engine wiring). CLICT does not own code changes.

Reconciliation is **phase 1 of N** for any family where builds are planned:
docs fixed now stop agent/user pain; a follow-up CLICT pass (or slice refresh) is
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
commands that are not registered in `crates/anvil-cli`, or describe behaviour the
implementation does not provide. Agents and users treat guides as executable
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
2. **Inventory runtime** — `anvil <cmd> --help`, `crates/anvil-cli/src/commands/`,
   `main.rs` dispatch, feature-flag/auth gates.
3. **Map substitutes** — top-level commands or gate checks that already provide
   the described behaviour under a different name.
4. **Map tests** — unit, integration, E2E, crate-level.
5. **Record verdict** — exists / stub / redirect / missing / falsely marked
   complete.
6. **Open APS item** — docs reconciliation before build decisions where scope is
   unclear.

---

## Slice 1: `anvil architecture` (2026-07-06)

### Documentation layers (conflicting claims)

| Source | Authority | Commands claimed |
| ------ | --------- | -------------- |
| `docs/guides/custom-architecture-policies.md` | Live guide (stale) | **10** — `init`, `validate`, `check`, `watch`, `visualise`, `list`, `impact`, `export`, `debug`, plus `check --fix` / `--baseline-all` |
| `docs/runbooks/cli-surface.md` | Authoritative runbook | **2** — `validate`, `show` |
| `docs/public/anvil/tutorials/architecture.md` | Public tutorial | **2** + gate substitute — `validate`, `show`, `gate --only-checks import-boundaries` |
| `docs/public/anvil/operations/config.md` | Public ops | `validate`, `show` |
| `plans/completed.aps.md` / `completed-index.aps.md` | APS (wrong) | `init` (OPA-004), `visualise` (TUI-015) marked **Complete** |
| `CHANGELOG.md` | Release notes (wrong) | Claims `anvil architecture visualise` shipped |

**Runtime truth** (`anvil architecture --help`, 2026-07-06): only **`validate`**
and **`show`**.

### Command-by-command map

| # | Documented command | In `anvil architecture` CLI? | Code location | Wired to `anvil_architecture`? | Tests |
| - | ------------------ | ---------------------------- | ------------- | ----------------------------- | ----- |
| 1 | `init` | No | `crates/anvil-architecture/src/yaml_parser.rs` (`create_definition_from_template`); legacy TS templates in `packages/anvil/core/src/architecture/templates/` | Crate API only | Crate + TS template tests |
| 2 | `validate` | **Yes** | `crates/anvil-cli/src/commands/architecture.rs` | **No** — local `serde_yaml::Value` parse; `depends_on` ref check only | 14 unit tests in `architecture.rs` |
| 3 | `show` | **Yes** | same file | **No** — same shallow parser | Covered in the 14 tests |
| 4 | `check` | No | `crates/anvil-cli/src/commands/gate.rs` → `run_check_architecture()` | **Yes** — full boundary analysis | 7+ gate tests |
| 5 | `watch` | No (doc: `anvil architecture watch`) | `crates/anvil-cli/src/commands/watch.rs` | **Yes** (kernel watcher) | watch + kernel tests |
| 6 | `visualise` | No | **None** | — | **None** |
| 7 | `list` | No | **None** (partial overlap with `show`) | — | **None** |
| 8 | `impact` | No | **None** | — | **None** |
| 9 | `export` | No (doc: `anvil architecture export`) | `crates/anvil-cli/src/commands/export.rs` (`anvil export`) | Partial — baseline read | 37 export tests |
| 10 | `debug` | No | **None** | — | **None** |

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

| Surface | Role | Code |
| ------- | ---- | ---- |
| `anvil gate` / `import-boundaries` | Boundary enforcement | `gate.rs` |
| `anvil watch` | Live boundary monitoring | `watch.rs` |
| `anvil drift *` | Baseline drift | `drift.rs` |
| `anvil dashboard architecture` | Architecture-health TUI | `commands/dashboard/architecture.rs` |
| `anvil export` | Agent context incl. baseline | `export.rs` |
| MCP `anvil_query_boundary` | Pre-write import check | `mcp/tools/query_boundary.rs` |
| `eddacraft-anvil-architecture` crate | Core engine | 138 crate tests |

### Test coverage gaps

| Layer | Covers | Gap |
| ----- | ------ | --- |
| `architecture.rs` (14 tests) | Arg parsing, shallow YAML | No `run()` E2E; no `anvil_architecture` integration |
| `gate.rs` (7 architecture tests) | Real boundary enforcement | No subprocess test for `anvil architecture validate` |
| E2E (`apps/e2e`) | Violation fixture helper only | No `anvil architecture *` suite |
| TCOV-004 “Complete” | — | Marks stub-path tests as full command coverage |

### Documentation drift hotspots (architecture)

1. **`docs/guides/custom-architecture-policies.md`** — highest priority; frontmatter
   claims “Live” and review against `architecture.rs` but lists eight commands
   that do not exist.
2. **`plans/completed.aps.md`** — `OPA-004` (`init`), `TUI-015` (`visualise`)
   falsely Complete.
3. **`CHANGELOG.md`** — `visualise` claim with no implementation.
4. **`plans/specs/2026-03-18-rust-cli-design.md`** — planned `watch` under
   `architecture.rs`; lives under top-level `anvil watch` instead.

Accurate today: `docs/runbooks/cli-surface.md`, `docs/public/anvil/operations/config.md`,
public architecture tutorial (validate + show + gate redirect).

### APS cross-links (architecture)

| Item | Module | Role |
| ---- | ------ | ---- |
| **CLICT-001** | CLICT | Docs reconciliation — correct guides, public docs, false Complete/CHANGELOG claims |
| **ARCHCFG-006** | ARCHCFG | Design gate — per missing command: build / defer / reject / redirect (merged PR #3203) |
| **ARCHCFG-007..014** | ARCHCFG | Implementation candidates, each blocked on ARCHCFG-006 |
| **ARCHCFG-001..005** | ARCHCFG | Wire `validate` to real semantic validation + gate preflight |

**Sequencing:** CLICT-001 (docs truth) can land before ARCHCFG-006 (build
decisions). Docs should redirect to existing surfaces where overlap is already
known (`check` → gate, `watch` → `anvil watch`, etc.) even while ARCHCFG-006
is open.

### PR #3203 (merged 2026-07-05)

[PR #3203](https://github.com/eddacraft/anvil-001/pull/3203) added **ARCHCFG-006..014**
to `plans/modules/architecture-config-validation.aps.md`:

- **ARCHCFG-006** — design gate with evidence-backed overlap analysis per
  candidate command (init highest-confidence yes; check/watch/visualise/export/
  list/impact/debug each mapped to existing or missing capability).
- **ARCHCFG-007..014** — one Draft work item per candidate command, all blocked
  on ARCHCFG-006.

No code or doc content changed in that PR — planning only. CLICT-001 complements
it by fixing user-facing documentation now rather than waiting for build verdicts.

---

## Slice queue

| # | Command family | CLICT item | Status | Notes |
| - | -------------- | ---------- | ------ | ----- |
| 1 | `anvil architecture` | CLICT-001 | **Audited** | Slice 1 complete; reconciliation Ready |
| 2 | `anvil policy` | CLICT-002 | **Queued** | Guides, OPA/regorus docs, `anvil policy *` vs runtime |
| 3 | `anvil drift` | CLICT-003 | **Queued** | Tutorial/docs vs `drift snapshot/compare/report/list` |
| 4 | `anvil watch` | CLICT-004 | **Queued** | Daemon docs, architecture-watch conflation, NDJSON lifecycle |
| 5 | `anvil gate` | CLICT-005 | **Queued** | Check aliases, profile docs, quality-model vocabulary |
