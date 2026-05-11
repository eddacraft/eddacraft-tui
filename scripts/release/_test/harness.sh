#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage:
  harness.sh run-contract --name <name> --expected-exit <n> --expected-command <command> -- <command...>
  harness.sh run-kill9-rerun --name <name> --state-file <path> -- <command...>
USAGE
}

fail() {
  echo "release harness: $*" >&2
  exit 1
}

require_node() {
  command -v node >/dev/null 2>&1 || fail "node is required for JSON contract validation"
}

parse_common_args() {
  name=""
  expected_exit=""
  expected_command=""
  state_file=""

  while (($# > 0)); do
    case "$1" in
      --name)
        name="${2:-}"
        shift 2
        ;;
      --expected-exit)
        expected_exit="${2:-}"
        shift 2
        ;;
      --expected-command)
        expected_command="${2:-}"
        shift 2
        ;;
      --state-file)
        state_file="${2:-}"
        shift 2
        ;;
      --)
        shift
        command_args=("$@")
        return 0
        ;;
      -h|--help)
        usage
        exit 0
        ;;
      *)
        fail "unknown argument: $1"
        ;;
    esac
  done

  fail "missing -- command separator"
}

validate_json_contract() {
  local stdout_file="$1"
  local rc="$2"
  local expected_rc="$3"
  local expected_cmd="$4"

  node - "$stdout_file" "$rc" "$expected_rc" "$expected_cmd" <<'NODE'
const fs = require('node:fs');

const [path, actualExitRaw, expectedExitRaw, expectedCommand] = process.argv.slice(2);
const raw = fs.readFileSync(path, 'utf8');
let doc;

try {
  doc = JSON.parse(raw);
} catch (error) {
  console.error(`stdout is not valid JSON: ${error.message}`);
  process.exit(1);
}

const actualExit = Number(actualExitRaw);
const expectedExit = Number(expectedExitRaw);
const failures = [];
const allowedModes = new Set(['compatibility', 'target', 'migration-exercise']);
const allowedStatuses = new Set(['success', 'noop', 'blocked', 'failed', 'recoverable', 'needs-operator']);
const allowedPhases = new Set(['assessment', 'preflight', 'prepare', 'promote', 'tag', 'monitor', 'verify', 'closeout']);
const allowedLifecycleStates = new Set(['candidate', 'discarded', 'published', 'superseded', 'yanked']);
const allowedPolicyDecisions = new Set(['version-override', 'initial-release', 'candidate-discard', 'release-yank']);
const allowedFailureCodes = new Set([
  'invalid-input',
  'auth-failed',
  'dirty-worktree',
  'stale-source',
  'validation-failed',
  'artifact-build-failed',
  'integrity-failed',
  'remote-conflict',
  'infra-failed',
  'operator-required',
  'contract-drift',
]);

function requireField(name, predicate = (value) => value !== undefined && value !== null && value !== '') {
  if (!predicate(doc[name])) failures.push(`missing or invalid ${name}`);
}

requireField('schemaVersion');
requireField('command');
requireField('phase');
requireField('mode');
requireField('status');
requireField('startedAt');
requireField('endedAt');
requireField('repository');
requireField('inputs', (value) => value && typeof value === 'object' && !Array.isArray(value));
requireField('data', (value) => value && typeof value === 'object' && !Array.isArray(value));
requireField('trackingIssue', (value) => value && typeof value === 'object' && !Array.isArray(value));
requireField('releaseRecord', (value) => value && typeof value === 'object' && !Array.isArray(value));
requireField('next', (value) => value && typeof value === 'object' && !Array.isArray(value));

if (doc.trackingIssue && typeof doc.trackingIssue === 'object') {
  for (const field of ['repository', 'number', 'url', 'metadataCommentUrl']) {
    if (!(field in doc.trackingIssue)) failures.push(`trackingIssue missing ${field}`);
  }
  if (
    'metadataCommentUrl' in doc.trackingIssue &&
    doc.trackingIssue.metadataCommentUrl !== null &&
    typeof doc.trackingIssue.metadataCommentUrl !== 'string'
  ) {
    failures.push('trackingIssue metadataCommentUrl must be a string or null');
  }
}

if (doc.releaseRecord && typeof doc.releaseRecord === 'object') {
  for (const field of ['lifecycleState', 'recordUrl', 'sha256']) {
    if (!(field in doc.releaseRecord)) failures.push(`releaseRecord missing ${field}`);
  }
  if (
    'lifecycleState' in doc.releaseRecord &&
    doc.releaseRecord.lifecycleState !== null &&
    !allowedLifecycleStates.has(doc.releaseRecord.lifecycleState)
  ) {
    failures.push(`releaseRecord invalid lifecycleState ${doc.releaseRecord.lifecycleState}`);
  }
  if (Array.isArray(doc.releaseRecord.policyDecisions)) {
    for (const [index, decision] of doc.releaseRecord.policyDecisions.entries()) {
      if (!decision || typeof decision !== 'object') {
        failures.push(`releaseRecord.policyDecisions[${index}] must be an object`);
        continue;
      }
      if (!allowedPolicyDecisions.has(decision.decision)) {
        failures.push(`releaseRecord.policyDecisions[${index}] invalid decision ${decision.decision}`);
      }
    }
  } else if ('policyDecisions' in doc.releaseRecord) {
    failures.push('releaseRecord policyDecisions must be an array when present');
  }
}

if (doc.next && typeof doc.next === 'object') {
  for (const field of ['command', 'reason']) {
    if (!(field in doc.next)) failures.push(`next missing ${field}`);
  }
}

if (!allowedModes.has(doc.mode)) failures.push(`invalid mode: ${doc.mode}`);
if (!allowedStatuses.has(doc.status)) failures.push(`invalid status: ${doc.status}`);
if (!allowedPhases.has(doc.phase)) failures.push(`invalid phase: ${doc.phase}`);

if (expectedCommand && doc.command !== expectedCommand) {
  failures.push(`expected command ${expectedCommand}, got ${doc.command}`);
}

if (doc.schemaVersion !== '1.0.0') {
  failures.push(`expected schemaVersion 1.0.0, got ${doc.schemaVersion}`);
}

if (actualExit !== expectedExit) {
  failures.push(`expected exit ${expectedExit}, got ${actualExit}`);
}

if (doc.command === 'preflight' && actualExit >= 1 && actualExit <= 125) {
  if (!doc.data || typeof doc.data.failedGateCount !== 'number') {
    failures.push('preflight output must include data.failedGateCount');
  } else if (actualExit !== Math.min(doc.data.failedGateCount, 125)) {
    failures.push(`preflight exit ${actualExit} does not match failedGateCount ${doc.data.failedGateCount}`);
  }
}

if (!Array.isArray(doc.warnings)) failures.push('warnings must be an array');
if (!Array.isArray(doc.failures)) failures.push('failures must be an array');

if (actualExit !== 0 && (!Array.isArray(doc.failures) || doc.failures.length === 0)) {
  failures.push('non-zero exits must include failures[]');
}

if (Array.isArray(doc.failures)) {
  for (const [index, failure] of doc.failures.entries()) {
    if (!failure || typeof failure !== 'object') {
      failures.push(`failures[${index}] must be an object`);
      continue;
    }
    for (const field of ['code', 'message', 'retryable', 'recovery', 'evidence']) {
      if (!(field in failure)) failures.push(`failures[${index}] missing ${field}`);
    }
    if ('code' in failure && !allowedFailureCodes.has(failure.code)) {
      failures.push(`failures[${index}] invalid code ${failure.code}`);
    }
  }
}

if (failures.length > 0) {
  for (const failure of failures) console.error(failure);
  process.exit(1);
}
NODE
}

