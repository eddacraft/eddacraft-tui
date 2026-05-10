# CICD Validation Council Review

Date: 2026-05-10

Target:

- `plans/specs/2026-05-10-ci-cd-validation-operating-model.md`
- `plans/modules/ci-cd-validation.aps.md`
- `plans/index.aps.md`
- `plans/modules/operating-model-migration.aps.md`
- `scripts/ci/cost-report.sh`
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

## Evidence

Validation commands run during the review/fix cycle:

- `pnpm ci:cost -- --limit 2 --jobs`
- `bash scripts/ci/cost-report.sh --limit 3 --json`
- `bash -n scripts/ci/cost-report.sh`

Full documentation validation should be rerun after this review file is added.
