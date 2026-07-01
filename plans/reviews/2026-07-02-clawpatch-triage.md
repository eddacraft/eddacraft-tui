# Clawpatch triage — 2026-07-02 (verify-first backlog cut)

**Prior scan:** `clawpatch map && clawpatch review` (run `20260701T170630-f5a709`, codex provider)
**Status command:** `clawpatch status`
**Findings input:** `plans/audits/2026-07-02-clawpatch-periodic-scan.json`
**Corpus SHA:** `d1fded280` (`main`)
**Predecessor:** `plans/reviews/2026-06-20-clawpatch-triage.md`

## Why this run

The 2026-06-20 pass filtered worktree noise and left ~246 open items on the
canonical tree. This pass is a **verify-first triage** of the current run
(`20260701T170630`, 126 open at entry): every finding that read as
product-blocking was checked against `main` source before recording a verdict,
rather than trusting the scanner's evidence line refs (which lag the tree).

Three findings were dispositioned this session; open count **126 → 123**. The
clawpatch state directory is gitignored (`.clawpatch/.gitignore` = `*` except
`config.json`), so the verdicts live in local state and the durable record is
this doc plus the exported audit JSON.

## Verdicts recorded (this session)

| Finding | Verdict | Basis |
| ------- | ------- | ----- |
| `` `start --verify` `` not in local-probe auth bypass (medium, `anvil-cli/src/main.rs`) | **fixed** | Stale — CIB-049 already on `main` |
| Adapter persists raw observations unredacted (high, `kindling-integration/adapter.ts`) | **wont-fix** | Real bug in provably dead code |
| Sensitive keywords persisted unredacted (high, `kindling-integration/observation-contract.ts`) | **wont-fix** | Real bug in provably dead code |

### 1. `start --verify` auth bypass — stale, marked `fixed`

The 2026-06-20 triage listed this as a product-actionable medium at
`main.rs:170`. It is **already resolved on `main`** under CIB-049 and the scan's
evidence line refs predate the current tree:

- `skips_auth_for_local_probe` now has `Commands::Start(args) => args.verify`
  (`crates/anvil-cli/src/main.rs:454`).
- Unit test `local_probe_skip_matches_start_verify` asserts the bypass.
- Air-gapped subprocess regression
  `anvil_start_verify_json_runs_local_probe_unauthenticated`
  (`crates/anvil-cli/tests/air_gapped.rs:409`) covers the runtime path.

No further action; the finding will not resurface.

### 2 & 3. Kindling redaction highs — real bugs, dead code, `wont-fix`

Both are genuine data-privacy defects (raw command / error strings persisted
with `redacted: false` and no effective redaction pass). Both are in the
**retiring JS/TS** `packages/kindling-integration` surface, and were verified to
be **unreachable at runtime** before recording `wont-fix`:

- No `package.json` depends on `@eddacraft/anvil-kindling-integration`.
- The package exposes a library `main` only — **no `bin`**; nothing executes it.
- `AnvilKindlingAdapter` is constructed **only in `adapter.test.ts`**; no app or
  package calls `emit()`. The redaction branch in `observation-contract.ts` is
  reached only from the package's own tests/benchmarks.
