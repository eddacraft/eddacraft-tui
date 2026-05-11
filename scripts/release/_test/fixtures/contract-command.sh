#!/usr/bin/env bash
set -euo pipefail

now() {
  date -u +%Y-%m-%dT%H:%M:%SZ
}

emit_success() {
  local command_name="${1:-assess}"
  local phase="${2:-assessment}"
  local extra_data="{}"
  local release_record='{"lifecycleState":"candidate","recordUrl":null,"sha256":null}'
  if (($# >= 3)); then
    extra_data="$3"
  fi
  if (($# >= 4)); then
    release_record="$4"
  fi
  node - "$command_name" "$phase" "$(now)" "$extra_data" "$release_record" <<'NODE'
const [command, phase, timestamp, extraDataRaw, releaseRecordRaw] = process.argv.slice(2);
const extraData = JSON.parse(extraDataRaw);
const releaseRecord = JSON.parse(releaseRecordRaw);
process.stdout.write(JSON.stringify({
  schemaVersion: '1.0.0',
  command,
  phase,
  mode: 'compatibility',
  status: 'success',
  startedAt: timestamp,
  endedAt: timestamp,
  repository: 'eddacraft/anvil-001',
  inputs: {},
  trackingIssue: {
    repository: 'eddacraft/anvil-001',
    number: 1234,
    url: 'https://github.com/eddacraft/anvil-001/issues/1234',
    metadataCommentUrl: 'https://github.com/eddacraft/anvil-001/issues/1234#issuecomment-1',
  },
  releaseRecord,
  data: extraData,
  warnings: [],
  failures: [],
  next: {
    command: 'preflight',
    reason: 'fixture next command',
  },
}) + '\n');
NODE
}

emit_failure() {
  local command_name="$1"
  local phase="$2"
  local status="$3"
  local failure_code="$4"
  local message="$5"
  local exit_code="$6"
  local extra_data="{}"
  local release_record='{"lifecycleState":"candidate","recordUrl":null,"sha256":null}'
  if (($# >= 7)); then
    extra_data="$7"
  fi
  if (($# >= 8)); then
    release_record="$8"
  fi

  node - "$command_name" "$phase" "$status" "$failure_code" "$message" "$(now)" "$extra_data" "$release_record" <<'NODE'
const [command, phase, status, failureCode, message, timestamp, extraDataRaw, releaseRecordRaw] = process.argv.slice(2);
const extraData = JSON.parse(extraDataRaw);
const releaseRecord = JSON.parse(releaseRecordRaw);
process.stdout.write(JSON.stringify({
  schemaVersion: '1.0.0',
  command,
  phase,
  mode: 'target',
  status,
  startedAt: timestamp,
  endedAt: timestamp,
  repository: 'eddacraft/anvil-001',
  inputs: {},
  trackingIssue: {
    repository: 'eddacraft/anvil-001',
    number: 1234,
    url: 'https://github.com/eddacraft/anvil-001/issues/1234',
    metadataCommentUrl: 'https://github.com/eddacraft/anvil-001/issues/1234#issuecomment-1',
  },
  releaseRecord,
  data: extraData,
  warnings: [],
  failures: [{
    code: failureCode,
    message,
    retryable: status === 'recoverable',
    recovery: status === 'recoverable' ? 'recover-and-rerun' : 'operator-decision-required',
    evidence: { command: `fixture ${command}`, url: null, path: null },
  }],
  next: {
    command,
    reason: 'fixture recovery path',
  },
}) + '\n');
NODE
  exit "$exit_code"
}

emit_failed_gate_mismatch() {
  node - "$(now)" <<'NODE'
const timestamp = process.argv[2];
process.stdout.write(JSON.stringify({
  schemaVersion: '1.0.0',
  command: 'preflight',
  phase: 'preflight',
  mode: 'compatibility',
  status: 'failed',
  startedAt: timestamp,
  endedAt: timestamp,
  repository: 'eddacraft/anvil-001',
  inputs: {},
  trackingIssue: {
    repository: 'eddacraft/anvil-001',
    number: 1234,
    url: 'https://github.com/eddacraft/anvil-001/issues/1234',
    metadataCommentUrl: 'https://github.com/eddacraft/anvil-001/issues/1234#issuecomment-1',
  },
  releaseRecord: {
    lifecycleState: 'candidate',
    recordUrl: null,
    sha256: null,
  },
  data: { failedGateCount: 3 },
  warnings: [],
  failures: [{
    code: 'validation-failed',
    message: 'two gates failed',
    retryable: true,
    recovery: 'fix-and-rerun',
    evidence: { command: 'fixture preflight', url: null, path: null },
  }],
  next: {
    command: 'preflight',
    reason: 'fixture next command',
  },
}) + '\n');
NODE
  exit 2
}

emit_invalid_failure_code() {
  node - "$(now)" <<'NODE'
const timestamp = process.argv[2];
process.stdout.write(JSON.stringify({
  schemaVersion: '1.0.0',
  command: 'preflight',
  phase: 'preflight',
  mode: 'compatibility',
  status: 'failed',
  startedAt: timestamp,
  endedAt: timestamp,
  repository: 'eddacraft/anvil-001',
  inputs: {},
  trackingIssue: {
    repository: 'eddacraft/anvil-001',
    number: 1234,
    url: 'https://github.com/eddacraft/anvil-001/issues/1234',
    metadataCommentUrl: 'https://github.com/eddacraft/anvil-001/issues/1234#issuecomment-1',
  },
  releaseRecord: {
    lifecycleState: 'candidate',
    recordUrl: null,
    sha256: null,
  },
  data: { failedGateCount: 1 },
  warnings: [],
  failures: [{
    code: 'tool-unavailable',
    message: 'fixture uses an out-of-schema failure code',
    retryable: true,
    recovery: 'fix-and-rerun',
    evidence: { command: 'fixture preflight', url: null, path: null },
  }],
  next: {
    command: 'preflight',
    reason: 'fixture next command',
  },
}) + '\n');
NODE
  exit 1
}

run_killable() {
  local state_file="$1"
  if [[ -f "$state_file" ]]; then
    emit_success prepare prepare '{"rerunAfterKill":true}'
    return 0
  fi

  printf '%s\n' started > "$state_file"
  while true; do
    sleep 1
  done
}

case "${1:-}" in
  success)
    emit_success assess assessment
    ;;
  metadata-comment)
    emit_success prepare prepare \
      '{"metadataComment":{"url":"https://github.com/eddacraft/anvil-001/issues/1234#issuecomment-1","phase":"prepare"}}'
    ;;
  remote-tag-recovery)
    emit_failure tag tag recoverable remote-conflict 'remote tag exists at an unexpected sha' 1 \
      '{"remoteTag":{"tag":"v0.6.1-beta","expectedSha":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","actualSha":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}}'
    ;;
  release-record-mismatch)
    emit_failure verify verify failed contract-drift 'release record sha does not match tagged source' 1 \
      '{"expectedSha":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","recordSha":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}' \
      '{"lifecycleState":"published","recordUrl":"https://github.com/eddacraft/anvil-001/releases/download/v0.6.1-beta/release-record.json","sha256":"abc123","policyDecisions":[]}'
    ;;
  cargo-dist-failure)
    emit_failure monitor monitor failed artifact-build-failed 'cargo-dist workflow failed' 1 \
      '{"workflowRun":"https://github.com/eddacraft/anvil-001/actions/runs/123","failedJob":"publish"}'
    ;;
  non-json)
    printf '%s\n' 'human output before json'
    ;;
  failed-gate-mismatch)
    emit_failed_gate_mismatch
    ;;
  invalid-failure-code)
    emit_invalid_failure_code
    ;;
  killable)
    run_killable "${2:?state file required}"
    ;;
  *)
    echo "unknown fixture scenario: ${1:-}" >&2
    exit 129
    ;;
esac
