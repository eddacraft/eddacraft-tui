#!/usr/bin/env bash
set -euo pipefail

COMMAND="tag"
PHASE="tag"
SCHEMA_VERSION="1.0.0"
DEFAULT_REPO="eddacraft/anvil-001"

json=false
dry_run=false
recover=false
hotfix=false
repo="$DEFAULT_REPO"
version=""
source_sha=""
mode="target"
started_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

usage() {
  cat <<'USAGE'
Usage: tag.sh --version <vX.Y.Z[-suffix]> --source-sha <sha> [--json] [--dry-run] [--recover] [--hotfix]

Verify release tag preconditions and create or recover the release tag. Normal
mode refuses to retag an existing remote tag; --recover inspects remote state and
hands off to monitor when the remote tag already matches the expected SHA.

--hotfix waives the main-reachability gate so a release can be cut from a
hotfix branch tip that does not (yet) live on main. All other gates still
apply: origin remote, package.json version match for the source SHA,
release-readiness pass, and tag-conflict checks. Use only with explicit
operator approval per docs/runbooks/emergency-hotfix.md, and log the bypass on
the release tracking issue.
USAGE
}

now() { date -u +%Y-%m-%dT%H:%M:%SZ; }

failure_json() {
  node - "$1" "$2" "$3" "$4" <<'NODE'
const [code, message, retryableRaw, recovery] = process.argv.slice(2);
process.stdout.write(JSON.stringify([{ code, message, retryable: retryableRaw === 'true', recovery, evidence: { command: 'scripts/release/tag.sh', url: null, path: null } }]));
NODE
}

empty_data_json() { printf '%s' '{"tag":null,"tagSha":null,"sourceSha":null,"readinessRunUrl":null,"pushed":false,"recovery":null,"recoveryRequired":false}'; }

emit_envelope() {
  local status="$1" data_json="$2" failures_json="$3" next_command="$4" next_reason="$5" ended_at
  ended_at="$(now)"
  node - "$SCHEMA_VERSION" "$COMMAND" "$PHASE" "$status" "$started_at" "$ended_at" "$repo" "$mode" "$version" "$source_sha" "$data_json" "$failures_json" "$next_command" "$next_reason" <<'NODE'
const [schemaVersion, command, phase, status, startedAt, endedAt, repository, mode, version, sourceSha, dataRaw, failuresRaw, nextCommand, nextReason] = process.argv.slice(2);
process.stdout.write(JSON.stringify({
  schemaVersion, command, phase, mode, status, startedAt, endedAt, repository,
  inputs: { base: null, head: null, version: version || null, sourceSha: sourceSha || null },
  trackingIssue: { repository, number: null, url: null, metadataCommentUrl: null },
  releaseRecord: { lifecycleState: status === 'success' ? 'candidate' : null, recordUrl: null, sha256: null },
  data: JSON.parse(dataRaw), warnings: [], failures: JSON.parse(failuresRaw),
  next: { command: nextCommand, reason: nextReason },
}) + '\n');
NODE
}

fail_usage() {
  local message="$1"
  if [[ "$json" == "true" ]]; then
    emit_envelope "failed" "$(empty_data_json)" "$(failure_json invalid-input "$message" false correct-usage)" "tag" "Fix command arguments and rerun tag."
  else
    printf 'tag: %s\n' "$message" >&2
    usage >&2
  fi
  exit 129
}

require_value() { [[ -n "${2:-}" ]] || fail_usage "$1 requires a value"; }

remote_matches_repo() {
  local remote_url="$1"
  local expected_repo="$2"
  node - "$remote_url" "$expected_repo" <<'NODE'
const [remoteUrl, expectedRepo] = process.argv.slice(2);
const normalise = (value) => value
  .trim()
  .replace(/^git@github\.com:/i, '')
  .replace(/^https:\/\/github\.com\//i, '')
  .replace(/^ssh:\/\/git@github\.com\//i, '')
  .replace(/\.git$/i, '')
  .toLowerCase();
process.exit(normalise(remoteUrl) === normalise(expectedRepo) ? 0 : 1);
NODE
}

emit_blocked() {
  local code="$1" message="$2" retryable="$3" recovery="$4" next_reason="$5"
  emit_envelope "blocked" "$(empty_data_json)" "$(failure_json "$code" "$message" "$retryable" "$recovery")" "tag" "$next_reason"
}

while (($# > 0)); do
  case "$1" in
    --json) json=true; shift ;;
    --dry-run) dry_run=true; shift ;;
    --recover) recover=true; shift ;;
    --hotfix) hotfix=true; shift ;;
    --version) require_value "$1" "${2:-}"; version="$2"; shift 2 ;;
    --source-sha) require_value "$1" "${2:-}"; source_sha="$2"; shift 2 ;;
    --repo) require_value "$1" "${2:-}"; repo="$2"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) fail_usage "unknown argument: $1" ;;
  esac
