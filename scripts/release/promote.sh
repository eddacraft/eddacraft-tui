#!/usr/bin/env bash
set -euo pipefail

COMMAND="promote"
PHASE="promote"
SCHEMA_VERSION="1.0.0"
DEFAULT_REPO="eddacraft/anvil-001"

json=false
dry_run=false
repo="$DEFAULT_REPO"
version=""
strategy="direct"
base_ref=""
head_ref=""
source_sha=""
tracking_issue=""
mode="compatibility"
started_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

usage() {
  cat <<'USAGE'
Usage: promote.sh --version <vX.Y.Z[-suffix]> [--strategy <direct|stabilisation>] [--base <ref> --head <ref> | --source-sha <sha>] [--json] [--dry-run]

Report or plan release promotion. This initial implementation is local and
dry-run safe; it does not create or merge GitHub pull requests.

Options:
  --json                         Emit one JSON object only
  --dry-run                      Report planned promotion without mutation
  --version <version>            Release version, e.g. v0.7.0-beta
  --strategy <strategy>          direct or stabilisation
  --base <ref>                   Compatibility-mode base ref
  --head <ref>                   Compatibility-mode head ref
  --source-sha <sha>             Target-mode exact source SHA; no promotion needed
  --tracking-issue <number|url>  Existing release tracking issue
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
  evidence: { command: 'scripts/release/promote.sh', url: null, path: null },
}]));
NODE
}

empty_data_json() {
  printf '%s' '{"pullRequest":null,"mergeState":null,"mergedSha":null,"operatorActionRequired":true}'
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
    "$repo" "$mode" "$base_ref" "$head_ref" "$version" "$source_sha" "$tracking_issue" \
    "$data_json" "$failures_json" "$next_command" "$next_reason" <<'NODE'
const [schemaVersion, command, phase, status, startedAt, endedAt, repository, mode, base, head, version, sourceSha, trackingIssue, dataRaw, failuresRaw, nextCommand, nextReason] = process.argv.slice(2);
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
    base: base || null,
    head: head || null,
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
    lifecycleState: status === 'success' || status === 'noop' ? 'candidate' : null,
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
    emit_envelope "failed" "$(empty_data_json)" "$(failure_json invalid-input "$message" false correct-usage)" "promote" "Fix command arguments and rerun promote."
  else
    printf 'promote: %s\n' "$message" >&2
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
    --strategy) require_value "$1" "${2:-}"; strategy="$2"; shift 2 ;;
    --base) require_value "$1" "${2:-}"; base_ref="$2"; shift 2 ;;
    --head) require_value "$1" "${2:-}"; head_ref="$2"; shift 2 ;;
    --source-sha) require_value "$1" "${2:-}"; source_sha="$2"; mode="target"; shift 2 ;;
    --tracking-issue) require_value "$1" "${2:-}"; tracking_issue="$2"; shift 2 ;;
    --repo) require_value "$1" "${2:-}"; repo="$2"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) fail_usage "unknown argument: $1" ;;
  esac
done

if ! git rev-parse --show-toplevel >/dev/null 2>&1; then
  emit_envelope "failed" "$(empty_data_json)" "$(failure_json invalid-input "not inside a git repository" false run-from-repository)" "promote" "Run promote from a git repository."
  exit 129
fi

[[ -n "$version" ]] || fail_usage "--version is required"
[[ "$version" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[A-Za-z0-9._-]+)?$ ]] || fail_usage "--version must look like vX.Y.Z[-suffix]"
[[ "$strategy" == "direct" || "$strategy" == "stabilisation" ]] || fail_usage "--strategy must be direct or stabilisation"

if [[ -n "$source_sha" ]]; then
  [[ -z "$base_ref" && -z "$head_ref" ]] || fail_usage "use --source-sha without --base/--head in target mode"
  [[ "$source_sha" =~ ^[0-9a-fA-F]{40}$ ]] || fail_usage "--source-sha requires a full 40-character commit SHA"
  resolved_sha="$(git rev-parse --verify "${source_sha}^{commit}" 2>/dev/null || true)"
  [[ -n "$resolved_sha" ]] || fail_usage "source SHA is not a commit: $source_sha"
  source_sha="$resolved_sha"
  data_json="$(node - "$source_sha" <<'NODE'
const sourceSha = process.argv[2];
process.stdout.write(JSON.stringify({
  pullRequest: null,
  mergeState: 'not-required',
  mergedSha: sourceSha,
  operatorActionRequired: false,
}));
NODE
)"
  emit_envelope "noop" "$data_json" "[]" "tag" "Target mode does not require promotion; continue to tag after readiness evidence is valid."
  exit 0
fi

[[ -n "$base_ref" ]] || fail_usage "--base is required unless --source-sha is used"
[[ -n "$head_ref" ]] || fail_usage "--head is required unless --source-sha is used"
base_sha="$(git rev-parse --verify "${base_ref}^{commit}" 2>/dev/null || true)"
head_sha="$(git rev-parse --verify "${head_ref}^{commit}" 2>/dev/null || true)"
[[ -n "$base_sha" ]] || fail_usage "base ref is not a commit: $base_ref"
[[ -n "$head_sha" ]] || fail_usage "head ref is not a commit: $head_ref"

data_json="$(node - "$version" "$strategy" "$base_ref" "$head_ref" "$base_sha" "$head_sha" <<'NODE'
const [version, strategy, baseRef, headRef, baseSha, headSha] = process.argv.slice(2);
process.stdout.write(JSON.stringify({
  pullRequest: {
    number: null,
    url: null,
    title: `Release ${version}`,
    base: baseRef,
    head: headRef,
    strategy,
  },
  mergeState: 'not-created',
  mergedSha: null,
  operatorActionRequired: true,
  comparison: { baseSha, headSha },
}));
NODE
)"

if [[ "$dry_run" == "true" ]]; then
  emit_envelope "success" "$data_json" "[]" "promote" "Dry-run promotion plan is valid; create or resume the promotion PR after operator approval."
else
  emit_envelope "needs-operator" "$data_json" "$(failure_json operator-required "non-dry-run promote is not enabled in the initial local implementation" false use-dry-run-or-implement-gh)" "promote" "Use --dry-run or complete GitHub-backed promotion implementation."
  exit 1
fi
