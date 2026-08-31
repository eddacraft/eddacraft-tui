# Clawpatch intake triage — 2026-08-30 to 2026-08-31

| Type  | Authority | Owner  | Status | Freshness                                                                                                                                                                                                                                                  |
| ----- | --------- | ------ | ------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Guide | Advisory  | CLAW30 | Live   | Reviewed 2026-08-31 against the complete persisted Clawpatch store, runs `20260830T054354-0927e9`, `20260830T165831-0f8d78`, and `20260831T032953-703e11`, filed GitHub issues, and current source under `apps/`, `crates/`, and `scripts/` at `80a2da110` |

| Upstream                                                                                                                                                            | Downstream                                                                                                                                                                                                                                                                                                                                                 |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Persisted Clawpatch finding/run records; current source and tests; [issue triage and APS authority](../../plans/specs/2026-05-28-issue-triage-and-aps-authority.md) | [CLAW30 intake receipt](../../plans/archive/modules/clawpatch-2026-08-30-intake.aps.md); GitHub issues [#4230](https://github.com/eddacraft/anvil-001/issues/4230)–[#4233](https://github.com/eddacraft/anvil-001/issues/4233) and [#4280](https://github.com/eddacraft/anvil-001/issues/4280)–[#4282](https://github.com/eddacraft/anvil-001/issues/4282) |

**Source paths reviewed:**

- `apps/docs-shell/app/auth/logout/route.ts`
- `crates/anvil-cli/src/mcp/validation.rs`
- `crates/anvil-cli/src/services/sample_analyser.rs`
- `crates/anvil-cli/src/graph_base_producer.rs`
- `crates/anvil-cli/tests/secret_coverage_surfaces.rs`
- `crates/anvil-checks/src/conformance/evaluate.rs`
- `crates/anvil-checks/tests/conformance_pr_evaluate.rs`
- `crates/anvil-checks/tests/secret_file_coverage.rs`
- `crates/anvil-checks/tests/secret_streaming.rs`
- `crates/anvil-checks/tests/secret_vendored_tier1.rs`
- `crates/anvil-intercept/tests/pattern_warm.rs`
- `scripts/secret/convert-gitleaks-rules.py`
- `scripts/secret/convert_gitleaks_rules_test.py`
- `scripts/secret/refresh-gitleaks-ruleset.sh`

## Decision

Two completed runs appeared after the first triage pass but before publication,
so the complete store and current source were refreshed. Across the resulting
bounded intake, thirteen records were calibrated:

- keep eleven substantiated records open and route them through seven coherent
  GitHub issues;
- mark one older temporary-directory finding fixed because the current suite
  owns fixtures with `TempDir` and tests unwind cleanup;
- mark one raw-detail assertion finding `wont-fix` because the exported record
  type has no raw-detail field and the mapping copies only approved fields;
- do not implement any finding in this intake PR;
- do not edit the shared CIB module or extend the bounded CLAWOPEN source set.

This document is review evidence, not a second backlog. The GitHub issues own
the filed follow-ups until a non-small item is promoted into APS.

## Corpus and scope

The complete persisted store was inventoried before each latest-run selection.
The final read-back snapshot was:

| Metric                |                                      Value |
| --------------------- | -----------------------------------------: |
| Persisted findings    |                                        995 |
| Open findings         |                                        517 |
| Active locks          |                                          0 |
| Lock files            |                                          0 |
| Latest run            |                   `20260831T032953-703e11` |
| Current-source anchor | `80a2da1106a4b8813c1c9c36d433f43ea1f7d844` |

| Run                      | Selected records | Reason                                                                                   |
| ------------------------ | ---------------: | ---------------------------------------------------------------------------------------- |
| `20260830T054354-0927e9` |                7 | Three new records plus four older records grouped by that report's action clusters       |
| `20260830T165831-0f8d78` |                5 | Every new record produced after the first pass                                           |
| `20260831T032953-703e11` |                1 | The latest new record; its report action clusters repeated already-routed older findings |

The lifetime-open count is not a count of newly confirmed defects. This intake
selected run-created records and explicit report companions, while the complete
store inventory and duplicate searches prevented the newest report from being
mistaken for the whole queue.

## Filed clusters

| Issue                                                                                                         | Priority and readiness                       | Findings                                                                                             | Disposition |
| ------------------------------------------------------------------------------------------------------------- | -------------------------------------------- | ---------------------------------------------------------------------------------------------------- | ----------- |
| [#4230 — make docs-shell logout same-origin and non-GET](https://github.com/eddacraft/anvil-001/issues/4230)  | P2 · needs design · needs APS                | `fnd_sig-feat-library-b870a9c90e-cbf2_86a19abeb4`                                                    | Open        |
| [#4231 — bound daemon and Git-history reads](https://github.com/eddacraft/anvil-001/issues/4231)              | P2 · ready in its authority lane · needs APS | `fnd_sig-feat-cli-command-7eda0e78cc-_51f593c5c3`; `fnd_sig-feat-cli-command-fb9c8a5a30-_7c7af02c9c` | Open        |
| [#4232 — bound base-graph committed-blob buffering](https://github.com/eddacraft/anvil-001/issues/4232)       | P2 · needs design · needs APS                | `fnd_sig-feat-cli-command-ba5ccdd3a6-_c5445f44fe`                                                    | Open        |
| [#4233 — use RAII cleanup in secret-scan fixtures](https://github.com/eddacraft/anvil-001/issues/4233)        | P3 · ready · small-fix lane                  | `fnd_sig-feat-test-suite-9f421eac7d-4_f6f9abfd11`; `fnd_sig-feat-test-suite-959129f6fd-1_b556c35f71` | Open        |
| [#4280 — peel gitleaks tags and test converter decisions](https://github.com/eddacraft/anvil-001/issues/4280) | P2 · ready in its authority lane · needs APS | `fnd_sig-feat-config-2c88b20c96-038f4_2b0b02cec8`; `fnd_sig-feat-test-suite-c427b9bb24-b_a97d41e5c5` | Open        |
| [#4281 — replace warm-up timing with deterministic proof](https://github.com/eddacraft/anvil-001/issues/4281) | P2 · needs design · needs APS                | `fnd_sig-feat-test-suite-36065aa8f7-c_2b7c75ead5`                                                    | Open        |
| [#4282 — exercise vendored tier-1 test contracts](https://github.com/eddacraft/anvil-001/issues/4282)         | P3 · ready · small-fix lane                  | `fnd_sig-feat-test-suite-7eb9de965f-5_ad64c99cf0`; `fnd_sig-feat-test-suite-7eb9de965f-5_fc14394c77` | Open        |

### Request and process bounds

[`GET /auth/logout`](../../apps/docs-shell/app/auth/logout/route.ts) mutates the
session cookie, so #4230 owns the same-origin, non-GET contract decision.

[`request_daemon_diagnostics`](../../crates/anvil-cli/src/mcp/validation.rs) has
no whole-exchange deadline, while
[`git_recent_files`](../../crates/anvil-cli/src/services/sample_analyser.rs)
drains piped output only after the child exits. #4231 owns the shared
bounded-I/O invariant.

[`read_blobs_batch`](../../crates/anvil-cli/src/graph_base_producer.rs) buffers
the complete `git cat-file --batch` transcript before enforcing its per-blob
limit. #4232 owns the aggregate memory-bound design.

### Fixture ownership

[`secret_streaming.rs`](../../crates/anvil-checks/tests/secret_streaming.rs) and
[`secret_coverage_surfaces.rs`](../../crates/anvil-cli/tests/secret_coverage_surfaces.rs)
rely on manual or absent cleanup. #4233 owns conversion to the existing
`TempDir` lifetime pattern.

### Vendored-rules refresh and tests

[`refresh-gitleaks-ruleset.sh`](../../scripts/secret/refresh-gitleaks-ruleset.sh)
uses the ref object's SHA directly, so an annotated tag supplies a tag-object
SHA where the raw-content URL needs a commit. The converter tests also stop at
helper return values instead of proving the `convert` rejection and
`secret_group` decisions. #4280 owns both residuals at the same refresh
boundary.

[`pattern_warm.rs`](../../crates/anvil-intercept/tests/pattern_warm.rs) uses a 5
ms wall-clock threshold as the correctness oracle after service construction.
Scheduler pre-emption can therefore fail a correctly warmed process. #4281 has a
design checkpoint for deterministic observation while preserving the single-test
binary.

[`secret_vendored_tier1.rs`](../../crates/anvil-checks/tests/secret_vendored_tier1.rs)
states that every credential-shaped canary is assembled at runtime but embeds
one canonical AWS example as a literal. Its shape-allowlist test also checks the
matcher and high-confidence flags separately without exercising a vendored scan
or suppression provenance. #4282 owns the paired test-only repair.

## Closed calibration

`fnd_sig-feat-test-suite-f53827d8bc-e_ddafcd834c` is fixed: the helper in
[`secret_file_coverage.rs`](../../crates/anvil-checks/tests/secret_file_coverage.rs)
returns `tempfile::TempDir`, retains ownership for the test lifetime, and has an
unwind-cleanup regression test. The implementation landed in CLAWOPEN-008
through [PR #4216](https://github.com/eddacraft/anvil-001/pull/4216).

`fnd_sig-feat-test-suite-78062855d2-d_114aaf17ab` is `wont-fix` at the current
boundary.
[`GitCommitNonEvaluation`](../../crates/anvil-checks/src/conformance/evaluate.rs)
has no `detail` field, and `pr_git_footprint_non_evaluations` copies only
commit, reason, stage, counts, digest, and bounded diagnostics. The fixture's
raw detail cannot cross without an explicit type and contract change, so another
assertion would not close a reachable leak.

## Receipt and duplicate checks

- Read back all thirteen selected Clawpatch records after disposition; eleven
  are open with issue owners, one is fixed, and one is `wont-fix`.
- Re-ran `clawpatch status --json`; the final snapshot is 995 findings, 517
  open, and no active lock or lock file.
- Searched open and closed GitHub issues and pull requests by finding ID, cited
  symbol/path, allegation, and PR #4234 review threads. No existing open owner
  matched the seven filed clusters.
- Read back all seven finding issues, including priority, kind, readiness, and
  tracking labels.
- The latest report's three action clusters contain six older records already
  routed by #4231–#4233; no duplicate issue was filed for them.