done

if [[ -n "${ANVIL_RELEASE_TAG_FAKE_REMOTE_FILE:-}" && "${ANVIL_RELEASE_TEST_MODE:-}" != "tag-fake-remote" ]]; then
  emit_envelope "failed" "$(empty_data_json)" "$(failure_json invalid-input "ANVIL_RELEASE_TAG_FAKE_REMOTE_FILE requires ANVIL_RELEASE_TEST_MODE=tag-fake-remote" false correct-test-usage)" "tag" "Unset the test hook or enable explicit fake remote test mode."
  exit 129
fi
if [[ -n "${ANVIL_RELEASE_TAG_FAKE_READINESS_FILE:-}" && "${ANVIL_RELEASE_TEST_MODE:-}" != "tag-fake-readiness" ]]; then
  emit_envelope "failed" "$(empty_data_json)" "$(failure_json invalid-input "ANVIL_RELEASE_TAG_FAKE_READINESS_FILE requires ANVIL_RELEASE_TEST_MODE=tag-fake-readiness" false correct-test-usage)" "tag" "Unset the test hook or enable explicit fake readiness test mode."
  exit 129
fi

[[ -n "$version" ]] || fail_usage "--version is required"
[[ "$version" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[A-Za-z0-9-]+(\.[A-Za-z0-9-]+)*)?$ ]] || fail_usage "--version must look like vX.Y.Z[-suffix]"
[[ -n "$source_sha" ]] || fail_usage "--source-sha is required"
[[ "$source_sha" =~ ^[0-9a-fA-F]{40}$ ]] || fail_usage "--source-sha requires a full 40-character commit SHA"
git rev-parse --verify "${source_sha}^{commit}" >/dev/null 2>&1 || fail_usage "source SHA is not a commit: $source_sha"

if [[ -n "${ANVIL_RELEASE_TAG_FAKE_REMOTE_FILE:-}" ]]; then
  result="$(node - "$ANVIL_RELEASE_TAG_FAKE_REMOTE_FILE" "$version" "$source_sha" "$dry_run" "$recover" <<'NODE'
const fs = require('node:fs');
const [path, version, sourceSha, dryRunRaw, recoverRaw] = process.argv.slice(2);
const state = fs.existsSync(path) ? JSON.parse(fs.readFileSync(path, 'utf8')) : { tags: {} };
state.tags ??= {};
const existing = state.tags[version] || null;
let status = 'success', exitCode = 0, failure = null, recovery = null, pushed = false;
if (existing && existing !== sourceSha && recoverRaw !== 'true') {
  status = 'blocked'; exitCode = 1; failure = { code: 'remote-conflict', message: 'remote tag exists at a different SHA', retryable: false, recovery: 'recover-tag', evidence: { command: 'tag fake remote', url: null, path } };
} else if (existing && existing === sourceSha && recoverRaw !== 'true') {
  status = 'blocked'; exitCode = 1; failure = { code: 'operator-required', message: 'tag already pushed; rerun with --recover', retryable: true, recovery: 'rerun-with-recover', evidence: { command: 'tag fake remote', url: null, path } };
    } else if (existing && recoverRaw === 'true') {
  recovery = 'remote-tag-matches';
} else if (dryRunRaw !== 'true') {
  state.tags[version] = sourceSha; pushed = true;
}
fs.writeFileSync(path, JSON.stringify(state, null, 2) + '\n');
process.stdout.write(JSON.stringify({ status, exitCode, failures: failure ? [failure] : [], data: { tag: version, tagSha: sourceSha, sourceSha, readinessRunUrl: null, pushed, recovery, recoveryRequired: Boolean(failure) } }));
NODE
)"
  status="$(node -e "const r=JSON.parse(process.argv[1]); process.stdout.write(r.status);" "$result")"
  exit_code="$(node -e "const r=JSON.parse(process.argv[1]); process.stdout.write(String(r.exitCode));" "$result")"
  data_json="$(node -e "const r=JSON.parse(process.argv[1]); process.stdout.write(JSON.stringify(r.data));" "$result")"
  failures_json="$(node -e "const r=JSON.parse(process.argv[1]); process.stdout.write(JSON.stringify(r.failures));" "$result")"
  emit_envelope "$status" "$data_json" "$failures_json" "monitor" "Tag state is resolved; monitor the release workflow."
  exit "$exit_code"
