# SARIF → GitHub Code Scanning Upload Runbook

| Type    | Authority | Owner    | Status | Freshness                                                       |
| ------- | --------- | -------- | ------ | --------------------------------------------------------------- |
| Runbook | Advisory  | SARIFOUT | Live   | Created 2026-05-29 for SARIFOUT-006 against the SARIFOUT module |

| Upstream                                                                                               | Downstream                                    |
| ------------------------------------------------------------------------------------------------------ | --------------------------------------------- |
| `crates/anvil-cli/src/output/sarif.rs`, `plans/decisions/058-sarif-shared-emitter-no-finding-model.md` | Operators validating `anvil … --format sarif` |

## What this is

`anvil check`, `anvil gate`, and `anvil audit` can emit SARIF 2.1.0 with
`--format sarif` (SARIFOUT). This runbook records the **manual, out-of-band**
smoke check that the emitted SARIF is accepted by GitHub Code Scanning and that
findings render in the Security tab.

It is deliberately **not** a CI test: uploading to Code Scanning depends on
network access and GitHub's ingest behaviour, which are non-deterministic. The
in-repo guarantee is the schema-validation test against the bundled
`crates/anvil-cli/src/output/sarif-schema-2.1.0.json`; this runbook is the
end-to-end confidence check on top of it.

## Prerequisites

- A throwaway **sandbox** GitHub repository you control (do not upload findings
  from a real project to an unrelated repo).
- `gh` authenticated with `security_events` scope, or a workflow using
  `github/codeql-action/upload-sarif`.
- A locally built `anvil` binary.

## Procedure

1. Emit SARIF from a finding-emitting command into a file:

   ```bash
   anvil check --all --format sarif > anvil.sarif
   # or: anvil audit --format sarif > anvil.sarif
   # or: anvil gate  --format sarif > anvil.sarif
   ```

   `--format sarif` is exit-code-neutral for `check`/`gate` (a blocking finding
   still exits non-zero), so capture stdout explicitly and don't gate the upload
   on the exit code.

2. Sanity-check the document locally before upload:

   ```bash
   jq '.version, .runs[0].tool.driver.name' anvil.sarif
   # → "2.1.0"
   # → "anvil"
   ```

3. Upload to the sandbox repo's Code Scanning. Either:

   **Via `gh` (gzip + base64, per the Code Scanning API):**

   ```bash
   gh api -X POST /repos/<owner>/<sandbox>/code-scanning/sarifs \
     -f commit_sha="$(git -C <sandbox-checkout> rev-parse HEAD)" \
     -f ref="refs/heads/main" \
     -f sarif="$(gzip -c anvil.sarif | base64 -w0)"
   ```

   (`base64 -w0` is GNU; on BSD/macOS use `base64` with no `-w` flag.)

   **Or via a workflow** committing `anvil.sarif` and running
   `github/codeql-action/upload-sarif@v3` with `sarif_file: anvil.sarif`.

4. In the sandbox repo's **Security → Code scanning** tab, confirm:
   - the upload processed without a SARIF-validation error;
   - results appear with the expected `ruleId`s (e.g. `AP-*`, `SECRET-*`, audit
     categories, gate check names);
   - `@anvil-ignore`-suppressed `check` findings show as **dismissed /
     suppressed**, not as open alerts;
   - gate results (repo-level, no `region`) — note whether Code Scanning
     surfaces location-less results (a known limitation; see below).

## Known limitation

`anvil gate` findings are per-check aggregates with **no physical location**, so
their SARIF results omit `locations[]`. GitHub Code Scanning may not surface
results without a location. `check` and `audit` results carry file/line
locations and render normally. Record the observed behaviour below.

## Verification record

Fill in when the manual check is performed (leave blank until then — do not
backfill):

| Date | Operator | Sandbox repo | Command(s) | Result |
| ---- | -------- | ------------ | ---------- | ------ |
|      |          |              |            |        |
