# Clawpatch intake triage — 2026-09-04

| Type  | Authority | Owner  | Status | Freshness                                                                                                                                                                                                                                                                                        |
| ----- | --------- | ------ | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Guide | Advisory  | CLAW04 | Live   | Reviewed 2026-09-04 against the complete persisted Clawpatch store, runs `20260831T052858-acdf65`, `20260902T115230-44b5da`, and `20260904T074935-2c02a1`, filed GitHub issues, and current source under `apps/website/scripts/` and `scripts/ci/` at `99a49b5975bd8e7b5a535365c28e4b18b7bc7d16` |

| Upstream                                                                                                                                                            | Downstream                                                                                                                                                                                                                     |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Persisted Clawpatch finding/run records; current source and tests; [issue triage and APS authority](../../plans/specs/2026-05-28-issue-triage-and-aps-authority.md) | [CLAW04 intake receipt](../../plans/archive/modules/clawpatch-2026-09-04-intake.aps.md); GitHub issues [#4387](https://github.com/eddacraft/anvil-001/issues/4387)–[#4391](https://github.com/eddacraft/anvil-001/issues/4391) |

**Source paths reviewed:**

- `scripts/ci/pr-required-status.mjs`
- `scripts/ci/pr-required-status.test.mjs`
- `scripts/ci/hostile-ambient.sh`
- `scripts/ci/hostile-ambient.test.sh`
- `scripts/ci/public-reference-regen-workflow.test.sh`
- `scripts/ci/secret-calibration-workflow.test.sh`
- `apps/website/scripts/check-public-trust.mjs`
- `apps/website/scripts/check-public-trust.test.mjs`
- `.github/workflows/secret-calibration.yml`
- `.github/workflows/public-reference-regen.yml`

## Decision

The latest completed run is `20260904T074935-2c02a1`. Two earlier unprocessed
runs sat between the CLAW30 publication and this intake, so the complete store
was inventoried before selecting records. Across the bounded intake, twelve
records were calibrated:

- keep the eight 4 September records open and route them through five coherent
  GitHub issues;
- leave the two 2 September open records with their existing owner
  [#4342](https://github.com/eddacraft/anvil-001/issues/4342);
- leave the two 2 September false-positive records closed at that prior
  calibration;
- do not re-file the latest report's three action clusters, which repeat CLAW30
  owners
  [#4231](https://github.com/eddacraft/anvil-001/issues/4231)–[#4233](https://github.com/eddacraft/anvil-001/issues/4233);
- do not implement any finding in this intake PR;
- do not edit the shared CIB module or extend the bounded CLAWOPEN source set.

This document is review evidence, not a second backlog. The GitHub issues own
the filed follow-ups until a non-small item is promoted into APS.

## Corpus and scope

The complete persisted store was inventoried before latest-run selection. The
final read-back snapshot was:

| Metric                |                                      Value |
| --------------------- | -----------------------------------------: |
| Persisted findings    |                                       1007 |
| Open findings         |                                        527 |
| Active locks          |                                          0 |
| Lock files            |                                          0 |
| Latest run            |                   `20260904T074935-2c02a1` |
| Current-source anchor | `99a49b5975bd8e7b5a535365c28e4b18b7bc7d16` |

| Run                      | Selected records | Reason                                                                                |
| ------------------------ | ---------------: | ------------------------------------------------------------------------------------- |
| `20260831T052858-acdf65` |                0 | Completed with no claimed features and no findings                                    |
| `20260902T115230-44b5da` |                4 | Already calibrated on 2026-09-02; two remain open under #4342, two are false-positive |
| `20260904T074935-2c02a1` |                8 | Every new record from the latest completed run                                        |

The lifetime-open count is not a count of newly confirmed defects. This intake
selected run-created records and confirmed that the latest report's action
clusters already have owners.

## Filed clusters

| Issue                                                                                                                                | Priority and readiness                       | Findings                                                                                                                                                | Disposition |
| ------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------- |
| [#4387 — stop hostile-ambient daemon leak and bind the nightly contract](https://github.com/eddacraft/anvil-001/issues/4387)         | P2 · needs design · needs APS                | `fnd_sig-feat-config-8519458ccb-706cc_a675edfa19`; `fnd_sig-feat-config-075d0c93e5-d638b_aec846cba3`; `fnd_sig-feat-config-075d0c93e5-3f14f_302c46cc34` | Open        |
| [#4388 — cover valid YAML block-scalar headers in the interpolation scanner](https://github.com/eddacraft/anvil-001/issues/4388)     | P2 · needs design · needs APS                | `fnd_sig-feat-config-af802fda3c-d822b_0a7e694776`                                                                                                       | Open        |
| [#4389 — paginate PR required-status discovery](https://github.com/eddacraft/anvil-001/issues/4389)                                  | P2 · ready in its authority lane · needs APS | `fnd_sig-feat-library-0aad9458b7-52f0_eb02556fc2`; `fnd_sig-feat-library-0aad9458b7-9206_8beb6015d2`                                                    | Open        |
| [#4390 — validate published PGP key contents against the advertised fingerprint](https://github.com/eddacraft/anvil-001/issues/4390) | P2 · needs design · needs APS                | `fnd_sig-feat-library-4b97edc98b-a8b7_bb22a625cc`                                                                                                       | Open        |
| [#4391 — bind secret-calibration fork ternary to the job runs-on field](https://github.com/eddacraft/anvil-001/issues/4391)          | P3 · ready · small-fix lane                  | `fnd_sig-feat-config-c1d2c5388b-c9ef6_6370e823f4`                                                                                                       | Open        |

### Hostile-ambient lifetime and contract

[`hostile-ambient.sh`](../../scripts/ci/hostile-ambient.sh) starts a live
intercept daemon and then replaces the shell with `cargo test`. There is no trap
or ownership marker, so a successfully started daemon can survive the job.
[#4387](https://github.com/eddacraft/anvil-001/issues/4387) also owns the
contract-test gaps: a missing `ci.yml` fails open because `grep` sits inside
`if`, and the nightly job name and invocation are unrestricted literals. CIB-391
created the profile; it does not own these residuals.

### Interpolation-scanner lock

[`public-reference-regen-workflow.test.sh`](../../scripts/ci/public-reference-regen-workflow.test.sh)
recognises a multiline `run:` body only when the block-scalar header ends after
`|` or `>` plus an optional chomping flag. Valid headers such as `run: |2` or
`run: | # comment` take the single-line path and miss interpolated github
context on the following shell line. The current workflow still uses bare
`run: |`, so today's file is covered; the lock is incomplete. #4388 owns the
residual of CIB-395.

### Merge-gate pagination

[`unresolvedThreadCount`](../../scripts/ci/pr-required-status.mjs) requests
`reviewThreads(first:100)` with no `pageInfo`, and the live ruleset list uses
`gh api` without `--paginate`. Fixture tests inject JSON and never exercise
those APIs. #4389 owns both discovery pages. CIB-404 owns a sibling defect on
the same script (conflicting PRs reported as "not finished") and does not cover
pagination.

### Public-trust key contents

[`check-public-trust.mjs`](../../apps/website/scripts/check-public-trust.mjs)
passes `existsSync(publishedKey)` into the contract, so an empty or unrelated
`pgp-key.txt` satisfies the advertised-key check. #4390 owns fail-closed content
validation and fingerprint matching.

### Secret-calibration test lock

[`secret-calibration-workflow.test.sh`](../../scripts/ci/secret-calibration-workflow.test.sh)
requires the fork ternary somewhere in the file and never binds it to
`jobs.calibrate.runs-on`. The workflow itself already has the ternary on that
field via CIB-396. #4391 owns the residual one-file test lock in the small-fix
lane.

## Already-owned calibration

`fnd_sig-feat-config-4745cd854d-2afba_2db4d9b544` and
`fnd_sig-feat-config-f91d741363-334ed_6786566c49` remain open under #4342 (Nx
graph proof isolation). They were calibrated on 2026-09-02; current source has
not removed the defects.

`fnd_sig-feat-test-suite-49e7b6f419-8_17c0772b6f` and
`fnd_sig-feat-test-suite-158d8cd04d-b_cce5a63236` remain `false-positive`:
default-running coverage already owns generated record-ID suppression and
per-rule vendored canaries. The ignored 3 MiB dogfood scan and the prefix-gate
aggregate check are not the repository's completeness proofs.

The latest report's three action clusters contain six older records already
routed by #4231–#4233; no duplicate issue was filed for them.

## Receipt and duplicate checks

- Read back all eight 4 September Clawpatch records after disposition; each is
  open with an issue owner.
- Re-ran `clawpatch status --json`; the final snapshot is 1007 findings, 527
  open, and no active lock or lock file.
- Searched open and closed GitHub issues and pull requests by finding ID, cited
  symbol/path, allegation, CIB-391, CIB-395, CIB-396, and CIB-404. No existing
  open owner matched the five filed clusters. #4342 already owns the 2 September
  Nx residuals.
- Read back all five finding issues, including priority, kind, readiness, and
  tracking labels.
- Claim issue [#4386](https://github.com/eddacraft/anvil-001/issues/4386)
  records CLAW04-001.