run_contract() {
  local -a command_args=()
  parse_common_args "$@"

  [[ -n "$name" ]] || fail "run-contract requires --name"
  [[ -n "$expected_exit" ]] || fail "run-contract requires --expected-exit"
  [[ -n "$expected_command" ]] || fail "run-contract requires --expected-command"
  ((${#command_args[@]} > 0)) || fail "run-contract requires a command"

  require_node

  local tmp
  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"; trap - RETURN' RETURN

  local rc=0
  "${command_args[@]}" >"$tmp/stdout.json" 2>"$tmp/stderr.log" || rc=$?
  validate_json_contract "$tmp/stdout.json" "$rc" "$expected_exit" "$expected_command"
}

run_kill9_rerun() {
  local -a command_args=()
  parse_common_args "$@"

  [[ -n "$name" ]] || fail "run-kill9-rerun requires --name"
  [[ -n "$state_file" ]] || fail "run-kill9-rerun requires --state-file"
  ((${#command_args[@]} > 0)) || fail "run-kill9-rerun requires a command"

  require_node

  rm -f "$state_file"
  perl -e 'setpgrp(0, 0) or die "setpgrp: $!"; exec @ARGV or die "exec: $!"' -- "${command_args[@]}" >/dev/null 2>&1 &
  local pid=$!

  local waited=0
  while [[ ! -f "$state_file" ]]; do
    if (( waited >= 50 )); then
      kill -9 "-$pid" >/dev/null 2>&1 || kill -9 "$pid" >/dev/null 2>&1 || true
      wait "$pid" 2>/dev/null || true
      fail "kill test command did not report started state"
    fi
    sleep 0.1
    waited=$((waited + 1))
  done

  kill -9 "-$pid" >/dev/null 2>&1 || kill -9 "$pid" >/dev/null 2>&1 || true
  wait "$pid" 2>/dev/null || true

  local tmp
  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"; trap - RETURN' RETURN

  local rc=0
  "${command_args[@]}" >"$tmp/stdout.json" 2>"$tmp/stderr.log" || rc=$?
  validate_json_contract "$tmp/stdout.json" "$rc" "$rc" ""

  node - "$tmp/stdout.json" <<'NODE'
const fs = require('node:fs');
const doc = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
const resumed = doc.status === 'success' && doc.data && doc.data.rerunAfterKill === true;
const recoverable = doc.status === 'recoverable' && Array.isArray(doc.failures) && doc.failures.length > 0;
if (!resumed && !recoverable) {
  console.error('kill-9 rerun must resume safely or report a deterministic recoverable state');
  process.exit(1);
}
NODE
}

main() {
  local subcommand="${1:-}"
  shift || true

  case "$subcommand" in
    run-contract) run_contract "$@" ;;
    run-kill9-rerun) run_kill9_rerun "$@" ;;
    -h|--help|help) usage ;;
    *)
      usage >&2
      exit 129
      ;;
  esac
}

main "$@"