- Rust crates reference the package **only in doc-comments**, never spawn it.
  `crates/anvil-intercept/src/kindling_observation.rs` explicitly lists the
  TS-side emit path as a **deferred, not-yet-wired follow-up** ("when the daemon
  gains a Kindling client handle").
- The live kindling path is the Rust `KindlingDaemonSink` (default since
  KDS-005), which supersedes the TS writer.

These are the [#1826](https://github.com/eddacraft/anvil-001/issues/1826)
umbrella class (kindling secret persistence). Per the 2026-05-29 rule they
**must not be re-filed**; the retire decision stands. If the TS surface is ever
revived, both defects re-open with it.

## Scan summary (post-triage)

| Metric | 2026-06-20 | 2026-07-02 |
| ------ | ---------- | ---------- |
| Total findings | 715 | 761 |
| Open | 246 | **123** |
| Open highs | 4 | **0** |

### Open finding mix (123)

- **Severity:** 0 high · 61 medium · 62 low
- **Triage:** 64 test-gap · 23 risk · 20 confirmed-bug · 15 contract-mismatch · 1 docs-gap
- **Confidence:** 95 high · 28 medium

### Area split (the decisive cut)

| Area | Open | High-sev |
| ---- | ---- | -------- |
| `crates/**/tests/` (Rust test hygiene) | 77 | 0 |
| `crates/**/src/` (Rust product) | **2** | **0** |
| `packages/` + `apps/` (JS/TS) | 29 | 0 |
| `scripts/` + `infra/` + `tools/` | 15 | 0 |

## Verdict

**The shipping Rust product has zero actionable defects from this scan.** Of the
79 open `crates/` findings, only **2 touch product source** — both low — and the
one item that read as severe (`start --verify`) was stale. Both high-severity
findings are real bugs in provably dead JS/TS code. Nothing here is tag-blocking
for the pure-Rust CLI.

## Triage — Rust product source (`crates/**/src/`, 2 open)

- **Help/usage paths run daemon tracing before clap handles them** (low,
  contract-mismatch, `crates/anvil-intercept/src/main.rs:49`). `init_tracing`
  runs before `Cli::parse()`, so `--help` / `--version` register the daemon
  subscriber first. **Confirmed but cosmetic** — help still prints correctly and
  exits 0; the only effect is a subscriber registration on an exit path. Fix
  candidate for a CIB tidy (parse before init), not blocking.
- **Pool cap behaviour is not covered by the library test** (low, test-gap,
  `crates/anvil-rayon-init/src/lib.rs`). `cap_threads` was deliberately factored
  out as a pure function "so the cap can be pinned by a unit" — the unit is
  missing. Clean CIB test-add.

## Triage — Rust test hygiene (`crates/**/tests/`, 77 open)

Overwhelmingly `test-gap` coverage suggestions plus two low `confirmed-bug`
items in harness code:

- **Fixture update mode is racy under the default Rust test harness**
  (`crates/anvil-cli/tests/status_render.rs`) — the `status_render.rs:47`
  fixture-update family flagged in prior passes.
- **Panic files are omitted from the reported not-cleanly-parsed rate**
  (`crates/anvil-kernel/tests/langtail_external_validation.rs`).

Route as a future test-hardening batch (same pattern as CLAWP #1740 → PRs
#2261 / #2265 / #2267). None block shipping.

## Triage — JS/TS workspace + apps (`packages/`, `apps/`, 29 open)

**Owner decision carried from 2026-06-20:** JS/TS is being retired. All 29 open
findings remain `wont-fix` under the `#1826` retirement verdict — including the
two kindling redaction highs dispositioned above. Do not re-file or fix-forward
unless retirement scope is explicitly reversed.

## Triage — tooling (`scripts/`, `infra/`, `tools/`, 15 open)

Unchanged from the 2026-06-20 disposition (dogfood + infra confirmed-bugs and
contract-mismatches). Cluster one item per file family for CIB / infra owner;
none block the Rust product.

## Recommended next actions

1. **No release blocker** — the pure-Rust CLI is clear; do not gate a tag on
   this backlog.
2. **Optional CIB tidy** — the two low product-src items
   (`anvil-intercept` tracing-before-clap, `anvil-rayon-init` pool-cap test) are
   small, safe, and self-contained.
3. **Do not re-file the kindling highs** — they are the `#1826` umbrella class
   and are wont-fix under retirement; they re-open only if the TS surface is
   revived.
4. **Re-run after the next `main` advance** — evidence line refs lag the tree
   (as `start --verify` showed); always verify a "product-actionable" finding
   against current source before filing.
