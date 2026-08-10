---
id: github
title: Add anvil to GitHub Actions
description:
  Run a version-pinned anvil pull-request gate and optionally upload SARIF
  findings.
---

# Add anvil to GitHub Actions

**For:** repositories using GitHub Actions

**Time:** 15–30 minutes

**Outcome:** pull requests run a shared anvil gate

Pilot the same command locally before adding it to CI.

## 1. Store beta credentials

CI needs a dedicated beta licence supplied by your beta programme or team
administrator. The personal credential created by `anvil auth login` has no
public export command. If you have not been given a CI licence, stop here rather
than copying local credential files.

Add the supplied value as an encrypted repository or organisation secret named
`ANVIL_LICENSE`. Never write the value into workflow YAML or logs.

## 2. Add the workflow

```yaml
name: anvil

on:
  pull_request:
  push:
    branches: [main]

permissions:
  contents: read

jobs:
  gate:
    runs-on: ubuntu-latest
    env:
      ANVIL_VERSION: 0.9.4-beta
    steps:
      - uses: actions/checkout@v4

      - name: Install anvil
        run: |
          curl --proto '=https' --tlsv1.2 -LsSf \
            "https://github.com/eddacraft/anvil/releases/download/v${ANVIL_VERSION}/eddacraft-anvil-installer.sh" | sh

      - name: Verify the pinned version
        run: test "$(anvil --version)" = "anvil ${ANVIL_VERSION}"

      - name: Run the CI gate
        env:
          ANVIL_LICENSE: ${{ secrets.ANVIL_LICENSE }}
        run: anvil gate --profile ci --format plain
```

The workflow pins anvil so its result cannot change without a reviewed workflow
edit. Upgrade by changing `ANVIL_VERSION`, reviewing the release notes, and
reproducing the gate locally before merging.

## 3. Reproduce locally

```text
anvil gate --profile ci --format plain
```

Success means the command and the Actions job both complete with the expected
gate verdict. Treat authentication, configuration, and tool failures separately
from product findings.

## Optional SARIF upload

SARIF is a standard analysis-result format understood by GitHub code scanning.
Add the following entries under `jobs.gate.steps`, immediately after the gate
step in the complete workflow above, and add `security-events: write` beside
`contents: read` under `permissions`.

```yaml
- name: Write SARIF
  if: always()
  id: write-sarif
  continue-on-error: true
  env:
    ANVIL_LICENSE: ${{ secrets.ANVIL_LICENSE }}
  run: anvil check --all --format sarif > anvil.sarif

- name: Validate SARIF
  if: always() && hashFiles('anvil.sarif') != ''
  id: validate-sarif
  continue-on-error: true
  run: jq -e '.version == "2.1.0" and (.runs | type == "array")' anvil.sarif

- name: Upload SARIF
  if: always() && steps.validate-sarif.outcome == 'success'
  continue-on-error: true
  uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: anvil.sarif
```

`anvil check` returns non-zero when it finds a blocking problem, so every SARIF
step is explicitly non-gating. The validation step prevents an empty or
malformed report from reaching the upload action. Keep the gate as the merge
decision; SARIF is only a reporting channel.

## Common problems

- **Authentication fails:** confirm the secret exists in the event context;
  secrets are normally unavailable to untrusted forked pull requests.
- **No CI licence is available:** ask the beta programme or team administrator;
  do not export a personal credential file.
- **The command is not found:** start a new shell step only after the installer
  has updated the runner path.
- **Local and CI results differ:** compare version, profile, commit,
  environment, and generated configuration.
- **Annotations are missing:** confirm the SARIF upload step ran and has
  `security-events: write`.

## Next step

Add branch protection only after the job is stable and the team agrees which
findings block.
