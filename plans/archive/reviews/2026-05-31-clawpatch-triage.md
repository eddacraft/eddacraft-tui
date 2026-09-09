# Clawpatch triage — 2026-05-31 (v0.7.3-beta pre-tag fresh sweep)

**Scan command:** `clawpatch init && clawpatch map && clawpatch review --limit 341 --jobs 8`
**Status command:** `clawpatch status`
**Findings input:** `plans/audits/2026-05-31-clawpatch-v0.7.3-beta.json`
**Run:** `20260530T190547-c782a5` (codex provider, codex-cli 0.135.0)
**Corpus SHA:** `d0fe56c03` (`chore(release): prepare v0.7.3-beta`)

## Why this run

The release runbook §2 loop rule invalidates the prior audit
(`2026-05-19-clawpatch-v0.7.0-beta.json` and its `2026-05-25` rolling corpus)
once post-merge bits land. Three `v0.7.x-beta` patches plus the 2026-05-30
CLAWP burn-down have since changed `main`, so this is a **fresh full sweep on a
clean `origin/main` worktree** (empty findings/features, all 341 features
re-reviewed) — the proper §2-council pre-tag input, not an incremental delta.

## Scan summary

- Features mapped: **341** (heuristic; 1,251 source files, 416 owned)
- Features reviewed: **341** (full, `--jobs 8`)
- Findings: **367** — 15 high, 201 medium, 151 low (confidence: 256 high / 110 medium / 1 low)
- Triage mix: 109 confirmed-bug · 112 risk · 72 contract-mismatch · 56 test-gap · 18 docs-gap

### Area split (the decisive cut)

| Area | Findings | High-sev |
| ---- | -------- | -------- |
| `packages/` (JS/TS) | 185 | 13 |
| `apps/` (JS/TS) | 82 | 2 |
| `tools/` + `infra/` + `scripts/` + root | 32 | 0 |
| **`crates/` (the Rust product)** | **68** | **0** |

**Every high-severity finding is in the JS/TS workspace; the Rust binary that
ships as `anvil` has zero.**

## Verdict (§2 council readiness)

**The v0.7.3-beta Rust tag is not blocked by new high-severity risk.** The
shipping product is the pure-Rust `crates/` binary (the JS/TS workspace publishes
nothing to npm and is on a deliberate retirement path). All 68 Rust findings are
**medium or low**; the actionable subset is small and none are tag-blocking.

The 15 high-severity findings are the **known JS/TS class already tracked under
the 20-high umbrella [#1826](https://github.com/eddacraft/anvil-001/issues/1826)**
(path traversal, lock split-brain, unredacted-secret persistence, dist-publish
breakage). Per the 2026-05-29 triage rule they **must not be re-filed**. They are
out of scope for the Rust tag but need an owner decision — see JS/TS section.

## Triage — Rust product (`crates/`, 68)

### A. New, product-actionable (file as CLAWP items / fix candidates)

Security / correctness (medium):

- **`scan_buffer` cannot enforce session ownership — contract omits `sessionId`**
  (`crates/anvil-intercept/src/main.rs:94`, security/confirmed-bug). IPC scan
  request has no authenticated session binding; add `sessionId`, validate
  against the connection, reject mismatches.
- **Credential load failures are coerced to successful auth-required responses**
  (`crates/anvil-cli/src/main.rs:412`, risk/bug). A genuine *load fault* is
  flattened into the normal "auth required" exit-0 path — the same
  silent-degrade-to-default class called out in PR #1721. Return a distinct
  non-zero (`EXIT_CONFIG_ERROR`) for load faults; only coerce truly
  missing/expired auth.
- **Pre-dispatch auth breaks JSON output contracts**
  (`crates/anvil-cli/src/main.rs:814`, contract-mismatch). `--format json`
  action commands can emit non-JSON auth envelopes on the wrong stream; resolve
  output mode before the auth gate.
- **Public `InterruptReason` can bypass the 1-based line invariant**
  (`crates/anvil-intercept-rules/src/lib.rs:85`, contract-mismatch). Make the
  invariant unrepresentable (`Option<NonZeroU32>`) or normalise `Some(0)` at the
  boundary.
- **`EngineEvent` can represent mismatched event type and payload**
  (`crates/anvil-kernel-types/tests/type_invariants.rs:41`, contract-mismatch).
  Derive the kind from the payload or validate on (de)serialise.

Windows (`anvil-intercept-win32`, medium — affects the Windows build only):

- **Named-pipe client allows server-side impersonation (default SQOS)**
  (`crates/anvil-intercept-win32/src/lib.rs:131`, security/confirmed-bug). Pass
  `SECURITY_SQOS_PRESENT | SECURITY_IDENTIFICATION`.
- **Process-liveness check treats exited handles as live**
  (`crates/anvil-intercept-win32/src/lib.rs:291`, risk/bug). Call
  `GetExitCodeProcess` and require `STILL_ACTIVE`.

Native-binding freshness guard (`anvil-checks-napi`, medium build-release — 3
related findings): the `.node` freshness heuristic only mtimes `src/`, so it can
accept a stale binding after `Cargo.toml`/registry/Rust-dep changes, and an
unrelated newer `.node` can mask the loaded one. Replace the mtime heuristic
with a build-stamp recording the exact input snapshot.

Docs-gaps (low, cheap one-liners): `anvil-baseline` lib docs say adversarial
refresh detection is out-of-scope while `analyze_refresh` is exported;
`anvil-witness` advertises stale witness contracts and doesn't re-export
`AppendOutcome`; `anvil-checks-napi` registry-load docs over-promise hard
failure after the embedded-catalogue fallback.

### B. Carry-overs / residuals (do not double-file)

- **CLAWP-005** echo — "Reusable contract API trapped inside an integration-test
  crate" (`midedit_contract.rs:25`). Already tracked, Deferred (#1737, needs
  `anvil-rmcp`).