fi

readiness_state="unknown"
readiness_run_url=""
readiness_check() {
  if [[ -n "${ANVIL_RELEASE_TAG_FAKE_READINESS_FILE:-}" ]]; then
    result="$(node - "$ANVIL_RELEASE_TAG_FAKE_READINESS_FILE" "$source_sha" <<'NODE'
const fs = require('node:fs');
const [path, sourceSha] = process.argv.slice(2);
const report = JSON.parse(fs.readFileSync(path, 'utf8'));
const runs = Array.isArray(report.runs) ? report.runs : [report];
const run = runs.find((candidate) => candidate.headSha === sourceSha || candidate.sourceSha === sourceSha);
if (!run) process.stdout.write(JSON.stringify({ state: 'missing', runUrl: null, exitCode: 1 }));
else if (run.conclusion === 'success' || run.state === 'passed') process.stdout.write(JSON.stringify({ state: 'passed', runUrl: run.url || run.runUrl || null, exitCode: 0 }));
else process.stdout.write(JSON.stringify({ state: run.conclusion || run.state || 'failed', runUrl: run.url || run.runUrl || null, exitCode: 1 }));
NODE
    )"
    readiness_state="$(node -e "const r=JSON.parse(process.argv[1]); process.stdout.write(r.state);" "$result")"
    readiness_run_url="$(node -e "const r=JSON.parse(process.argv[1]); process.stdout.write(r.runUrl || '');" "$result")"
    return "$(node -e "const r=JSON.parse(process.argv[1]); process.stdout.write(String(r.exitCode));" "$result")"
  fi

  command -v gh >/dev/null 2>&1 || { readiness_state="infra-missing-gh"; return 2; }
  local run_json
  run_json="$(gh run list --repo "$repo" --workflow release-readiness.yml --json headSha,conclusion,status,url --limit 20 2>/dev/null || true)"
  if [[ -z "$run_json" ]]; then
    readiness_state="infra-gh-run-list"
    return 2
  fi
  result="$(node - "$run_json" "$source_sha" <<'NODE'
const [runsRaw, sourceSha] = process.argv.slice(2);
const runs = runsRaw ? JSON.parse(runsRaw) : [];
const run = runs.find((candidate) => candidate.headSha === sourceSha);
if (!run) process.stdout.write(JSON.stringify({ state: 'missing', runUrl: null, exitCode: 1 }));
else if (run.conclusion === 'success') process.stdout.write(JSON.stringify({ state: 'passed', runUrl: run.url || null, exitCode: 0 }));
else process.stdout.write(JSON.stringify({ state: run.conclusion || run.status || 'failed', runUrl: run.url || null, exitCode: 1 }));
NODE
  )"
  readiness_state="$(node -e "const r=JSON.parse(process.argv[1]); process.stdout.write(r.state);" "$result")"
  readiness_run_url="$(node -e "const r=JSON.parse(process.argv[1]); process.stdout.write(r.runUrl || '');" "$result")"
  return "$(node -e "const r=JSON.parse(process.argv[1]); process.stdout.write(String(r.exitCode));" "$result")"
}

remote_url="$(git remote get-url origin 2>/dev/null || true)"
if [[ -z "$remote_url" ]]; then
  emit_blocked remote-conflict "origin remote is not configured" false configure-origin "Configure the origin remote before tagging."
  exit 1
fi
if ! remote_matches_repo "$remote_url" "$repo"; then
  emit_blocked remote-conflict "origin remote does not match --repo $repo" false fix-origin "Run tag from the intended repository checkout."
  exit 1
fi

if [[ "$hotfix" != "true" ]]; then
  if ! git merge-base --is-ancestor "$source_sha" main >/dev/null 2>&1; then
    emit_blocked stale-source "source SHA is not reachable from local main" true update-main "Update local main or pass the promoted source SHA."
    exit 1
  fi
  if ! git fetch --quiet origin main >/dev/null 2>&1; then
    emit_blocked infra-failed "failed to fetch origin/main before tagging" true fetch-origin-main "Check network/auth and rerun tag."
    exit 1
  fi
  local_main_sha="$(git rev-parse --verify main 2>/dev/null || true)"
  remote_main_sha="$(git rev-parse --verify origin/main 2>/dev/null || true)"
  if [[ "$local_main_sha" != "$remote_main_sha" || "$source_sha" != "$local_main_sha" ]]; then
    emit_blocked stale-source "source SHA must equal both local main and origin/main" true update-main "Update main and rerun tag with the promoted SHA."
    exit 1
  fi
