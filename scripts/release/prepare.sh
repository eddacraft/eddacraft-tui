#!/usr/bin/env bash
set -euo pipefail

COMMAND="prepare"
PHASE="prepare"
SCHEMA_VERSION="1.0.0"
DEFAULT_REPO="eddacraft/anvil-001"

json=false
dry_run=false
repo="$DEFAULT_REPO"
version=""
release_type=""
strategy="direct"
source_sha=""
tracking_issue=""
request_readiness=false
request_candidate_artifacts=false
mode="compatibility"
started_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

usage() {
  cat <<'USAGE'
Usage: prepare.sh --version <vX.Y.Z[-suffix]> --release-type <beta|production> --strategy <direct|stabilisation> [--json] [--dry-run]

Prepare release metadata from live git state. This initial implementation is
local and dry-run safe; it does not call GitHub or mutate release files.

Options:
  --json                         Accepted for command-surface consistency; output is always one JSON object
  --dry-run                      Report planned preparation without mutation
  --version <version>            Release version, e.g. v0.7.0-beta
  --release-type <type>          beta or production
  --strategy <strategy>          direct or stabilisation
  --source-sha <sha>             Exact source SHA; defaults to HEAD
  --tracking-issue <number|url>  Existing release tracking issue
  --request-readiness            Record intent to request readiness
  --request-candidate-artifacts  Record intent to request candidate artifacts
  --repo <owner/name>            Source repository; defaults to eddacraft/anvil-001
  -h, --help                     Show this help
USAGE
}

now() {
  date -u +%Y-%m-%dT%H:%M:%SZ
}

json_array_from_file() {
  node - "$1" <<'NODE'
const fs = require('node:fs');
const path = process.argv[2];
const lines = fs.existsSync(path) ? fs.readFileSync(path, 'utf8').split(/\r?\n/).filter(Boolean) : [];
process.stdout.write(JSON.stringify(lines));
NODE
}

failure_json() {
  local code="$1"
  local message="$2"
  local retryable="$3"
  local recovery="$4"
  node - "$code" "$message" "$retryable" "$recovery" <<'NODE'
const [code, message, retryableRaw, recovery] = process.argv.slice(2);
process.stdout.write(JSON.stringify([{
  code,
  message,
  retryable: retryableRaw === 'true',
  recovery,
  evidence: { command: 'scripts/release/prepare.sh', url: null, path: null },
}]));
NODE
}

empty_data_json() {
  printf '%s' '{"prepCommitSha":null,"changedFiles":[],"trackingIssueUrl":null,"candidateMetadata":{},"idempotencyKey":null}'
}

emit_envelope() {
  local status="$1"
  local data_json="$2"
  local failures_json="$3"
  local next_command="$4"
  local next_reason="$5"
  local ended_at
  ended_at="$(now)"

  node - \
    "$SCHEMA_VERSION" "$COMMAND" "$PHASE" "$status" "$started_at" "$ended_at" \
    "$repo" "$mode" "$version" "$source_sha" "$tracking_issue" "$data_json" "$failures_json" \
    "$next_command" "$next_reason" <<'NODE'
const [schemaVersion, command, phase, status, startedAt, endedAt, repository, mode, version, sourceSha, trackingIssue, dataRaw, failuresRaw, nextCommand, nextReason] = process.argv.slice(2);
process.stdout.write(JSON.stringify({
  schemaVersion,
  command,
  phase,
  mode,
  status,
  startedAt,
  endedAt,
  repository,
  inputs: {
    base: null,
    head: null,
    version: version || null,
    sourceSha: sourceSha || null,
    trackingIssue: trackingIssue || null,
  },
  trackingIssue: {
    repository,
    number: /^\d+$/.test(trackingIssue) ? Number(trackingIssue) : null,
    url: /^https?:/.test(trackingIssue) ? trackingIssue : null,
    metadataCommentUrl: null,
  },
  releaseRecord: {
    lifecycleState: status === 'success' ? 'candidate' : null,
    recordUrl: null,
    sha256: null,
  },
  data: JSON.parse(dataRaw),
  warnings: [],
  failures: JSON.parse(failuresRaw),
  next: { command: nextCommand, reason: nextReason },
}) + '\n');
NODE
}

fail_usage() {
  local message="$1"
  if [[ "$json" == "true" ]]; then
    emit_envelope "failed" "$(empty_data_json)" "$(failure_json invalid-input "$message" false correct-usage)" "prepare" "Fix command arguments and rerun prepare."
  else
    printf 'prepare: %s\n' "$message" >&2
    usage >&2
  fi
  exit 129
}

require_value() {
  local name="$1"
  local value="${2:-}"
  [[ -n "$value" ]] || fail_usage "$name requires a value"
}

while (($# > 0)); do
  case "$1" in
    --json) json=true; shift ;;
    --dry-run) dry_run=true; shift ;;
    --version) require_value "$1" "${2:-}"; version="$2"; shift 2 ;;
    --release-type) require_value "$1" "${2:-}"; release_type="$2"; shift 2 ;;
    --strategy) require_value "$1" "${2:-}"; strategy="$2"; shift 2 ;;
    --source-sha) require_value "$1" "${2:-}"; source_sha="$2"; mode="target"; shift 2 ;;
    --tracking-issue) require_value "$1" "${2:-}"; tracking_issue="$2"; shift 2 ;;
    --request-readiness) request_readiness=true; shift ;;
    --request-candidate-artifacts) request_candidate_artifacts=true; shift ;;
    --repo) require_value "$1" "${2:-}"; repo="$2"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) fail_usage "unknown argument: $1" ;;
  esac
