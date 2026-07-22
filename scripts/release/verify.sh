#!/usr/bin/env bash
set -euo pipefail

COMMAND="verify"; PHASE="verify"; SCHEMA_VERSION="1.0.0"; DEFAULT_REPO="eddacraft/anvil-001"
json=false; repo="$DEFAULT_REPO"; public_repo="eddacraft/anvil"; version=""; source_sha=""; mode="target"; started_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
usage() { printf '%s\n' 'Usage: verify.sh --version <vX.Y.Z[-suffix]> --source-sha <sha> [--json]'; }
now() { date -u +%Y-%m-%dT%H:%M:%SZ; }
failure_json() { node - "$1" "$2" "$3" "$4" <<'NODE'
const [code, message, retryableRaw, recovery] = process.argv.slice(2); process.stdout.write(JSON.stringify([{ code, message, retryable: retryableRaw === 'true', recovery, evidence: { command: 'scripts/release/verify.sh', url: null, path: null } }]));
NODE
}
emit() { local status="$1" data="$2" failures="$3" next="$4" reason="$5" ended; ended="$(now)"; node - "$SCHEMA_VERSION" "$COMMAND" "$PHASE" "$status" "$started_at" "$ended" "$repo" "$mode" "$version" "$source_sha" "$data" "$failures" "$next" "$reason" <<'NODE'
const [schemaVersion, command, phase, status, startedAt, endedAt, repository, mode, version, sourceSha, dataRaw, failuresRaw, nextCommand, nextReason] = process.argv.slice(2);
const data = JSON.parse(dataRaw);
process.stdout.write(JSON.stringify({ schemaVersion, command, phase, mode, status, startedAt, endedAt, repository, inputs: { base: null, head: null, version, sourceSha }, trackingIssue: { repository, number: null, url: null, metadataCommentUrl: null }, releaseRecord: { lifecycleState: status === 'success' ? 'published' : null, recordUrl: data.releaseRecordUrl || null, sha256: data.releaseRecordSha256 || null }, data, warnings: [], failures: JSON.parse(failuresRaw), next: { command: nextCommand, reason: nextReason } }) + '\n');
NODE
}
fail_usage() { if [[ "$json" == true ]]; then emit failed '{"checks":[],"releaseRecordUrl":null,"releaseRecordSha256":null,"commsDraft":null}' "$(failure_json invalid-input "$1" false correct-usage)" verify 'Fix command arguments.'; else usage >&2; fi; exit 129; }
while (($# > 0)); do case "$1" in --json) json=true; shift;; --version) version="${2:-}"; shift 2;; --source-sha) source_sha="${2:-}"; shift 2;; --repo) repo="${2:-}"; shift 2;; --public-repo) public_repo="${2:-}"; shift 2;; -h|--help) usage; exit 0;; *) fail_usage "unknown argument: $1";; esac; done
[[ -n "$version" ]] || fail_usage '--version is required'; [[ "$version" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[A-Za-z0-9-]+(\.[A-Za-z0-9-]+)*)?$ ]] || fail_usage '--version must look like vX.Y.Z[-suffix]'; [[ -n "$source_sha" ]] || fail_usage '--source-sha is required'; [[ "$source_sha" =~ ^[0-9a-fA-F]{40}$ ]] || fail_usage '--source-sha requires a full 40-character commit SHA'
if [[ -n "${ANVIL_RELEASE_VERIFY_FAKE_REPORT_FILE:-}" && "${ANVIL_RELEASE_TEST_MODE:-}" != verify-fake-report ]]; then emit failed '{"checks":[],"releaseRecordUrl":null,"releaseRecordSha256":null,"commsDraft":null}' "$(failure_json invalid-input 'ANVIL_RELEASE_VERIFY_FAKE_REPORT_FILE requires ANVIL_RELEASE_TEST_MODE=verify-fake-report' false correct-test-usage)" verify 'Unset test hook.'; exit 129; fi
if [[ -n "${ANVIL_RELEASE_VERIFY_FAKE_REPORT_FILE:-}" ]]; then
  result="$(node - "$ANVIL_RELEASE_VERIFY_FAKE_REPORT_FILE" "$version" "$source_sha" <<'NODE'
const fs = require('node:fs'); const [path, version, sourceSha] = process.argv.slice(2); const report = JSON.parse(fs.readFileSync(path, 'utf8')); const failed = (report.checks || []).filter((check) => check.status !== 'pass'); const mismatches = [];
if (report.version && report.version !== version) mismatches.push({ name: 'version binding', code: 'integrity-failed', url: null });
if (report.sourceSha && report.sourceSha !== sourceSha) mismatches.push({ name: 'source SHA binding', code: 'integrity-failed', url: null });
const failures = failed.concat(mismatches);
process.stdout.write(JSON.stringify({ status: failures.length ? 'failed' : 'success', exitCode: failures.length ? 1 : 0, data: report, failures: failures.map((check) => ({ code: check.code || 'integrity-failed', message: `${check.name} failed`, retryable: true, recovery: 'fix-and-rerun-verify', evidence: { command: 'verify fake report', url: check.url || null, path: null } })) }));
NODE
)"
  status="$(node -e "const r=JSON.parse(process.argv[1]);process.stdout.write(r.status)" "$result")"; code="$(node -e "const r=JSON.parse(process.argv[1]);process.stdout.write(String(r.exitCode))" "$result")"; data="$(node -e "const r=JSON.parse(process.argv[1]);process.stdout.write(JSON.stringify(r.data))" "$result")"; fails="$(node -e "const r=JSON.parse(process.argv[1]);process.stdout.write(JSON.stringify(r.failures))" "$result")"; emit "$status" "$data" "$fails" closeout 'Verification complete; proceed to closeout if successful.'; exit "$code"
fi
emit blocked '{"checks":[],"releaseRecordUrl":null,"releaseRecordSha256":null,"commsDraft":null}' "$(failure_json operator-required 'live verification requires release host and publisher checks' true provide-verification-report)" verify 'Provide fake verification report or complete live host checks.'; exit 1
