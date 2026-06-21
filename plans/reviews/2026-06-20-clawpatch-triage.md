# Clawpatch triage — 2026-06-20 (corpus hygiene + filtered backlog)

**Prior scan:** `clawpatch map && clawpatch review` (run `20260618T190747-355c1c`, codex provider)
**Status command:** `clawpatch status`
**Findings input:** `plans/audits/2026-06-20-clawpatch-periodic-scan.json`
**Corpus SHA:** `e3e0c1078` (`main`)

## Why this run

The local clawpatch state had grown to **715 findings / 548 open**, with **302 open
items (55%)** pointing at `.claude/worktrees/**` — stale worktree copies of files
already on `main`. That noise made the backlog look unmanageable and duplicated
eleven of the fifteen open high-severity items.

This pass is **corpus hygiene plus council-style triage**, not a fresh full sweep.
It filters worktree duplicates, records durable verdicts for the canonical tree,
and exports a post-filter audit artefact. The underlying review run
(`20260618T190747-355c1c`) is unchanged.

## Hygiene actions (this session)

1. **Config:** added `.claude/worktrees/**` to `.clawpatch/config.json` `exclude`
   (`.worktrees/**` was already excluded; Worktrunk also parks trees under
   `.claude/worktrees/`).
2. **Bulk triage:** marked **302** open worktree-copy findings `wont-fix` with
   note `worktree copy — not canonical tree`.
3. **Export:** `plans/audits/2026-06-20-clawpatch-periodic-scan.json` captures
   post-hygiene state.

## Scan summary (post-hygiene)

| Metric | Before hygiene | After hygiene |
| ------ | -------------- | ------------- |
| Total findings | 715 | 715 |
| Open | 548 | **246** |
| Wont-fix (worktree) | 0 | **302** |
| Features mapped | 686 | 686 |

### Open finding mix (246 on canonical tree)

- **Severity:** 4 high · 146 medium · 96 low
- **Triage:** 72 confirmed-bug · 68 test-gap · 55 contract-mismatch · 39 risk · 12 docs-gap
- **Confidence:** not re-summarised — use audit JSON for per-finding detail

### Area split (the decisive cut)

| Area | Open | High-sev |
| ---- | ---- | -------- |
| `packages/` (JS/TS) | 106 | 3 |
| `apps/` (JS/TS) | 55 | 1 |
| **`crates/` (Rust product)** | **73** | **0** |
| `scripts/` + `infra/` | 6 | 0 |
| `.claude/` (non-worktree) | 0 | 0 |

**Every open high-severity finding is on the retiring JS/TS workspace. The
shipping Rust binary has zero open highs.**

## Verdict

**The Rust product is not blocked by clawpatch findings.** After worktree noise
removal, the actionable canonical backlog is **~246 items**, not 700+. None are
tag-blocking for the pure-Rust CLI.

