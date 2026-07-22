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
request_readiness=false
channel=""
base_boundary=""
started_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

usage() {
  cat <<'USAGE'
Usage: promote.sh --version <vX.Y.Z[-suffix]> [--strategy <direct|stabilisation>] [--base <ref> --head <ref> | --source-sha <sha>] [--json] [--dry-run]

Report or plan release promotion. This initial implementation is local and
dry-run safe; it does not create or merge GitHub pull requests.

Options:
  --json                         Accepted for command-surface consistency; output is always one JSON object
  --dry-run                      Report planned promotion without mutation
  --version <version>            Release version, e.g. v0.7.0-beta
  --strategy <strategy>          direct or stabilisation
  --base <ref>                   Compatibility-mode base ref
  --head <ref>                   Compatibility-mode head ref
  --source-sha <sha>             Target-mode exact source SHA; no promotion needed
  --tracking-issue <number|url>  Existing release tracking issue
  --request-readiness            Request or resume release-readiness after merge
  --channel <beta|stable>        Readiness channel when --request-readiness is set
  --base-boundary <tag|ref>      Previous release boundary for readiness
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
  printf '%s' '{"pullRequest":null,"mergeState":null,"mergedSha":null,"operatorActionRequired":true,"readiness":null,"resumed":false}'
}

check_promote_fake_guard() {
  if [[ -n "${ANVIL_RELEASE_PROMOTE_FAKE_GH_FILE:-}" && "${ANVIL_RELEASE_TEST_MODE:-}" != "promote-fake-gh" ]]; then
    emit_envelope "failed" "$(empty_data_json)" "$(failure_json invalid-input "ANVIL_RELEASE_PROMOTE_FAKE_GH_FILE requires ANVIL_RELEASE_TEST_MODE=promote-fake-gh" false correct-test-usage)" "promote" "Unset the test hook or enable explicit promote fake GitHub test mode."
    exit 129
  fi
}

# Reads the tracking issue for a previously persisted readiness-run marker
# matching the given source SHA. Echoes one JSON line {runId, runUrl,
# sourceSha} on stdout, or nothing if no marker is found / no tracking
# issue / no gh. Picks the latest matching marker if several exist.
find_persisted_readiness_run() {
  local issue="$1"
  local sha="$2"
  [[ -n "$issue" ]] || return 0
  command -v gh >/dev/null 2>&1 || return 0
  local comments_json
  comments_json="$(gh issue view "$issue" --repo "$repo" --json comments --jq '[.comments[].body]' 2>/dev/null || true)"
  [[ -n "$comments_json" ]] || return 0
  node - "$comments_json" "$sha" <<'NODE'
const [raw, sourceSha] = process.argv.slice(2);
let bodies = [];
try { bodies = JSON.parse(raw); } catch (_) { /* leave bodies empty */ }
const matches = [];
for (const body of bodies) {
  if (typeof body !== 'string') continue;
  if (!body.startsWith('<!-- anvil-release-readiness-run -->')) continue;
  const newline = body.indexOf('\n');
  if (newline < 0) continue;
  try {
    const meta = JSON.parse(body.slice(newline + 1));
    if (meta && meta.sourceSha === sourceSha && Number.isInteger(meta.runId)) matches.push(meta);
  } catch (_) { /* ignore malformed markers */ }
}
if (matches.length) process.stdout.write(JSON.stringify(matches[matches.length - 1]));
NODE
}

