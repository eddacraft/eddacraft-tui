#!/usr/bin/env bash
set -euo pipefail

COMMAND="closeout"
PHASE="closeout"
SCHEMA_VERSION="1.0.0"
DEFAULT_REPO="eddacraft/anvil-001"

json=false
dry_run=false
repo="$DEFAULT_REPO"
version=""
tag=""
source_sha=""
tracking_issue=""
verification_record=""
verification_passed=false
close_issue=false
cleanup_branch=""
mode="target"
started_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

usage() {
  cat <<'USAGE'
Usage: closeout.sh --version <vX.Y.Z[-suffix]> --tag <tag> --source-sha <sha> --verification-record <url> [--json] [--dry-run]

Plan release closeout after verification has passed. This initial implementation
is local and dry-run safe; it does not mutate branches, releases, or issues.

Options:
  --json                         Accepted for command-surface consistency; output is always one JSON object
  --dry-run                      Report planned closeout without mutation
  --version <version>            Release version, e.g. v0.7.0-beta
  --tag <tag>                    Verified release tag
  --source-sha <sha>             Exact released source SHA
  --verification-record <url>    URL to passed verification evidence
  --verification-passed          Assert verification has passed
  --tracking-issue <number|url>  Existing release tracking issue
  --close-issue                  Include tracking issue closure in the plan
  --cleanup-branch <branch>      Include release branch cleanup in the plan
  --repo <owner/name>            Source repository; defaults to eddacraft/anvil-001
  -h, --help                     Show this help
USAGE
}

now() { date -u +%Y-%m-%dT%H:%M:%SZ; }

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
  evidence: { command: 'scripts/release/closeout.sh', url: null, path: null },
}]));
NODE
}

empty_data_json() {
  printf '%s' '{"closedIssue":false,"cleanupActions":[],"releaseRecordUrl":null,"operatorActionRequired":true}'
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
    "$repo" "$mode" "$version" "$tag" "$source_sha" "$tracking_issue" "$verification_record" \
    "$data_json" "$failures_json" "$next_command" "$next_reason" <<'NODE'
const [schemaVersion, command, phase, status, startedAt, endedAt, repository, mode, version, tag, sourceSha, trackingIssue, verificationRecord, dataRaw, failuresRaw, nextCommand, nextReason] = process.argv.slice(2);
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
    tag: tag || null,
    sourceSha: sourceSha || null,
    verificationRecord: verificationRecord || null,
    trackingIssue: trackingIssue || null,
  },
  trackingIssue: {
    repository,
    number: /^\d+$/.test(trackingIssue) ? Number(trackingIssue) : null,
    url: /^https?:/.test(trackingIssue) ? trackingIssue : null,
    metadataCommentUrl: null,
  },
  releaseRecord: {
    lifecycleState: status === 'success' ? 'published' : null,
    recordUrl: verificationRecord || null,
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
    emit_envelope "failed" "$(empty_data_json)" "$(failure_json invalid-input "$message" false correct-usage)" "closeout" "Fix command arguments and rerun closeout."
  else
    printf 'closeout: %s\n' "$message" >&2
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
    --tag) require_value "$1" "${2:-}"; tag="$2"; shift 2 ;;
    --source-sha) require_value "$1" "${2:-}"; source_sha="$2"; shift 2 ;;
    --verification-record) require_value "$1" "${2:-}"; verification_record="$2"; shift 2 ;;
    --verification-passed) verification_passed=true; shift ;;
    --tracking-issue) require_value "$1" "${2:-}"; tracking_issue="$2"; shift 2 ;;
    --close-issue) close_issue=true; shift ;;
    --cleanup-branch) require_value "$1" "${2:-}"; cleanup_branch="$2"; shift 2 ;;
    --repo) require_value "$1" "${2:-}"; repo="$2"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) fail_usage "unknown argument: $1" ;;
  esac
done

if ! git rev-parse --show-toplevel >/dev/null 2>&1; then
  emit_envelope "failed" "$(empty_data_json)" "$(failure_json invalid-input "not inside a git repository" false run-from-repository)" "closeout" "Run closeout from a git repository."
  exit 129
fi