The **4 open high-severity findings** are the known [#1826](https://github.com/eddacraft/anvil-001/issues/1826)
umbrella class (kindling secret persistence ×3, website device-activation legacy
payload ×1). Per the 2026-05-29 triage rule they **must not be re-filed**. They
need an owner **retire-vs-fix** decision on the JS/TS surface, not a fresh issue
per finding.

## Triage — Rust product (`crates/`, 73 open)

### A. Product-actionable (medium — fix candidates for CIB or a focused APS slice)

Contract / correctness:

- **`start --verify` is not included in the local-probe auth bypass**
  (`crates/anvil-cli/src/main.rs:170`, contract-mismatch).
- **impact affected-symbol cap is input-order dependent despite deterministic
  report contract** (`crates/anvil-gctx-egress/src/lib.rs:363`, confirmed-bug).
- **find_dependents silently hides dependents when the lock-held walk hits its
  node cap** (`crates/anvil-gctx-egress/src/lib.rs:238`, contract-mismatch).
- **ImpactQuery cannot deserialize an empty query into the structured
  InvalidQuery path** (`crates/anvil-gctx-types/src/lib.rs:385`, contract-mismatch).
- **Eval silently drops all but the first Rego query expression/result**
  (`crates/anvil-policy-engine/src/lib.rs:81`, contract-mismatch).
- **Region builders allow schema-invalid zero line or column values**
  (`crates/anvil-sarif/src/lib.rs:287`, contract-mismatch).

Build / CI risk:

- **Pretest freshness check can approve a stale native binding** and **freshness
  guard ignores native dependency inputs** (`crates/anvil-checks-napi/package.json:29`,
  risk ×2). Related to the CLAWP-019 / #2181 class — verify whether residual or
  regression before filing.

### B. Carry-overs / residuals (do not double-file)

- **CLAWP-005 echo** — "Shared contract API is trapped inside an integration test
  target" (`midedit_contract.rs:25`). Deferred (#1737, needs `anvil-rmcp`).
- **CLAWP-027 / CLAWP-033 class** — `status_render.rs:47` fixture-update race;
  `device_flow_e2e.rs:99` hang-instead-of-fail. Same test-hygiene families the
  #1740 batch and follow-up PRs addressed — batch, do not re-file individually.
- **CLAWP-023** — any dual-run / TS-parity echoes remain **Ship** (TS engine
  retiring).

### C. Test-gap tail (56)

The majority of open `crates/` findings are `test-gap`. Candidates for a future
test-hardening batch (same pattern as CLAWP #1740 → PRs #2261 / #2265 / #2267).
None block shipping.

## Triage — JS/TS workspace + apps (161 → Ship)

**Owner decision (2026-06-20):** JS/TS is being retired. All **161** open
`packages/` and `apps/` findings are marked `wont-fix` with note
`JS/TS retirement — Ship under #1826`. Do not re-file or fix-forward unless
retirement scope is explicitly reversed.

The four highs (kindling secret persistence ×3, website device-activation ×1)
and all `anvil-api` medium findings are covered by this verdict — no separate
clustered issue required.

## Triage — tooling (`scripts/` + `infra/`, 6 open)

| Severity | Triage | Path | Title |
| -------- | ------ | ---- | ----- |
| medium | confirmed-bug | `scripts/dogfood/external-fp/classify.py` | Warning file paths can escape the checked repository |
| low | confirmed-bug | `scripts/dogfood/external-fp/classify.py` | Worksheet build fails when output directory is absent |
| medium | contract-mismatch | `infra/src/components/vercel-app.ts` | domainImports accepts import IDs for a different domain |
| medium | confirmed-bug | `infra/scripts/admin-key-manage.mjs` | SSL is disabled by substring matching the connection string |
| low | contract-mismatch | `infra/scripts/admin-key-manage.mjs` | Documented create output uses hashed_key but script emits hashedKey |

Route the `scripts/dogfood` pair to **CIB** (dogfood tooling). Route `infra/`
items to the infra owner or CIB — cluster as one item per file family, not five
separate issues.

## Recommended next actions

1. **Keep `.claude/worktrees/**` excluded** — re-run `clawpatch map` after
   removing stale worktrees (`wt remove`) so future scans do not re-accumulate
   duplicate features.
2. **Do not treat the remaining open backlog as a blocking gate.** After hygiene
   and JS/TS retirement verdicts, **85 open** findings remain — almost all
   `crates/` test-gap and a handful of tooling items.
3. **Rust fixes:** file a small CIB batch for the §A contract/correctness items
   (6 findings, no Windows-only deps). Defer test-gap (56) to a dedicated
   hardening batch.
4. **Tooling:** optional CIB items for `scripts/dogfood` (2) and `infra/` (3)
   if those surfaces outlive JS/TS retirement; otherwise Ship with the stack.
5. **`clawpatch fix`:** run one finding at a time from a **clean tree** only;
   prefer manual TDD using `suggestedRegressionTest` for non-trivial items.

## Evidence

- `clawpatch status` (post-hygiene): 686 features, 715 findings, **246 open**, 0
  active locks, last run `20260618T190747-355c1c`.
- Worktree noise removed: **302** findings → `wont-fix`.
- JS/TS retirement verdict: **161** findings → `wont-fix` (Ship under #1826).
- **Open after both passes: 85** (primarily `crates/`).
- Config change: `.clawpatch/config.json` excludes `.claude/worktrees/**`.
- `clawpatch fix` was **not** run (config + triage-only pass).
- Tracker: `clawpatch-pre-tag-v0.7.0-beta` is **archived** (CIB-039). New work
  routes to CIB items or owning APS modules, not CLAWP-NNN IDs.