# Posts a marker comment to the tracking issue recording a newly-dispatched
# readiness run id. Best-effort: missing tracking issue, missing gh, or a
# failed comment is silently ignored so the parent command continues.
persist_readiness_run_id() {
  local issue="$1"
  local sha="$2"
  local run_id="$3"
  local run_url="$4"
  [[ -n "$issue" && -n "$run_id" ]] || return 0
  command -v gh >/dev/null 2>&1 || return 0
  local body
  body="$(node - "$sha" "$run_id" "$run_url" <<'NODE'
const [sourceSha, runId, runUrl] = process.argv.slice(2);
process.stdout.write(`<!-- anvil-release-readiness-run -->\n${JSON.stringify({ sourceSha, runId: Number(runId), runUrl: runUrl || null, dispatchedAt: new Date().toISOString() }, null, 2)}`);
NODE
)"
  gh issue comment "$issue" --repo "$repo" --body "$body" >/dev/null 2>&1 || true
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
    --request-readiness) request_readiness=true; shift ;;
    --channel) require_value "$1" "${2:-}"; channel="$2"; shift 2 ;;
    --base-boundary) require_value "$1" "${2:-}"; base_boundary="$2"; shift 2 ;;
    --repo) require_value "$1" "${2:-}"; repo="$2"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) fail_usage "unknown argument: $1" ;;
  esac
done

check_promote_fake_guard

if ! git rev-parse --show-toplevel >/dev/null 2>&1; then
  emit_envelope "failed" "$(empty_data_json)" "$(failure_json invalid-input "not inside a git repository" false run-from-repository)" "promote" "Run promote from a git repository."
  exit 129
fi

[[ -n "$version" ]] || fail_usage "--version is required"
[[ "$version" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[A-Za-z0-9-]+(\.[A-Za-z0-9-]+)*)?$ ]] || fail_usage "--version must look like vX.Y.Z[-suffix]"
[[ "$strategy" == "direct" || "$strategy" == "stabilisation" ]] || fail_usage "--strategy must be direct or stabilisation"
if [[ "$request_readiness" == "true" ]]; then
  [[ "$channel" == "beta" || "$channel" == "stable" ]] || fail_usage "--channel beta|stable is required with --request-readiness"
  [[ -n "$base_boundary" ]] || fail_usage "--base-boundary is required with --request-readiness"
fi

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
  readiness: null,
  resumed: false,
}));
NODE
)"
  if [[ "$request_readiness" != "true" ]]; then
    emit_envelope "noop" "$data_json" "[]" "tag" "Target mode does not require promotion; tag will enforce release-readiness for the exact SHA."
    exit 0
  fi
  if [[ "$dry_run" == "true" ]]; then
    readiness_json="$(node - "$source_sha" <<'NODE'
const sourceSha = process.argv[2];
process.stdout.write(JSON.stringify({ state: 'would-request', runUrl: null, headSha: sourceSha }));
NODE
)"
  elif [[ -n "${ANVIL_RELEASE_PROMOTE_FAKE_GH_FILE:-}" ]]; then
    readiness_json="$(node - "$ANVIL_RELEASE_PROMOTE_FAKE_GH_FILE" "$source_sha" "$version" "$tracking_issue" "$channel" "$base_boundary" <<'NODE'
const fs = require('node:fs');
const [path, sourceSha, version, trackingIssue, channel, baseBoundary] = process.argv.slice(2);
const state = fs.existsSync(path) ? JSON.parse(fs.readFileSync(path, 'utf8')) : { nextPr: 1400, prs: [], runs: [], calls: [] };
state.runs ??= [];
state.calls ??= [];
let run = state.runs.find((candidate) => candidate.headSha === sourceSha && candidate.workflow === 'release-readiness.yml');
if (!run) {
  run = { id: state.runs.length + 1, workflow: 'release-readiness.yml', headSha: sourceSha, status: 'queued', conclusion: null, url: `https://github.com/eddacraft/anvil-001/actions/runs/${state.runs.length + 1}` };
  state.runs.push(run);
  state.calls.push({ command: 'workflow-run', workflow: 'release-readiness.yml', sourceSha, mode: 'readiness', channel, baseBoundary, requestedVersion: version, trackingIssue });
}
fs.writeFileSync(path, JSON.stringify(state, null, 2) + '\n');
process.stdout.write(JSON.stringify({ state: run.conclusion === 'success' ? 'passed' : 'requested', runUrl: run.url, headSha: sourceSha }));
NODE
)"
  else
    command -v gh >/dev/null 2>&1 || {
      emit_envelope "failed" "$data_json" "$(failure_json infra-failed "gh is required to request release-readiness" true install-gh)" "promote" "Install/authenticate gh or rerun with --dry-run."
      exit 127
    }
    readiness_json=""
    persisted_marker="$(find_persisted_readiness_run "$tracking_issue" "$source_sha" 2>/dev/null || true)"
    if [[ -n "$persisted_marker" ]]; then
      persisted_run_id="$(node -e "process.stdout.write(String(JSON.parse(process.argv[1]).runId))" "$persisted_marker")"
      persisted_run_json="$(gh run view "$persisted_run_id" --repo "$repo" --json databaseId,headSha,status,conclusion,url 2>/dev/null || true)"
      if [[ -n "$persisted_run_json" ]]; then
        readiness_json="$(node - "$persisted_run_json" "$source_sha" <<'NODE'
