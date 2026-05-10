# CICD Validation Council Review

Date: 2026-05-10

Target:

- `plans/specs/2026-05-10-ci-cd-validation-operating-model.md`
- `plans/modules/ci-cd-validation.aps.md`
- `plans/index.aps.md`
- `plans/modules/operating-model-migration.aps.md`
- `scripts/ci/cost-report.sh`
- `scripts/ci/cost-report.test.sh`
- `scripts/ci/classify-changes.sh`
- `scripts/ci/classify-changes.test.sh`
- `scripts/validate/local.sh`
- `scripts/validate/local.test.sh`
- `.github/workflows/ci-cost-report.yml`
- `package.json`

## Status

**Converged.** No open critical, major, or minor findings remain from the review.

The council agreed that the overall CI/CD validation plan is acceptable and fits
the broader Plan / Build / Release operating model. The `CICD` module is the
right specialist execution module and should coordinate with `OPMODEL` rather
than being folded into it.

## Review Pack

The review used role-specific council prompts for:

- Operations
- Pragmatic Lead
- Adversarial
- Security

The local task runner did not accept the named Council subagent labels in this
session, so the review was executed through general agents with explicit
role-specific prompts.

## Initial Findings

The first pass found no critical issues. It did find major issues around
operator trust in the initial `CICD-001` implementation:

- The report used “cost/minutes” language while measuring elapsed workflow time.
- Scheduled runs did not collect job-level timings by default.
- `CICD-001` over-claimed full runner-cost, path/risk, matrix, coverage, and
  security-spend observability.
- Manual `--limit` was unbounded when `--jobs` could call `gh run view` once per
  sampled run.
- Per-run `gh run view` failures could abort the whole report.
- Classifier labels/manual inputs needed an explicit trust model.
- Checkout persisted credentials unnecessarily in the read-only report workflow.
- Markdown report output needed table escaping for GitHub-controlled names.

## Resolutions

The implementation and plan were updated to resolve the findings:

- Report output now labels workflow/event/branch totals as elapsed wall-clock
  minutes.
- JSON output includes `measurementModel` explaining what is and is not measured.
- Scheduled reports collect job timings by default.
- Script limits are capped, with a stricter cap for `--jobs`.
- Run IDs from input files are validated before `gh run view` calls.
- `gh run view` stderr is separated from JSON stdout.
- Per-run `gh` and job-extraction failures are recorded as omitted-run
  diagnostics instead of aborting the report.
- Checkout sets `persist-credentials: false`.
- Markdown report tables escape table metacharacters.
- The spec states labels/manual inputs may escalate validation or document an
  audited override, but must not silently downgrade the path/SHA-derived baseline
  classification.
- `CICD-001` wording now describes this as baseline elapsed/job timing
  observability, with full runner-cost attribution left for later CICD work.

## Final Verdict

Final focused re-review results:

- Operations: no open findings; prior major concerns resolved.
- Adversarial: no open findings; `gh` error handling and JSON measurement model
  acceptable.
- Security: no open findings; token permissions, checkout credential handling,
  run ID validation, classifier trust semantics, and Markdown escaping acceptable.
- Pragmatic: no blocking findings; confirmed `CICD` is the right specialist
  module shape and should not replace `OPMODEL`.

## Follow-Up Review For CICD-002/CICD-003

After `CICD-002` and `CICD-003` were added, a standard Council pass reviewed the
classifier and local validation command surface. It found major gaps around
fail-closed classification and command-plan fidelity:

- Unclassified paths failed open, especially when mixed with recognised paths.
- Local validation silently ignored unsupported required checks.
- Empty command plans could serialise as a blank command.
- `validate:full` was not a superset of the local deterministic suite.
- `bash -n file1 file2` checked only the first shell script.
- `cost-report --input --jobs` could exceed the requested sample limit.
- The current-window OPMODEL progress row drifted from the module/index state.

The implementation was updated to resolve those findings:

- Classifier matching is now per-path; mixed known/unknown changes add `unknown`,
  `unclassified-paths`, fail-closed checks, and Operations review.
- Empty path sets remain an intentional no-op with `no-changed-paths` and no
  selected commands.
- Shell automation paths are explicitly classified and require shell syntax plus
  script fixture tests.
- Local validation fails on unsupported required checks instead of dropping them.
- Shell syntax validation loops over each script explicitly.
- `validate:full` includes the classifier, cost-report, local-validation, Rust,
  OPA, and Regal validation surfaces in addition to the standard package checks.
- Cost-report input mode applies `--limit` before any optional per-job expansion.
- APS current-window OPMODEL progress was aligned to `8/12`.
- Final focused re-review converged after shell syntax validation was changed to
  check the actual classified shell paths and `--paths-file` accepted readable
  streams such as `/dev/null`.

## Evidence

Validation commands run during the review/fix cycle:

- `pnpm ci:cost -- --limit 2 --jobs`
- `bash scripts/ci/cost-report.sh --limit 3 --json`
- `bash -n scripts/ci/cost-report.sh`
- `pnpm test:ci-classify`
- `pnpm test:ci-cost`
- `pnpm test:validate-local`
- `pnpm validate:changed -- --dry-run --json`
- `pnpm validate:full -- --dry-run --json`
- Per-script `bash -n` loop over `scripts/ci/*.sh` and `scripts/validate/*.sh`
- `pnpm validate:changed -- --paths-file /dev/null --dry-run --json`
- `pnpm format:check`
- `pnpm lint:md`
- `git diff --check`

Full documentation validation should be rerun after this review file is added.
