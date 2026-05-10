#!/usr/bin/env bash
set -euo pipefail

COMMAND="assess"
PHASE="assessment"
SCHEMA_VERSION="1.0.0"
DEFAULT_REPO="eddacraft/anvil-001"

json=false
repo="$DEFAULT_REPO"
base_ref=""
head_ref=""
source_sha=""
mode="compatibility"
started_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

usage() {
  cat <<'USAGE'
Usage: assess.sh --base <ref> (--head <ref> | --source-sha <sha>) [--json] [--repo <owner/name>]

Assess local git state for a release candidate. This initial implementation
does not call GitHub or the network.

Options:
  --json             Emit one JSON object only
  --base <ref>       Comparison base ref or previous release boundary
  --head <ref>       Compatibility-mode comparison head ref
  --source-sha <sha> Target-mode exact source SHA to assess
  --repo <owner/name> Source repository name; defaults to eddacraft/anvil-001
  -h, --help         Show this help
USAGE
}

now() {
  date -u +%Y-%m-%dT%H:%M:%SZ
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
    "$repo" "$mode" "${base_ref:-}" "${head_ref:-}" "${source_sha:-}" "$data_json" "$failures_json" \
    "$next_command" "$next_reason" <<'NODE'
const [
  schemaVersion,
  command,
  phase,
  status,
  startedAt,
  endedAt,
  repository,
  mode,
  base,
  head,
  sourceSha,
  dataRaw,
  failuresRaw,
  nextCommand,
  nextReason,
] = process.argv.slice(2);

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
    version: null,
    sourceSha: sourceSha || null,
    trackingIssue: null,
  },
  trackingIssue: {
    repository,
    number: null,
    url: null,
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
  next: {
    command: nextCommand,
    reason: nextReason,
  },
}) + '\n');
NODE
}

empty_data_json() {
  printf '%s' '{"candidateVersion":null,"releaseType":null,"recommendedStrategy":null,"previousTag":null,"sourceSha":null,"changedPaths":[],"apsItems":[],"riskSignals":[],"releaseWarranted":false}'
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
  evidence: { command: 'scripts/release/assess.sh', url: null, path: null },
}]));
NODE
}

fail_usage() {
  local message="$1"
  emit_envelope "failed" "$(empty_data_json)" "$(failure_json invalid-input "$message" false correct-usage)" "assess" "Fix command arguments and rerun assessment."
  exit 129
}

require_value() {
  local name="$1"
  local value="${2:-}"
  [[ -n "$value" ]] || fail_usage "$name requires a value"
}

while (($# > 0)); do
  case "$1" in
    --json)
      json=true
      shift
      ;;
    --base)
      require_value "$1" "${2:-}"
      base_ref="$2"
      shift 2
      ;;
    --head)
      require_value "$1" "${2:-}"
      head_ref="$2"
      shift 2
      ;;
    --source-sha)
      require_value "$1" "${2:-}"
      source_sha="$2"
      mode="target"
      shift 2
      ;;
    --repo)
      require_value "$1" "${2:-}"
      repo="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      fail_usage "unknown argument: $1"
      ;;
  esac
done

if ! git rev-parse --show-toplevel >/dev/null 2>&1; then
  emit_envelope "failed" "$(empty_data_json)" "$(failure_json invalid-input "not inside a git repository" false "run-from-repository")" "assess" "Run assessment from a git repository."
  exit 129
fi

[[ -n "$base_ref" ]] || fail_usage "--base is required"
if [[ -n "$source_sha" && -n "$head_ref" ]]; then
  fail_usage "use either --head or --source-sha, not both"
fi
if [[ -z "$source_sha" && -z "$head_ref" ]]; then
  fail_usage "--head or --source-sha is required"
fi

if [[ -n "$source_sha" ]]; then
  if [[ ! "$source_sha" =~ ^[0-9a-fA-F]{40}$ ]]; then
    fail_usage "--source-sha requires a full 40-character commit SHA"
  fi
  head_sha="$(git rev-parse --verify "${source_sha}^{commit}" 2>/dev/null || true)"
  [[ -n "$head_sha" ]] || fail_usage "source SHA is not a commit: $source_sha"
  source_sha="$head_sha"
else
  head_sha="$(git rev-parse --verify "${head_ref}^{commit}" 2>/dev/null || true)"
  [[ -n "$head_sha" ]] || fail_usage "head ref is not a commit: $head_ref"
fi

