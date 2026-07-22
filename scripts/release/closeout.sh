#!/usr/bin/env bash
set -euo pipefail

COMMAND="closeout"
PHASE="closeout"
SCHEMA_VERSION="1.0.0"
DEFAULT_REPO="eddacraft/anvil-001"

json=false
dry_run=false
apply=false
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
  --apply                        Execute the mutating cleanup actions (gh release edit --latest,
                                 tracking issue comment/close, branch deletion). Required for
                                 non-dry-run execution; without it, closeout reports the plan
                                 and returns needs-operator so accidental invocations on a
                                 dev machine cannot mutate the public release.
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
    --apply) apply=true; shift ;;
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
[[ "$version" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[A-Za-z0-9-]+(\.[A-Za-z0-9-]+)*)?$ ]] || fail_usage "--version must look like vX.Y.Z[-suffix]"
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
  exit 0
fi

if [[ "$apply" != "true" ]]; then
  emit_envelope "needs-operator" "$data_json" "$(failure_json operator-required "non-dry-run closeout requires --apply to execute mutating cleanup" false rerun-with-apply)" "closeout" "Rerun with --apply to execute, or use --dry-run to preview the plan."
  exit 1
fi

command -v gh >/dev/null 2>&1 || {
  emit_envelope "failed" "$data_json" "$(failure_json infra-failed "gh is required for non-dry-run closeout" true install-gh)" "closeout" "Install/authenticate gh or rerun with --dry-run."
  exit 127
}

public_repo="${ANVIL_RELEASE_CLOSEOUT_PUBLIC_REPO:-eddacraft/anvil}"

classify_gh_failure() {
  local stderr_text="$1"
  local default_code="$2"
  local default_recovery="$3"
  local lc
  lc="$(printf '%s' "$stderr_text" | tr '[:upper:]' '[:lower:]')"
  if [[ "$lc" == *"http 401"* || "$lc" == *"http 403"* || "$lc" == *"authentication"* || "$lc" == *"unauthorized"* || "$lc" == *"forbidden"* || "$lc" == *"permission"* ]]; then
    printf '%s\n%s\n' auth-failed gh-auth
  elif [[ "$lc" == *"http 404"* || "$lc" == *"not found"* || "$lc" == *"could not find"* ]]; then
    printf '%s\n%s\n' infra-failed verify-target-exists
  else
    printf '%s\n%s\n' "$default_code" "$default_recovery"
  fi
}

if [[ -n "$tracking_issue" ]]; then
  summary_body="$(node - "$version" "$tag" "$source_sha" "$verification_record" <<'NODE'
const [version, tag, sourceSha, verificationRecord] = process.argv.slice(2);
const lines = [
  '<!-- anvil-release-closeout -->',
  `## Release closeout — ${version}`,
  '',
  `- Tag: \`${tag}\``,
  `- Source SHA: \`${sourceSha}\``,
  `- Verification record: ${verificationRecord}`,
  `- Closed at: ${new Date().toISOString()}`,
];
process.stdout.write(lines.join('\n') + '\n');
NODE
)"
  gh issue comment "$tracking_issue" --repo "$repo" --body "$summary_body" >/dev/null 2>&1 || {
    emit_envelope "failed" "$data_json" "$(failure_json auth-failed "failed to post final summary on tracking issue" true gh-auth-or-issue)" "closeout" "Authenticate gh or verify the tracking issue, then rerun closeout."
    exit 1
  }
fi

release_edit_err="$(gh release edit "$tag" --repo "$public_repo" --latest 2>&1 >/dev/null)" || {
  mapfile -t classification < <(classify_gh_failure "$release_edit_err" infra-failed verify-public-release)
  emit_envelope "failed" "$data_json" "$(failure_json "${classification[0]}" "failed to mark public release latest on ${public_repo}: ${release_edit_err}" true "${classification[1]}")" "closeout" "Resolve the gh error above and rerun closeout. Override the public repo with ANVIL_RELEASE_CLOSEOUT_PUBLIC_REPO if needed."
  exit 1
}

closed_issue=false
if [[ "$close_issue" == "true" && -n "$tracking_issue" ]]; then
  gh issue close "$tracking_issue" --repo "$repo" --reason completed >/dev/null 2>&1 || {
    emit_envelope "failed" "$data_json" "$(failure_json auth-failed "failed to close tracking issue" true gh-auth-or-issue)" "closeout" "Authenticate gh or verify the tracking issue, then rerun closeout."
    exit 1
  }
  closed_issue=true
fi

if [[ -n "$cleanup_branch" ]]; then
  encoded_branch="$(node -e "process.stdout.write(encodeURIComponent(process.argv[1]))" "$cleanup_branch")"
  gh api -X DELETE "repos/$repo/git/refs/heads/$encoded_branch" >/dev/null 2>&1 || {
    emit_envelope "failed" "$data_json" "$(failure_json infra-failed "failed to delete release branch $cleanup_branch" true verify-branch-or-permissions)" "closeout" "Verify the branch exists and the token has delete permission, then rerun closeout."
    exit 1
  }
fi

data_json="$(node - "$data_json" "$closed_issue" <<'NODE'
const [raw, closedRaw] = process.argv.slice(2);
const data = JSON.parse(raw);
data.closedIssue = closedRaw === 'true';
data.operatorActionRequired = false;
process.stdout.write(JSON.stringify(data));
NODE
)"

emit_envelope "success" "$data_json" "[]" "done" "Release closeout completed."
exit 0