const [runRaw, sourceSha] = process.argv.slice(2);
const run = JSON.parse(runRaw);
if (run.conclusion === 'success') process.stdout.write(JSON.stringify({ state: 'passed', runUrl: run.url, headSha: sourceSha }));
else if (run.conclusion && run.conclusion !== 'success') process.stdout.write(JSON.stringify({ state: 'failed', runUrl: run.url, headSha: sourceSha }));
else process.stdout.write(JSON.stringify({ state: 'in-progress', runUrl: run.url, headSha: sourceSha }));
NODE
)"
      fi
    fi
    if [[ -z "$readiness_json" ]]; then
      run_json="$(gh run list --repo "$repo" --workflow release-readiness.yml --json databaseId,headSha,status,conclusion,url --limit 20 2>/dev/null || true)"
      readiness_json="$(node - "$run_json" "$source_sha" <<'NODE'
const [runsRaw, sourceSha] = process.argv.slice(2);
const runs = runsRaw ? JSON.parse(runsRaw) : [];
const run = runs.find((candidate) => candidate.headSha === sourceSha);
if (!run) process.stdout.write(JSON.stringify(null));
else if (run.conclusion === 'success') process.stdout.write(JSON.stringify({ state: 'passed', runUrl: run.url, headSha: sourceSha }));
else if (run.conclusion && run.conclusion !== 'success') process.stdout.write(JSON.stringify({ state: 'failed', runUrl: run.url, headSha: sourceSha }));
else process.stdout.write(JSON.stringify({ state: 'in-progress', runUrl: run.url, headSha: sourceSha }));
NODE
)"
    fi
    if [[ "$readiness_json" == "null" ]]; then
      dispatch_started_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
      gh workflow run release-readiness.yml --repo "$repo" --ref main \
        --field sourceSha="$source_sha" --field mode=readiness --field channel="$channel" \
        --field expectedReachableFrom=main --field baseBoundary="$base_boundary" \
        --field requestedVersion="$version" --field trackingIssue="$tracking_issue" >/dev/null 2>&1 || {
        emit_envelope "failed" "$data_json" "$(failure_json infra-failed "failed to dispatch release-readiness workflow" true retry-readiness-dispatch)" "promote" "Retry readiness dispatch after checking workflow permissions."
        exit 1
      }
      sleep "${ANVIL_RELEASE_PROMOTE_DISPATCH_SETTLE_SECONDS:-3}"
      dispatched_run_json="$(gh run list --repo "$repo" --workflow release-readiness.yml --event workflow_dispatch --limit 10 --json databaseId,url,createdAt --jq "[.[] | select(.createdAt >= \"$dispatch_started_at\")] | sort_by(.createdAt) | .[0] // empty" 2>/dev/null || true)"
      dispatched_run_id=""
      dispatched_run_url=""
      if [[ -n "$dispatched_run_json" ]]; then
        dispatched_run_id="$(node -e "process.stdout.write(String(JSON.parse(process.argv[1]).databaseId||''))" "$dispatched_run_json")"
        dispatched_run_url="$(node -e "process.stdout.write(JSON.parse(process.argv[1]).url||'')" "$dispatched_run_json")"
        persist_readiness_run_id "$tracking_issue" "$source_sha" "$dispatched_run_id" "$dispatched_run_url"
      fi
      readiness_json="$(node - "$source_sha" "$dispatched_run_url" <<'NODE'
const [sourceSha, runUrl] = process.argv.slice(2);
process.stdout.write(JSON.stringify({ state: 'requested', runUrl: runUrl || null, headSha: sourceSha }));
NODE
)"
    fi
  fi
  data_json="$(node - "$data_json" "$readiness_json" <<'NODE'
