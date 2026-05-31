# Clawpatch Rust follow-ups — 2026-05-31 sweep

**Purpose:** portable backlog of the actionable Rust (`crates/`) findings from the
2026-05-31 v0.7.3-beta pre-tag clawpatch sweep, scoped so a fresh session can
pick them up without prior context.

**Source of truth:**

- Audit export: [`plans/audits/2026-05-31-clawpatch-v0.7.3-beta.json`](../audits/2026-05-31-clawpatch-v0.7.3-beta.json) (367 findings)
- Triage + verdict: [`plans/reviews/2026-05-31-clawpatch-triage.md`](./2026-05-31-clawpatch-triage.md)
- Tracker module: [`plans/modules/clawpatch-pre-tag-v0.7.0-beta.aps.md`](../modules/clawpatch-pre-tag-v0.7.0-beta.aps.md)

**Regenerate the non-test-gap list:**

```sh
jq -r '.items[] | select((.evidence[0].path|startswith("crates/")) and (.triage!="test-gap")) | "\(.id)\t\(.severity)/\(.triage)\t\(.evidence[0].path):\(.evidence[0].startLine)\t\(.title)"' plans/audits/2026-05-31-clawpatch-v0.7.3-beta.json
```

## Context for a new session

The Anvil product is the pure-Rust `crates/` binary. The JS/TS workspace
(`packages/`, `apps/`) is on a deliberate retirement path and publishes nothing,
so its 15 high-severity findings are the known
[#1826](https://github.com/eddacraft/anvil-001/issues/1826) umbrella and are out
of scope here. **None of the items below are release-tag-blocking** — the §2
council verdict for v0.7.3-beta is "proceed". Use the hardened flow: clean
worktree off `origin/main`, TDD, keep PRs pure-code (reconcile the CLAWP status
separately to avoid the shared index-count collision), Council, single-purpose
PR.

## Tier 1 — product source bugs (best fix candidates)

| Severity | File:line | Issue → Fix |
| -------- | --------- | ----------- |
| medium / risk ⭐ | `crates/anvil-cli/src/main.rs:412` | Credential **load failures** are coerced into a successful exit-0 "auth required" response (silent-degrade-to-default, the class that bit PR #1721). → Return a distinct non-zero (`EXIT_CONFIG_ERROR`) for load faults; only coerce truly missing/expired auth. |
| medium / confirmed-bug (security) | `crates/anvil-intercept/src/main.rs:94` | `scan_buffer` has no authenticated session binding — contract omits `sessionId`. → Add `sessionId`, validate against the connection, reject mismatch with a structured JSON-RPC error. |
| medium / contract-mismatch | `crates/anvil-cli/src/main.rs:814` | Pre-dispatch auth breaks `--format json` output contracts (auth envelope on the wrong stream/shape). → Resolve output mode before the auth gate for `--format`-bearing commands. |
| medium / contract-mismatch | `crates/anvil-intercept-rules/src/lib.rs:85` | Public `InterruptReason` can bypass the 1-based line invariant (`Some(0)`). → `Option<NonZeroU32>` or normalise at the boundary. |
| medium / contract-mismatch | `crates/anvil-kernel-types/tests/type_invariants.rs:41` | `EngineEvent` can represent a mismatched event type + payload. → Derive the kind from the payload, or validate on (de)serialise. |

## Tier 2 — Windows-only (`anvil-intercept-win32`, needs a Windows-capable verification pass)

| Severity | File:line | Issue → Fix |
| -------- | --------- | ----------- |
| medium / confirmed-bug (security) | `crates/anvil-intercept-win32/src/lib.rs:131` | Named-pipe client allows server-side impersonation (default SQOS). → Pass `SECURITY_SQOS_PRESENT` combined with `SECURITY_IDENTIFICATION`. |
| medium / risk | `crates/anvil-intercept-win32/src/lib.rs:291` | Process-liveness check treats exited handles as live. → Call `GetExitCodeProcess`, require `STILL_ACTIVE`. |

## Tier 3 — native-binding freshness (`anvil-checks-napi`, build reliability)

Three related findings — `crates/anvil-checks-napi/package.json:29`,
`crates/anvil-checks-napi/scripts/check-binding-fresh.mjs:12` and `:27`. The
`.node` freshness heuristic only mtimes `src/`, so a stale binding survives
`Cargo.toml`/registry/Rust-dependency changes, and an unrelated newer `.node`
can mask the one tests actually load. → Replace the mtime heuristic with a
build-stamp recording the exact input snapshot (or include all native build
inputs in the freshness calculation).

## Tier 4 — cheap docs-gaps (low, one-liners)

- `crates/anvil-baseline/src/lib.rs:48` — docs say adversarial-refresh detection is out of scope, but `analyze_refresh` / `RefreshSuspicion` are exported.
- `crates/anvil-witness/src/lib.rs:17` — crate docs advertise stale witness contracts; `crates/anvil-witness/src/lib.rs:55` — `AppendOutcome` is not re-exported from the library surface.
- `crates/anvil-checks-napi/src/lib.rs:20` — registry-load docs over-promise hard failure after the embedded-catalogue fallback.

## Do NOT re-file — carry-overs / residuals

- `crates/anvil-intercept/tests/midedit_contract.rs:25` = **CLAWP-005** ([#1737](https://github.com/eddacraft/anvil-001/issues/1737), Deferred, needs `anvil-rmcp`).
- `crates/anvil-kernel/tests/dual_run.rs:56` = **CLAWP-023** ([#1753](https://github.com/eddacraft/anvil-001/issues/1753), blocked on the TS engine).
- `crates/anvil-cli/tests/status_render.rs:93` + `crates/anvil-cli/tests/protection_claim_cross_surface.rs:37` = **CLAWP-027 residual** (PR #2145 fixed the `read_dir` count; these are adjacent same-pattern sites).
- `crates/anvil-run/tests/shell_integration.rs:100` = **CLAWP-031 residual** (PR #2143 quoted the sourced path; this is the stub-body `printf` the council flagged).
- `crates/anvil-kernel/tests/watch_pattern_filter.rs:43` = **CLAWP-033 class** (PR #2136), a different watcher test.

## Tier 5 — test-gap tail (~38, pure coverage)

Lower priority, same flavour as the CLAWP backlog already burned down this week
(assertions that pass for the wrong reason, missing-coverage, Windows-skipped
contracts). Top files: `crates/anvil-cli/tests/status_json_contract.rs`,
`crates/anvil-cli/tests/policy_eval.rs`, `crates/anvil-cli/tests/mcp_config.rs`
(2 each), plus singles across `anvil-checks`, `anvil-kernel`, `anvil-cli`,
`anvil-config`, `anvil-witness`, `anvil-policy-engine`, `eddacraft-tui`.
Full list:

```sh
jq -r '.items[] | select((.evidence[0].path|startswith("crates/")) and .triage=="test-gap") | "\(.evidence[0].path):\(.evidence[0].startLine)\t\(.title)"' plans/audits/2026-05-31-clawpatch-v0.7.3-beta.json
```