done

if ! git rev-parse --show-toplevel >/dev/null 2>&1; then
  emit_envelope "failed" "$(empty_data_json)" "$(failure_json invalid-input "not inside a git repository" false run-from-repository)" "prepare" "Run prepare from a git repository."
  exit 129
fi

[[ -n "$version" ]] || fail_usage "--version is required"
[[ "$version" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[A-Za-z0-9._-]+)?$ ]] || fail_usage "--version must look like vX.Y.Z[-suffix]"
[[ -n "$release_type" ]] || fail_usage "--release-type is required"
[[ "$release_type" == "beta" || "$release_type" == "production" ]] || fail_usage "--release-type must be beta or production"
[[ "$strategy" == "direct" || "$strategy" == "stabilisation" ]] || fail_usage "--strategy must be direct or stabilisation"

if [[ -n "$source_sha" ]]; then
  [[ "$source_sha" =~ ^[0-9a-fA-F]{40}$ ]] || fail_usage "--source-sha requires a full 40-character commit SHA"
  resolved_sha="$(git rev-parse --verify "${source_sha}^{commit}" 2>/dev/null || true)"
  [[ -n "$resolved_sha" ]] || fail_usage "source SHA is not a commit: $source_sha"
  source_sha="$resolved_sha"
else
  source_sha="$(git rev-parse HEAD)"
fi

if [[ -n "${ANVIL_RELEASE_PREPARE_KILL_STATE:-}" ]]; then
  if [[ "${ANVIL_RELEASE_TEST_MODE:-}" != "kill-rerun" ]]; then
    emit_envelope "failed" "$(empty_data_json)" "$(failure_json invalid-input "ANVIL_RELEASE_PREPARE_KILL_STATE requires ANVIL_RELEASE_TEST_MODE=kill-rerun" false correct-test-usage)" "prepare" "Unset the test hook or enable explicit kill-rerun test mode."
    exit 129
  fi
  if [[ -f "$ANVIL_RELEASE_PREPARE_KILL_STATE" ]]; then
    data='{"prepCommitSha":null,"changedFiles":[],"trackingIssueUrl":null,"candidateMetadata":{},"idempotencyKey":"prepare-kill-rerun","rerunAfterKill":true}'
    emit_envelope "success" "$data" "[]" "promote" "Prepare resumed after interruption; continue to promotion."
    exit 0
  fi
  printf '%s\n' 'prepare: entering explicit kill-rerun test hook' >&2
  printf '%s\n' started >"$ANVIL_RELEASE_PREPARE_KILL_STATE"
  timeout_seconds="${ANVIL_RELEASE_TEST_TIMEOUT_SECONDS:-30}"
  sleep "$timeout_seconds"
  emit_envelope "recoverable" "$(empty_data_json)" "$(failure_json operator-required "kill-rerun test hook timed out without signal" true rerun-prepare)" "prepare" "Rerun prepare to verify resumability."
  exit 1
fi

if [[ "$dry_run" != "true" && -n "$(git status --porcelain)" ]]; then
  emit_envelope "failed" "$(empty_data_json)" "$(failure_json dirty-worktree "prepare requires a clean worktree unless --dry-run is used" true clean-or-dry-run)" "prepare" "Clean the worktree or rerun with --dry-run."
  exit 1
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
: >"${tmp}/changed-files"
for path in package.json CHANGELOG.md docs/public/anvil/releases/changelog.md; do
  [[ -e "$path" ]] && printf '%s\n' "$path" >>"$tmp/changed-files"
done

changed_files_json="$(json_array_from_file "$tmp/changed-files")"
data_json="$(node - \
  "$version" "$release_type" "$strategy" "$source_sha" "$tracking_issue" "$request_readiness" "$request_candidate_artifacts" "$changed_files_json" <<'NODE'
const [version, releaseType, strategy, sourceSha, trackingIssue, readinessRaw, artifactsRaw, changedFilesRaw] = process.argv.slice(2);
const trackingIssueUrl = /^https?:/.test(trackingIssue) ? trackingIssue : null;
process.stdout.write(JSON.stringify({
  prepCommitSha: null,
  changedFiles: JSON.parse(changedFilesRaw),
  trackingIssueUrl,
  candidateMetadata: {
    version,
    releaseType,
    strategy,
    sourceSha,
    readinessRequests: {
      readiness: readinessRaw === 'true',
      candidateArtifacts: artifactsRaw === 'true',
    },
  },
  idempotencyKey: `prepare:${version}:${strategy}:${sourceSha}`,
}));
NODE
)"

if [[ "$dry_run" == "true" ]]; then
  emit_envelope "success" "$data_json" "[]" "promote" "Dry-run preparation is valid; run promote after operator approval."
else
  emit_envelope "needs-operator" "$data_json" "$(failure_json operator-required "non-dry-run prepare is not enabled in the initial local implementation" false use-dry-run-or-implement-gh)" "prepare" "Use --dry-run or complete GitHub-backed prepare implementation."
  exit 1
fi