const data = JSON.parse(process.argv[2]);
data.readiness = JSON.parse(process.argv[3]);
process.stdout.write(JSON.stringify(data));
NODE
)"
  readiness_state="$(node -e "const r=JSON.parse(process.argv[1]); process.stdout.write(r.state);" "$readiness_json")"
  if [[ "$readiness_state" == "failed" ]]; then
    emit_envelope "blocked" "$data_json" "$(failure_json validation-failed "release-readiness workflow failed" true inspect-readiness-run)" "promote" "Inspect the readiness run and rerun promote after recovery."
    exit 1
  fi
  next_command="$(node -e "const r=JSON.parse(process.argv[1]); process.stdout.write(r.state === 'passed' ? 'tag' : 'promote');" "$readiness_json")"
  next_reason="$(node -e "const r=JSON.parse(process.argv[1]); process.stdout.write(r.state === 'passed' ? 'Readiness passed; continue to tag.' : 'Readiness requested or running; resume promote after it passes.');" "$readiness_json")"
  emit_envelope "success" "$data_json" "[]" "$next_command" "$next_reason"
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
  if [[ -n "${ANVIL_RELEASE_PROMOTE_FAKE_GH_FILE:-}" ]]; then
    result_json="$(node - "$ANVIL_RELEASE_PROMOTE_FAKE_GH_FILE" "$version" "$strategy" "$base_ref" "$head_ref" "$head_sha" "$tracking_issue" "$request_readiness" "$channel" "$base_boundary" <<'NODE'
const fs = require('node:fs');
const [path, version, strategy, baseRef, headRef, headSha, trackingIssue, requestReadinessRaw, channel, baseBoundary] = process.argv.slice(2);
const state = fs.existsSync(path) ? JSON.parse(fs.readFileSync(path, 'utf8')) : { nextPr: 1400, prs: [], runs: [], calls: [] };
state.nextPr ??= 1400;
state.prs ??= [];
state.runs ??= [];
state.calls ??= [];
let pr = state.prs.find((candidate) => candidate.base === baseRef && candidate.head === headRef && candidate.state !== 'CLOSED');
let resumed = Boolean(pr);
if (!pr) {
  pr = {
    number: state.nextPr++,
    url: `https://github.com/eddacraft/anvil-001/pull/${state.nextPr - 1}`,
    title: `Release ${version}`,
    base: baseRef,
    head: headRef,
    state: 'OPEN',
    mergeStateStatus: 'CLEAN',
    reviewDecision: 'REVIEW_REQUIRED',
    mergeCommit: null,
  };
  state.prs.push(pr);
  state.calls.push({ command: 'pr-create', base: baseRef, head: headRef, title: pr.title });
}

let status = 'success';
let exitCode = 0;
let failure = null;
let mergeState = 'awaiting-merge';
let mergedSha = null;
let nextCommand = 'promote';
let nextReason = 'Promotion PR is open; wait for merge.';
let readiness = null;

