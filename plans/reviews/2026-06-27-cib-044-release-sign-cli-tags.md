# CIB-044 Review — Release Signing CLI Tag Gate

| Type   | Authority | Owner | Status |
| ------ | --------- | ----- | ------ |
| Review | Advisory  | CIB   | Closed |

## Scope

CIB-044 prevents `.github/workflows/release-sign-artefacts.yml` from running on
prefixed non-CLI library releases such as `eddacraft-tui-v*`.

Changed surfaces:

- `.github/workflows/release-sign-artefacts.yml`
- `.github/workflows/README.md`
- `scripts/ci/release-sign-artefacts-workflow.test.sh`
- `plans/modules/continuous-improvement-backlog.aps.md`
- `plans/index.aps.md`

## Implemented behaviour

- Manual `workflow_dispatch` remains allowed for explicit operator reruns.
- Automatic `release: published` signing now requires both:
  - the release is not a prerelease; and
  - the release tag starts with the CLI convention `v`.
- Prefixed library tags such as `eddacraft-tui-v*` no longer satisfy the job gate.
- Workflow README documents that the signer is CLI-tag-only for automatic release
  events.

## Validation evidence

RED evidence:

```text
scripts/ci/release-sign-artefacts-workflow.test.sh
```

The new test failed before the workflow edit because the workflow did not contain
`!github.event.release.prerelease &&`.

Final validation passed:

```text
scripts/ci/release-sign-artefacts-workflow.test.sh
scripts/ci/workflow-contracts.test.sh
```

`workflow-contracts.test.sh` reported `workflow-contract map checks passed`.