[[ -n "$version" ]] || fail_usage "--version is required"
[[ "$version" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[A-Za-z0-9._-]+)?$ ]] || fail_usage "--version must look like vX.Y.Z[-suffix]"
[[ -n "$tag" ]] || fail_usage "--tag is required"
[[ -n "$source_sha" ]] || fail_usage "--source-sha is required"
[[ "$source_sha" =~ ^[0-9a-fA-F]{40}$ ]] || fail_usage "--source-sha requires a full 40-character commit SHA"
resolved_sha="$(git rev-parse --verify "${source_sha}^{commit}" 2>/dev/null || true)"
[[ -n "$resolved_sha" ]] || fail_usage "source SHA is not a commit: $source_sha"
source_sha="$resolved_sha"
[[ -n "$verification_record" ]] || fail_usage "--verification-record is required"

if [[ "$verification_passed" != "true" ]]; then
  emit_envelope "blocked" "$(empty_data_json)" "$(failure_json operator-required "closeout requires passed verification evidence" true provide-verification-evidence)" "verify" "Run verify and rerun closeout with --verification-passed."
  exit 1
fi

data_json="$(node - "$version" "$tag" "$source_sha" "$verification_record" "$tracking_issue" "$close_issue" "$cleanup_branch" <<'NODE'
const [version, tag, sourceSha, verificationRecord, trackingIssue, closeIssueRaw, cleanupBranch] = process.argv.slice(2);
const cleanupActions = [
  { action: 'record-final-summary', target: trackingIssue || null, required: Boolean(trackingIssue) },
  { action: 'mark-public-release-latest', target: tag, required: true },
];
if (closeIssueRaw === 'true') cleanupActions.push({ action: 'close-tracking-issue', target: trackingIssue || null, required: true });
if (cleanupBranch) cleanupActions.push({ action: 'delete-release-branch', target: cleanupBranch, required: true });
process.stdout.write(JSON.stringify({
  closedIssue: false,
  cleanupActions,
  releaseRecordUrl: verificationRecord,
  operatorActionRequired: true,
  finalSummary: { version, tag, sourceSha, verificationRecord },
}));
NODE
)"

if [[ -n "${ANVIL_RELEASE_CLOSEOUT_FAKE_ISSUE_FILE:-}" ]]; then
  if [[ "${ANVIL_RELEASE_TEST_MODE:-}" != "closeout-fake-issue" ]]; then
    emit_envelope "failed" "$(empty_data_json)" "$(failure_json invalid-input "ANVIL_RELEASE_CLOSEOUT_FAKE_ISSUE_FILE requires ANVIL_RELEASE_TEST_MODE=closeout-fake-issue" false correct-test-usage)" "closeout" "Unset the test hook or enable explicit fake issue test mode."
    exit 129
  fi
  [[ "$close_issue" == "true" ]] || fail_usage "fake issue closeout requires --close-issue"
  node - "$ANVIL_RELEASE_CLOSEOUT_FAKE_ISSUE_FILE" "$version" "$tag" "$source_sha" "$verification_record" <<'NODE'
const fs = require('node:fs');
const [path, version, tag, sourceSha, verificationRecord] = process.argv.slice(2);
fs.writeFileSync(path, JSON.stringify({ closed: true, version, tag, sourceSha, verificationRecord }) + '\n');
NODE
  data_json="$(node - "$data_json" <<'NODE'
const data = JSON.parse(process.argv[2]);
data.closedIssue = true;
data.operatorActionRequired = false;
process.stdout.write(JSON.stringify(data));
NODE
)"
  emit_envelope "success" "$data_json" "[]" "done" "Fake issue closeout completed for harness validation."
  exit 0
fi

if [[ "$dry_run" == "true" ]]; then
  emit_envelope "success" "$data_json" "[]" "done" "Dry-run closeout plan is valid; execute mutating cleanup after operator approval."
else
  emit_envelope "needs-operator" "$data_json" "$(failure_json operator-required "non-dry-run closeout is not enabled in the initial local implementation" false use-dry-run-or-implement-gh)" "closeout" "Use --dry-run or complete GitHub-backed closeout implementation."
  exit 1
fi