if (pr.mergeStateStatus === 'DIRTY') {
  status = 'blocked';
  exitCode = 1;
  failure = { code: 'remote-conflict', message: 'promotion PR has merge conflicts', retryable: true, recovery: 'resolve-conflicts', evidence: { command: 'gh pr view', url: pr.url, path: null } };
} else if (pr.reviewDecision === 'CHANGES_REQUESTED') {
  status = 'blocked';
  exitCode = 1;
  failure = { code: 'operator-required', message: 'promotion PR has requested changes', retryable: true, recovery: 'address-review', evidence: { command: 'gh pr view', url: pr.url, path: null } };
} else if (pr.state === 'MERGED' || pr.merged === true) {
  mergeState = 'merged';
  mergedSha = pr.mergeCommit?.oid || pr.mergeCommit || headSha;
  nextCommand = 'tag';
  nextReason = 'Promotion merged; continue to tag.';
  if (requestReadinessRaw === 'true') {
    let run = state.runs.find((candidate) => candidate.headSha === mergedSha && candidate.workflow === 'release-readiness.yml');
    if (!run) {
      run = { id: state.runs.length + 1, workflow: 'release-readiness.yml', headSha: mergedSha, status: 'queued', conclusion: null, url: `https://github.com/eddacraft/anvil-001/actions/runs/${state.runs.length + 1}` };
      state.runs.push(run);
      state.calls.push({ command: 'workflow-run', workflow: 'release-readiness.yml', sourceSha: mergedSha, mode: 'readiness', channel, baseBoundary, requestedVersion: version, trackingIssue });
      readiness = { state: 'requested', runUrl: run.url, headSha: mergedSha };
    } else {
      readiness = { state: run.conclusion === 'success' ? 'passed' : 'in-progress', runUrl: run.url, headSha: mergedSha };
    }
    nextCommand = readiness.state === 'passed' ? 'tag' : 'promote';
    nextReason = readiness.state === 'passed' ? 'Readiness already passed; continue to tag.' : 'Readiness requested or running; resume promote after it passes.';
  }
}

fs.writeFileSync(path, JSON.stringify(state, null, 2) + '\n');
process.stdout.write(JSON.stringify({
  exitCode,
  status,
  failures: failure ? [failure] : [],
  nextCommand,
  nextReason,
  data: {
    pullRequest: { number: pr.number, url: pr.url, title: pr.title, base: pr.base, head: pr.head, strategy },
    mergeState,
    mergedSha,
    operatorActionRequired: mergeState !== 'merged',
    reviewDecision: pr.reviewDecision || null,
    mergeStateStatus: pr.mergeStateStatus || null,
    readiness,
    resumed,
  },
}));
NODE
)"
    status="$(node -e "const r=JSON.parse(process.argv[1]); process.stdout.write(r.status);" "$result_json")"
    exit_code="$(node -e "const r=JSON.parse(process.argv[1]); process.stdout.write(String(r.exitCode));" "$result_json")"
    failures_json="$(node -e "const r=JSON.parse(process.argv[1]); process.stdout.write(JSON.stringify(r.failures));" "$result_json")"
    data_json="$(node -e "const r=JSON.parse(process.argv[1]); process.stdout.write(JSON.stringify(r.data));" "$result_json")"
    next_command="$(node -e "const r=JSON.parse(process.argv[1]); process.stdout.write(r.nextCommand);" "$result_json")"
    next_reason="$(node -e "const r=JSON.parse(process.argv[1]); process.stdout.write(r.nextReason);" "$result_json")"
    emit_envelope "$status" "$data_json" "$failures_json" "$next_command" "$next_reason"
    exit "$exit_code"
  fi

  command -v gh >/dev/null 2>&1 || {
    emit_envelope "failed" "$data_json" "$(failure_json infra-failed "gh is required for non-dry-run promote" true install-gh)" "promote" "Install/authenticate gh or rerun with --dry-run."
    exit 127
  }
  pr_json="$(gh pr list --repo "$repo" --base "$base_ref" --head "$head_ref" --state all --limit 1 --json number,url,title,state,mergeStateStatus,reviewDecision,baseRefName,headRefName,mergeCommit 2>/dev/null || true)"
  if [[ -z "$pr_json" ]]; then
    emit_envelope "failed" "$data_json" "$(failure_json auth-failed "failed to query promotion PRs" true gh-auth)" "promote" "Authenticate gh and rerun promote."
    exit 1
  fi
  pr_count="$(node -e "const prs=JSON.parse(process.argv[1]); process.stdout.write(String(prs.length));" "$pr_json")"
  resumed=true
  if [[ "$pr_count" == "0" ]]; then
    pr_url="$(gh pr create --repo "$repo" --base "$base_ref" --head "$head_ref" --title "Release $version" --body "Release promotion for $version." 2>/dev/null || true)"
    if [[ -z "$pr_url" ]]; then
      emit_envelope "failed" "$data_json" "$(failure_json auth-failed "failed to create promotion PR" true gh-auth)" "promote" "Authenticate gh and rerun promote."
      exit 1
    fi
    pr_json="$(gh pr view "$pr_url" --repo "$repo" --json number,url,title,state,mergeStateStatus,reviewDecision,baseRefName,headRefName,mergeCommit 2>/dev/null || true)"
    pr_json="[$pr_json]"
    resumed=false
  fi

  result_json="$(node - "$pr_json" "$version" "$strategy" "$head_sha" "$request_readiness" "$channel" "$base_boundary" "$resumed" <<'NODE'
