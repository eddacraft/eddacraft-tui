#!/usr/bin/env bash
set -euo pipefail

now() {
  date -u +%Y-%m-%dT%H:%M:%SZ
}

emit_success() {
  local command_name="${1:-assess}"
  local phase="${2:-assessment}"
  local extra_data="{}"
  if (($# >= 3)); then
    extra_data="$3"
  fi
  node - "$command_name" "$phase" "$(now)" "$extra_data" <<'NODE'
const [command, phase, timestamp, extraDataRaw] = process.argv.slice(2);
const extraData = JSON.parse(extraDataRaw);
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
  trackingIssue: null,
  releaseRecord: null,
  data: extraData,
  warnings: [],
  failures: [],
  next: null,
}) + '\n');
NODE
}

emit_missing_failure() {
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
  trackingIssue: null,
  releaseRecord: null,
  data: { failedGateCount: 3 },
  warnings: [],
  failures: [{
    code: 'validation-failed',
    message: 'two gates failed',
    retryable: true,
    recovery: 'fix-and-rerun',
    evidence: { command: 'fixture preflight', url: null, path: null },
  }],
  next: null,
}) + '\n');
NODE
  exit 2
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
  non-json)
    printf '%s\n' 'human output before json'
    ;;
  missing-failure)
    emit_missing_failure
    ;;
  killable)
    run_killable "${2:?state file required}"
    ;;
  *)
    echo "unknown fixture scenario: ${1:-}" >&2
    exit 129
    ;;
esac