fi
package_version="$(git show "${source_sha}:package.json" 2>/dev/null | node -e "let input=''; process.stdin.on('data', c => input += c); process.stdin.on('end', () => { if (!input) return; const doc = JSON.parse(input); process.stdout.write(doc.version || ''); });" || true)"
if [[ -n "$package_version" && "v${package_version}" != "$version" ]]; then
  emit_blocked stale-source "package.json version does not match requested tag" true rerun-prepare "Run prepare for the requested version before tagging."
  exit 1
fi
readiness_rc=0
readiness_check || readiness_rc=$?
data_json="$(node - "$version" "$source_sha" "$dry_run" "$readiness_run_url" <<'NODE'
const [tag, sourceSha, dryRunRaw, readinessRunUrl] = process.argv.slice(2);
process.stdout.write(JSON.stringify({ tag, tagSha: sourceSha, sourceSha, readinessRunUrl: readinessRunUrl || null, pushed: dryRunRaw !== 'true', recovery: null, recoveryRequired: false }));
NODE
)"
if [[ "$readiness_rc" == "2" ]]; then
  emit_envelope "blocked" "$data_json" "$(failure_json infra-failed "could not check release-readiness state: $readiness_state" true install-or-auth-gh)" "tag" "Install/authenticate gh, or rerun with explicit guarded readiness evidence."
  exit 1
elif [[ "$readiness_rc" != "0" ]]; then
  emit_envelope "blocked" "$data_json" "$(failure_json validation-failed "release-readiness has not passed for source SHA" true run-readiness)" "tag" "Run or recover release-readiness for the exact source SHA before tagging."
  exit 1
fi

local_tag_sha="$(git rev-parse --verify --quiet "refs/tags/${version}^{}" 2>/dev/null || true)"
if [[ -n "$local_tag_sha" && "$local_tag_sha" != "$source_sha" ]]; then
  emit_blocked remote-conflict "local tag exists at a different SHA" false inspect-local-tag "Inspect or remove the local tag before retrying."
  exit 1
fi

remote_tag_sha="$(git ls-remote --tags origin "refs/tags/${version}^{}" 2>/dev/null | awk '{print $1}' | head -n 1 || true)"
if [[ -z "$remote_tag_sha" ]]; then
  remote_tag_sha="$(git ls-remote --tags origin "refs/tags/${version}" 2>/dev/null | awk '{print $1}' | head -n 1 || true)"
fi
if [[ -n "$remote_tag_sha" && "$remote_tag_sha" != "$source_sha" ]]; then
  emit_blocked remote-conflict "remote tag exists at a different SHA" false inspect-remote-tag "Inspect the remote tag before retrying."
  exit 1
fi
if [[ -n "$remote_tag_sha" && "$remote_tag_sha" == "$source_sha" ]]; then
  if [[ "$recover" != "true" ]]; then
    emit_blocked operator-required "tag already pushed; rerun with --recover" true rerun-with-recover "Confirm the remote tag state before continuing."
    exit 1
  fi
  recovery_data="$(node - "$version" "$source_sha" "$readiness_run_url" <<'NODE'
const [tag, sourceSha, readinessRunUrl] = process.argv.slice(2);
process.stdout.write(JSON.stringify({ tag, tagSha: sourceSha, sourceSha, readinessRunUrl: readinessRunUrl || null, pushed: false, recovery: 'remote-tag-matches', recoveryRequired: false }));
NODE
)"
  emit_envelope "success" "$recovery_data" "[]" "monitor" "Remote tag already matches; monitor release workflow."
  exit 0
fi

if [[ "$dry_run" == "true" ]]; then
  emit_envelope "success" "$data_json" "[]" "monitor" "Dry-run tag preconditions are valid."
else
  if [[ -z "$local_tag_sha" ]]; then
    git tag -a "$version" "$source_sha" -m "Release $version"
  fi
  if ! git push origin "$version" >/dev/null 2>&1; then
    emit_envelope "recoverable" "$(empty_data_json)" "$(failure_json remote-conflict "failed to push tag; local tag may already exist" true rerun-with-recover)" "tag" "Inspect the remote tag and rerun with --recover if it matches."
    exit 1
  fi
  emit_envelope "success" "$data_json" "[]" "monitor" "Tag pushed; monitor release workflow."
fi