const [prsRaw, version, strategy, headSha, requestReadinessRaw, channel, baseBoundary, resumedRaw] = process.argv.slice(2);
const pr = JSON.parse(prsRaw)[0];
let status = 'success';
let exitCode = 0;
let failure = null;
let mergeState = 'awaiting-merge';
let mergedSha = null;
let nextCommand = 'promote';
let nextReason = 'Promotion PR is open; wait for merge.';
if (pr.mergeStateStatus === 'DIRTY') {
  status = 'blocked';
  exitCode = 1;
  failure = { code: 'remote-conflict', message: 'promotion PR has merge conflicts', retryable: true, recovery: 'resolve-conflicts', evidence: { command: 'gh pr view', url: pr.url, path: null } };
} else if (pr.reviewDecision === 'CHANGES_REQUESTED') {
  status = 'blocked';
  exitCode = 1;
  failure = { code: 'operator-required', message: 'promotion PR has requested changes', retryable: true, recovery: 'address-review', evidence: { command: 'gh pr view', url: pr.url, path: null } };
} else if (pr.state === 'MERGED') {
  mergeState = 'merged';
  mergedSha = pr.mergeCommit?.oid || headSha;
  nextCommand = requestReadinessRaw === 'true' ? 'promote' : 'tag';
  nextReason = requestReadinessRaw === 'true' ? 'Promotion merged; request or resume readiness.' : 'Promotion merged; continue to tag.';
}
process.stdout.write(JSON.stringify({
  status,
  exitCode,
  failures: failure ? [failure] : [],
  nextCommand,
  nextReason,
  data: {
    pullRequest: { number: pr.number, url: pr.url, title: pr.title || `Release ${version}`, base: pr.baseRefName, head: pr.headRefName, strategy },
    mergeState,
    mergedSha,
    operatorActionRequired: mergeState !== 'merged',
    reviewDecision: pr.reviewDecision || null,
    mergeStateStatus: pr.mergeStateStatus || null,
    readiness: null,
    resumed: resumedRaw === 'true',
  },
}));
NODE
)"
  status="$(node -e "const r=JSON.parse(process.argv[1]); process.stdout.write(r.status);" "$result_json")"
  exit_code="$(node -e "const r=JSON.parse(process.argv[1]); process.stdout.write(String(r.exitCode));" "$result_json")"
  failures_json="$(node -e "const r=JSON.parse(process.argv[1]); process.stdout.write(JSON.stringify(r.failures));" "$result_json")"
  data_json="$(node -e "const r=JSON.parse(process.argv[1]); process.stdout.write(JSON.stringify(r.data));" "$result_json")"
  next_command="$(node -e "const r=JSON.parse(process.argv[1]); process.stdout.write(r.nextCommand);" "$result_json")"
  next_reason="$(node -e "const r=JSON.parse(process.argv[1]); process.stdout.write(r.nextReason);" "$result_json")"
  if [[ "$status" == "success" && "$request_readiness" == "true" ]]; then
    merged_sha="$(node -e "const d=JSON.parse(process.argv[1]); process.stdout.write(d.mergedSha || '');" "$data_json")"
    if [[ -n "$merged_sha" ]]; then
      run_json="$(gh run list --repo "$repo" --workflow release-readiness.yml --json databaseId,headSha,status,conclusion,url --limit 20 2>/dev/null || true)"
      readiness_json="$(node - "$run_json" "$merged_sha" <<'NODE'
