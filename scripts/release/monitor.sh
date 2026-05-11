#!/usr/bin/env bash
set -euo pipefail

COMMAND="monitor"; PHASE="monitor"; SCHEMA_VERSION="1.0.0"; DEFAULT_REPO="eddacraft/anvil-001"
json=false; poll=false; repo="$DEFAULT_REPO"; version=""; run_url=""; mode="target"; started_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

usage() { printf '%s\n' 'Usage: monitor.sh --version <vX.Y.Z[-suffix]> [--json] [--poll] [--run-url <url>]'; }
now() { date -u +%Y-%m-%dT%H:%M:%SZ; }
failure_json() { node - "$1" "$2" "$3" "$4" <<'NODE'
const [code, message, retryableRaw, recovery] = process.argv.slice(2);
process.stdout.write(JSON.stringify([{ code, message, retryable: retryableRaw === 'true', recovery, evidence: { command: 'scripts/release/monitor.sh', url: null, path: null } }]));
NODE
}
emit() { local status="$1" data="$2" failures="$3" next="$4" reason="$5" ended; ended="$(now)"; node - "$SCHEMA_VERSION" "$COMMAND" "$PHASE" "$status" "$started_at" "$ended" "$repo" "$mode" "$version" "$data" "$failures" "$next" "$reason" <<'NODE'
const [schemaVersion, command, phase, status, startedAt, endedAt, repository, mode, version, dataRaw, failuresRaw, nextCommand, nextReason] = process.argv.slice(2);
process.stdout.write(JSON.stringify({ schemaVersion, command, phase, mode, status, startedAt, endedAt, repository, inputs: { base: null, head: null, version }, trackingIssue: { repository, number: null, url: null, metadataCommentUrl: null }, releaseRecord: { lifecycleState: status === 'success' ? 'published' : null, recordUrl: null, sha256: null }, data: JSON.parse(dataRaw), warnings: [], failures: JSON.parse(failuresRaw), next: { command: nextCommand, reason: nextReason } }) + '\n');
NODE
}
fail_usage() { if [[ "$json" == true ]]; then emit failed '{"workflowRun":null,"state":null,"failedJob":null,"logUrl":null}' "$(failure_json invalid-input "$1" false correct-usage)" monitor 'Fix command arguments.'; else usage >&2; fi; exit 129; }
while (($# > 0)); do case "$1" in --json) json=true; shift;; --poll) poll=true; shift;; --version) version="${2:-}"; shift 2;; --run-url) run_url="${2:-}"; shift 2;; --repo) repo="${2:-}"; shift 2;; -h|--help) usage; exit 0;; *) fail_usage "unknown argument: $1";; esac; done
[[ -n "$version" ]] || fail_usage '--version is required'
[[ "$version" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[A-Za-z0-9._-]+)?$ ]] || fail_usage '--version must look like vX.Y.Z[-suffix]'
if [[ -n "${ANVIL_RELEASE_MONITOR_FAKE_RUN_FILE:-}" && "${ANVIL_RELEASE_TEST_MODE:-}" != monitor-fake-run ]]; then emit failed '{"workflowRun":null,"state":null,"failedJob":null,"logUrl":null}' "$(failure_json invalid-input 'ANVIL_RELEASE_MONITOR_FAKE_RUN_FILE requires ANVIL_RELEASE_TEST_MODE=monitor-fake-run' false correct-test-usage)" monitor 'Unset test hook.'; exit 129; fi
if [[ -n "${ANVIL_RELEASE_MONITOR_FAKE_RUN_FILE:-}" ]]; then
  result="$(node - "$ANVIL_RELEASE_MONITOR_FAKE_RUN_FILE" "$version" <<'NODE'
const fs = require('node:fs'); const [path, version] = process.argv.slice(2); const run = JSON.parse(fs.readFileSync(path, 'utf8'));
const ok = run.status === 'completed' && run.conclusion === 'success'; const failed = run.status === 'completed' && run.conclusion !== 'success';
process.stdout.write(JSON.stringify({ status: ok ? 'success' : failed ? 'failed' : 'blocked', exitCode: ok ? 0 : failed ? 1 : 1, data: { workflowRun: run.url, state: ok ? 'passed' : failed ? 'failed' : run.status, failedJob: run.failedJob || null, logUrl: run.logUrl || null, version }, failures: failed ? [{ code: 'artifact-build-failed', message: 'release workflow failed', retryable: true, recovery: 'retry-workflow', evidence: { command: 'gh run view', url: run.url, path: null } }] : [] }));
NODE
)"
  status="$(node -e "const r=JSON.parse(process.argv[1]);process.stdout.write(r.status)" "$result")"; code="$(node -e "const r=JSON.parse(process.argv[1]);process.stdout.write(String(r.exitCode))" "$result")"; data="$(node -e "const r=JSON.parse(process.argv[1]);process.stdout.write(JSON.stringify(r.data))" "$result")"; fails="$(node -e "const r=JSON.parse(process.argv[1]);process.stdout.write(JSON.stringify(r.failures))" "$result")"; emit "$status" "$data" "$fails" verify 'Workflow reached terminal state or needs recovery.'; exit "$code"
fi
emit blocked '{"workflowRun":null,"state":"operator-required","failedJob":null,"logUrl":null}' "$(failure_json operator-required 'live workflow monitoring requires gh run integration or --run-url fake evidence' true provide-run-evidence)" monitor 'Provide workflow evidence or complete live monitor integration.'; exit 1