previous_tag="$(git describe --tags --abbrev=0 "$head_sha" 2>/dev/null || true)"
base_sha="$(git rev-parse --verify "${base_ref}^{commit}" 2>/dev/null || true)"
[[ -n "$base_sha" ]] || fail_usage "base ref is not a commit: $base_ref"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

git diff --name-only "$base_sha" "$head_sha" >"$tmp/changed-paths"

git log --format=%B "$base_sha..$head_sha" 2>/dev/null \
  | grep -Eo '[A-Z][A-Z0-9]+-[0-9]{3}' >"$tmp/aps-log" || true
if git diff "$base_sha" "$head_sha" -- plans 2>/dev/null \
  | grep -Eo '[A-Z][A-Z0-9]+-[0-9]{3}' >"$tmp/aps-plan"; then
  true
else
  : >"$tmp/aps-plan"
fi
cat "$tmp/aps-log" "$tmp/aps-plan" | sort -u >"$tmp/aps-items"

: >"$tmp/risk-signals"
while IFS= read -r path; do
  case "$path" in
    .github/workflows/*) printf '%s\n' "ci-workflow-changed" >>"$tmp/risk-signals" ;;
    scripts/release/*) printf '%s\n' "release-command-changed" >>"$tmp/risk-signals" ;;
    Cargo.lock|pnpm-lock.yaml|package.json|pnpm-workspace.yaml) printf '%s\n' "dependency-surface-changed" >>"$tmp/risk-signals" ;;
    plans/decisions/*) printf '%s\n' "architecture-decision-changed" >>"$tmp/risk-signals" ;;
  esac
done <"$tmp/changed-paths"
sort -u "$tmp/risk-signals" -o "$tmp/risk-signals"

candidate_version="v0.1.0-beta"
release_type="beta"
if [[ "$previous_tag" =~ ^v([0-9]+)\.([0-9]+)\.([0-9]+)(-.+)?$ ]]; then
  major="${BASH_REMATCH[1]}"
  minor="${BASH_REMATCH[2]}"
  suffix="${BASH_REMATCH[4]:-}"
  candidate_version="v${major}.$((minor + 1)).0${suffix}"
  if [[ "$suffix" == "-beta" ]]; then
    release_type="beta"
  elif [[ -n "$suffix" ]]; then
    release_type="prerelease"
  else
    release_type="stable"
  fi
fi

changed_count="$(wc -l <"$tmp/changed-paths" | tr -d ' ')"
status="success"
release_warranted="true"
next_command="preflight"
next_reason="Assessment found local changes; run readiness gates next."
if [[ "$changed_count" == "0" ]]; then
  status="noop"
  release_warranted="false"
  candidate_version="null"
  release_type="null"
  next_command="assess"
  next_reason="No changed paths between base and head; no release action is warranted."
fi

data_json="$(node - \
  "$candidate_version" "$release_type" "$previous_tag" "$head_sha" "$release_warranted" \
  "$tmp/changed-paths" "$tmp/aps-items" "$tmp/risk-signals" <<'NODE'
const fs = require('node:fs');
const [candidateVersionRaw, releaseTypeRaw, previousTag, sourceSha, releaseWarrantedRaw, changedPathFile, apsFile, riskFile] = process.argv.slice(2);
function lines(path) {
  return fs.readFileSync(path, 'utf8').split(/\r?\n/).filter(Boolean);
}
const candidateVersion = candidateVersionRaw === 'null' ? null : candidateVersionRaw;
const releaseType = releaseTypeRaw === 'null' ? null : releaseTypeRaw;
process.stdout.write(JSON.stringify({
  candidateVersion,
  releaseType,
  recommendedStrategy: 'direct',
  previousTag: previousTag || null,
  sourceSha,
  changedPaths: lines(changedPathFile),
  apsItems: lines(apsFile),
  riskSignals: lines(riskFile),
  releaseWarranted: releaseWarrantedRaw === 'true',
}));
NODE
)"

if [[ "$json" == "true" ]]; then
  emit_envelope "$status" "$data_json" "[]" "$next_command" "$next_reason"
else
  if [[ "$status" == "noop" ]]; then
    printf 'Assessment: no release warranted (%s..%s has no changed paths)\n' "$base_ref" "$head_ref"
  else
    printf 'Assessment: release warranted for %s (%s paths, source %s)\n' "$candidate_version" "$changed_count" "$head_sha"
  fi
fi