const [runsRaw, mergedSha] = process.argv.slice(2);
const runs = runsRaw ? JSON.parse(runsRaw) : [];
const run = runs.find((candidate) => candidate.headSha === mergedSha);
if (!run) process.stdout.write(JSON.stringify(null));
else if (run.conclusion === 'success') process.stdout.write(JSON.stringify({ state: 'passed', runUrl: run.url, headSha: mergedSha }));
else if (run.conclusion && run.conclusion !== 'success') process.stdout.write(JSON.stringify({ state: 'failed', runUrl: run.url, headSha: mergedSha }));
else process.stdout.write(JSON.stringify({ state: 'in-progress', runUrl: run.url, headSha: mergedSha }));
NODE
)"
      if [[ "$readiness_json" == "null" ]]; then
        gh workflow run release-readiness.yml --repo "$repo" --ref main \
          --field sourceSha="$merged_sha" --field mode=readiness --field channel="$channel" \
          --field expectedReachableFrom=main --field baseBoundary="$base_boundary" \
          --field requestedVersion="$version" --field trackingIssue="$tracking_issue" >/dev/null 2>&1 || {
          emit_envelope "failed" "$data_json" "$(failure_json infra-failed "failed to dispatch release-readiness workflow" true retry-readiness-dispatch)" "promote" "Retry readiness dispatch after checking workflow permissions."
          exit 1
        }
        readiness_json="$(node - "$merged_sha" <<'NODE'
const mergedSha = process.argv[2];
process.stdout.write(JSON.stringify({ state: 'requested', runUrl: null, headSha: mergedSha }));
NODE
)"
      fi
      data_json="$(node - "$data_json" "$readiness_json" <<'NODE'
const data = JSON.parse(process.argv[2]);
data.readiness = JSON.parse(process.argv[3]);
process.stdout.write(JSON.stringify(data));
NODE
)"
      readiness_state="$(node -e "const r=JSON.parse(process.argv[1]); process.stdout.write(r.state);" "$readiness_json")"
      if [[ "$readiness_state" == "failed" ]]; then
        emit_envelope "blocked" "$data_json" "$(failure_json validation-failed "release-readiness workflow failed" true inspect-readiness-run)" "promote" "Inspect the readiness run and rerun promote after recovery."
        exit 1
      fi
      next_command="$(node -e "const r=JSON.parse(process.argv[1]); process.stdout.write(r.state === 'passed' ? 'tag' : 'promote');" "$readiness_json")"
      next_reason="$(node -e "const r=JSON.parse(process.argv[1]); process.stdout.write(r.state === 'passed' ? 'Readiness passed; continue to tag.' : 'Readiness requested or running; resume promote after it passes.');" "$readiness_json")"
    fi
  fi
  emit_envelope "$status" "$data_json" "$failures_json" "$next_command" "$next_reason"
  exit "$exit_code"
fi
