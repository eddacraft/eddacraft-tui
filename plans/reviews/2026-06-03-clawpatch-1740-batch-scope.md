# Clawpatch #1740 test-hardening batch — scope (2026-06-03)

**Purpose:** execution scope for burning down the remaining open
[`clawpatch-pre-tag-v0.7.0-beta`](../modules/clawpatch-pre-tag-v0.7.0-beta.aps.md)
findings that sit in the low-severity test-hygiene batch tracker
([#1740](https://github.com/eddacraft/anvil-001/issues/1740)). Produced by a
2026-06-03 verify sweep of every remaining Draft finding against `origin/main`.

> **Why now.** Closing this batch lands CLAWP fully and unblocks **CIB-039**
> (archive the tracker + drop its `aps:active-lint` carve-out) in
> [continuous-improvement-backlog](../modules/continuous-improvement-backlog.aps.md).

## Sweep result

The sweep checked all 32 then-Draft findings. **4 were already shipped
untracked** and were reconciled to Merged via PR #2257 (CLAWP-034/-043/-044/-051,
25/65 → 29/65). Of the **28 still open**:

- **24 are batchable Rust test-hardening** (this doc).
- **3 are JS/TS config nits** (CLAWP-006/-010/-032) on the deliberately retiring
  JS/TS surface — **out of scope**, let them bitrot.
- **1 is re-deferred** — CLAWP-049 (Windows positive OPA assertions) is a
  documented Windows path-glob follow-up, not a test tweak; prod target is Linux
  and the negative cases already run on Windows. **Defer.**

## The common shape

Every finding here is "a test that passes for the wrong reason" — vacuous or
over-loose assertions. The deliverable is a strengthened assertion, and the
**acceptance bar is non-vacuity**: each strengthened test must be proven to fail
against a deliberate mutant (wrong count, missing field, dangling endpoint),
the same technique CLAWP-019 used. No production behaviour changes; the lone
helper change is CLAWP-048 (a test-support fn).

## Execution model — pure-code + one reconcile

Parallel/overlapping PRs in this module collide on the shared `N/M` count cell
(module header + index row). Per the
[2026-05-31 followups](./2026-05-31-clawpatch-rust-followups.md) precedent, keep
each batch PR **pure-code** (no APS status edits), then flip all closed items +
regenerate the count in **one reconcile PR** at the end via
`node scripts/aps/index-counts.mjs`.

Per-PR gates (dev-workflow): `cargo test -p <crate>` (the touched test files),
`cargo clippy --workspace --all-targets -- -D warnings`,
`cargo fmt --all --check`, Council, single-purpose PR.

## Batch contents — 24 items, ~10 test files, 3 PRs

Sizing: **T** = trivial (add/​tighten an assertion, < ~15 lines), **S** = small
(new fixture or several assertions), **M** = medium (new helper/fixture harness).

### PR 1 — `anvil-cli` (10 items)

| Item | File | Fix | Size |
| ---- | ---- | --- | ---- |
| CLAWP-040 | `tests/ai_guardrail_profile.rs` | iterate every diagnostic, not just `.first()` | T |
| CLAWP-052 | `tests/status_render.rs` | gate fixture-update on `ANVIL_UPDATE_FIXTURES == "1"` exactly | T |
| CLAWP-056 | `tests/version_offline.rs` | assert `current_version == env!("CARGO_PKG_VERSION")` | T |
| CLAWP-057 | `tests/version_offline.rs` | clear inherited auth env (`env_remove`/allowlist) | T |
| CLAWP-059 | `tests/status_verify_languages.rs` | exact unclassified count, not `>= 2` | T |
| CLAWP-058 | `tests/status_verify_languages.rs` | bind each language to its tier (row association) | S |
| CLAWP-060 | `tests/status_verify_languages.rs` | add a `.git/`-path fixture, assert excluded | S |
| CLAWP-048 | `tests/watch_json_output.rs` | `extract_json_blocks` fails loud on unparseable fenced JSON | S |
| CLAWP-035 | `tests/air_gapped.rs` | bounded child-process-with-timeout wrapper (kill on timeout) | **M** |
| CLAWP-045 | `tests/spawn_probe.rs` | negative test: hanging MCP cmd → returns within budget, no promotion | **M** |

### PR 2 — `anvil-checks` (7 items)

| Item | File | Fix | Size |
| ---- | ---- | --- | ---- |
| CLAWP-041 | `tests/antipattern_scanning.rs` | exact AP-003 count (+ distinct spans) | T |
| CLAWP-053 | `tests/surfenv_gitignore_hygiene.rs` | assert suppressed finding's `kind` + `suggested_pattern` | T |
| CLAWP-054 | `tests/command_safety_validation.rs` | assert full nested wrapper chain `["sudo","bash"]` | T |
| CLAWP-055 | `tests/command_safety_validation.rs` | pin `warns_on_git_clean_f` to `CommandAction::Warn` (matches existing score test — no prod change) | T |
| CLAWP-039 | `tests/surfenv_prod_value.rs` | assert all 4 expected keys/indicators (DATABASE_URL, FEATURE_FLAGS_ENV, SECRET_PROD, LEGACY_HOST) | S |
| CLAWP-042 | `tests/antipattern_scanning.rs` | add non-empty-catch negative test (AP-006 absent) | S |
| CLAWP-063 | `tests/secret_detection.rs` | run lockfile-hash case with entropy enabled | S |

### PR 3 — `anvil-intercept` + `anvil-policy` + `anvil-kernel-types` + `anvil-witness` (7 items)

| Item | File | Fix | Size |
| ---- | ---- | --- | ---- |
| CLAWP-064 | `anvil-intercept/tests/midedit_contract.rs` | `assert_busy_response` echoes the JSON-RPC id | T |
| CLAWP-050 | `anvil-policy/tests/opa_real_binary.rs` | exact `coverage_min` boundary (`== 80`) | T |
| CLAWP-061 | `anvil-kernel-types/tests/type_invariants.rs` | round-trip both edge endpoints, assert both present | T |
| CLAWP-062 | `anvil-kernel-types/tests/type_invariants.rs` | assert EventType↔EventPayload pairing for all 4 kinds | S |
| CLAWP-046 | `anvil-witness/tests/concurrency.rs` | exact `seq`/`commit_sha` set match across all writers | S |
| CLAWP-047 | `anvil-witness/tests/concurrency.rs` | `Arc<Barrier>` to force simultaneous append contention | S |
| CLAWP-037 | `anvil-intercept/tests/` (new) | process-level binary test: `--help` + invalid invocation via `env!("CARGO_BIN_EXE_anvil-intercept")` (no new dev-dep); SIGTERM-spawn variant optional | **M** |

## The 3 non-mechanical items

- **CLAWP-055** — looks like a design decision (Warn vs Block for `git clean -f`)
  but isn't: `check_reports_correct_score_for_mixed_findings` already asserts
  `warned == 1`, which fixes the contract at **Warn**. Pin the loose
  `matches!(.., Warn | Block)` to `Warn` exactly. No production change.
- **CLAWP-035 / CLAWP-045** — the only two needing new test scaffolding: a
  spawn-with-timeout wrapper (035) and a never-responding MCP stub (045). These
  are the real work; everything else is assertion tightening.
- **CLAWP-037** — minimum (`--help` + invalid invocation) is small via the
  Cargo-provided `CARGO_BIN_EXE_anvil-intercept` env var (no `assert_cmd`
  dependency). The `start` + SIGTERM spawn test is a Unix-only stretch; skip or
  note as follow-up.

## Deferred / out of scope

| Item | Reason |
| ---- | ------ |
| CLAWP-049 | Windows OPA positive assertions — documented path-glob follow-up; prod is Linux, negatives already run on Windows. |
| CLAWP-006 | `@types/node ^25` vs `engines.node >=24` — JS/TS retiring surface. |
| CLAWP-010 | Vitest `__tests__` discovery / `passWithNoTests` — JS/TS retiring surface. |
| CLAWP-032 | Inert `tsconfig` includes — JS/TS retiring surface. |

## Source of truth

- Tracker module: [`plans/modules/clawpatch-pre-tag-v0.7.0-beta.aps.md`](../modules/clawpatch-pre-tag-v0.7.0-beta.aps.md)
- Original audit export: [`plans/audits/2026-05-19-clawpatch-v0.7.0-beta.json`](../audits/2026-05-19-clawpatch-v0.7.0-beta.json)
- Batch tracker issue: [#1740](https://github.com/eddacraft/anvil-001/issues/1740)