- **CLAWP-023** echo — "Dual-run parity harness never executes the TypeScript
  engine" + "Dual-engine comparison ignores duplicate violation counts"
  (`dual_run.rs`). Already tracked/blocked (#1753).
- **CLAWP-027 residual** — fixture update-mode races now appear in *other* files
  (`status_render.rs:93` round-trip reader; `protection_claim_cross_surface.rs:37`).
  #2145 fixed the `read_dir` count in `status_render`; these are adjacent
  same-pattern sites the lock fix didn't reach.
- **CLAWP-031 residual** — "Generated shell stubs embed temp paths without shell
  quoting" (`shell_integration.rs:100`). #2143 quoted the *sourced* path; the
  council flagged the stub-body `printf` quoting as pre-existing — this is it.
- **CLAWP-033 class** — watcher tests still use sleep-based coordination
  (`watch_pattern_filter.rs:43`, `watcher_integration.rs:13`); same flake class,
  different tests than the one #2136 hardened.

### C. Test-gap tail (~38)

The remainder of `crates/` is `test-gap` (assertions that pass for the wrong
reason, missing-coverage, Windows-skipped contracts) — the same flavour as the
CLAWP backlog already burned down this week. Candidates for a future
hardening batch; none block the tag.

## Triage — JS/TS workspace + tooling (299)

- **15 high-severity = the [#1826](https://github.com/eddacraft/anvil-001/issues/1826)
  umbrella class** (path traversal in speckit/opa-binary/edda/file-cache; lock
  split-brain in `lock-manager`/`aps state`; unredacted-secret persistence in
  `kindling-integration`; `dist` publish breakage; missing runtime template
  YAML). **Not re-filed; not Rust-tag-blocking.**
- **Owner decision needed, NOT auto-dismissed:** if any JS/TS surface is actually
  deployed — notably `apps/anvil-api` (auth-session/auth-device token-rotation
  data-loss findings), `apps/docs-site` (`feature-flags.ts` subjectless beta
  access), and the `infra/` Vercel components — those high/medium findings are
  real for that surface and should be triaged by its owner independently of the
  Rust retirement timeline. This sweep does not adjudicate the retirement scope.
- The remaining medium/low JS/TS findings are advisory backlog on the retiring
  surface; do not convert to APS work without a deliberate "fix-forward vs
  bitrot" call.

## Recommended next actions

1. **§2 council:** proceed for the v0.7.3-beta Rust tag — no new high-severity
   product risk; cite this artefact as the fresh sweep.
2. **File a small CLAWP follow-up batch** for the §A new Rust items, prioritising
   the two non-Windows confirmed/contract bugs in `anvil-cli`/`anvil-intercept`
   (credential-load coercion, `scan_buffer` session binding) and the cheap
   docs-gaps. Defer the Windows `anvil-intercept-win32` pair to a Windows-capable
   verification pass.
3. **Route the JS/TS highs to the workspace owner** for an explicit
   retire-vs-fix decision; keep them under #1826, do not re-file.
4. Refresh `plans/modules/clawpatch-pre-tag-v0.7.0-beta.aps.md` provenance to
   point at this 2026-05-31 corpus (the 2026-05-19 audit is now superseded).
