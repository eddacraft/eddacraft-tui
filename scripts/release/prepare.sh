#!/usr/bin/env bash
set -euo pipefail

COMMAND="prepare"
PHASE="prepare"
SCHEMA_VERSION="1.0.0"
DEFAULT_REPO="eddacraft/anvil-001"
# Resolved from this script's own location so the changelog promoter is found
# regardless of the caller's working directory (prepare runs from the repo root
# of the release checkout, which is not necessarily this checkout).
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

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
metadata_comment_url=""

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

check_prepare_fake_guard() {
  if [[ -n "${ANVIL_RELEASE_PREPARE_FAKE_ISSUES_FILE:-}" && "${ANVIL_RELEASE_TEST_MODE:-}" != "prepare-fake-gh" ]]; then
    emit_envelope "failed" "$(empty_data_json)" "$(failure_json invalid-input "ANVIL_RELEASE_PREPARE_FAKE_ISSUES_FILE requires ANVIL_RELEASE_TEST_MODE=prepare-fake-gh" false correct-test-usage)" "prepare" "Unset the test hook or enable explicit prepare fake GitHub test mode."
    exit 129
  fi
}

release_version_without_prefix() {
  printf '%s' "${version#v}"
}

# Operator-facing tracking issue body. Checkbox rules are the agent contract:
# a step is approved only when its box is `[x]`/`[X]` and the Signed-off-by
# line under that step is non-empty. Unchecked or unsigned = not approved.
tracking_issue_body() {
  node - "$version" "$release_type" "$strategy" "${source_sha:-}" <<'NODE'
const [version, releaseType, strategy, sourceSha] = process.argv.slice(2);
const sha = sourceSha || '_(filled by prepare metadata comment)_';
process.stdout.write(`Release tracking issue for \`${version}\`.

## Intent

| Field | Value |
| --- | --- |
| Version | \`${version}\` |
| Type | ${releaseType} |
| Strategy | ${strategy} |
| Source SHA (at prepare) | \`${sha}\` |

Operator log and **approval surface** for supporting agents. Shipped-state
authority remains the release record and APS; this issue does not replace
\`plans/releases/<tag>.md\`.

## Operator approvals

**Agent rules (do not invent authority):**

1. A step is **approved** only when its checkbox is \`[x]\` or \`[X]\` **and**
   the \`Signed-off-by:\` line under that step names a human operator
   (GitHub login or real name). A bare check without a sign-off is **not**
   approval.
2. Unchecked (\`[ ]\`) or missing sign-off = **not approved**. Do not proceed
   past that gate.
3. \`tag.sh\` requires **Tag authority** checked and signed. Pre-tag review
   artefacts may still be required by the runbook; the issue gate is the
   agent-readable go/no-go.
4. Operators tick boxes by editing this issue body (preferred) so the top of
   the issue stays the single source of truth.

### Claim freeze

- [ ] Claim locked — theme and primary claim IDs accepted for this cut
  - Signed-off-by:
  - At (UTC):

### Preflight and readiness

- [ ] Local preflight green (\`scripts/release/preflight.sh\`)
  - Signed-off-by:
  - At (UTC):
- [ ] Release readiness green on the **exact** source SHA
  - Run URL:
  - Signed-off-by:
  - At (UTC):

### Pre-tag review

- [ ] Pre-tag review complete (focused or full per \`docs/runbooks/release-process.md\`)
  - Artefact path (if any):
  - Signed-off-by:
  - At (UTC):
- [ ] Human gate — authorised to tag
  - Signed-off-by:
  - At (UTC):

### Tag and publish

- [ ] **Tag authority** — \`tag.sh\` for \`${version}\` may run on the frozen SHA
  - Signed-off-by:
  - At (UTC):
- [ ] Publish / monitor / verify may proceed after tag (optional batch tick)
  - Signed-off-by:
  - At (UTC):

### Closeout

- [ ] Closeout hygiene (record filled, APS advance, \`RELEASE-PLAN\` roll) may merge
  - Signed-off-by:
  - At (UTC):

## Links

- Runbook: \`docs/runbooks/release-runbook.md\`
- Pre-tag doctrine: \`docs/runbooks/release-process.md\`
- Cut actions: \`plans/execution/${version}.cut.actions.md\` (when present)
- Closeout skeleton: \`plans/releases/${version}.md\` (when present)
`);
NODE
}

ensure_tracking_issue() {
  if [[ -n "${ANVIL_RELEASE_PREPARE_FAKE_ISSUES_FILE:-}" ]]; then
    node - "$ANVIL_RELEASE_PREPARE_FAKE_ISSUES_FILE" "$repo" "$version" "$tracking_issue" <<'NODE'
const fs = require('node:fs');
const [path, repo, version, requested] = process.argv.slice(2);
const state = fs.existsSync(path) ? JSON.parse(fs.readFileSync(path, 'utf8')) : { nextNumber: 1234, issues: [] };
state.nextNumber ??= 1234;
state.issues ??= [];
const requestedNumber = /^\d+$/.test(requested) ? Number(requested) : null;
let issue = requestedNumber ? state.issues.find((candidate) => candidate.number === requestedNumber) : null;
if (!issue && requested) {
  issue = state.issues.find((candidate) => candidate.url === requested);
}
if (!issue) {
  issue = {
    number: requestedNumber || state.nextNumber++,
    url: requested && /^https?:/.test(requested) ? requested : `https://github.com/${repo}/issues/${requestedNumber || state.nextNumber - 1}`,
    title: `Release ${version}`,
    comments: [],
  };
  state.issues.push(issue);
}
fs.writeFileSync(path, JSON.stringify(state, null, 2) + '\n');
process.stdout.write(JSON.stringify({ number: issue.number, url: issue.url }));
NODE
    return 0
  fi

  command -v gh >/dev/null 2>&1 || {
    emit_envelope "failed" "$(empty_data_json)" "$(failure_json infra-failed "gh is required for non-dry-run prepare" true install-gh)" "prepare" "Install/authenticate gh or rerun with --dry-run."
    exit 127
  }

  if [[ -n "$tracking_issue" ]]; then
    gh issue view "$tracking_issue" --repo "$repo" --json number,url 2>/dev/null || {
      emit_envelope "failed" "$(empty_data_json)" "$(failure_json auth-failed "failed to read tracking issue" true gh-auth-or-issue)" "prepare" "Authenticate gh or provide a valid tracking issue."
      exit 1
    }
  else
    local body
    body="$(tracking_issue_body)"
    # JSON stdin keeps multiline body intact (unlike -f body=).
    if ! node - "$version" "$body" <<'NODE' | gh api "repos/${repo}/issues" --method POST --input - --jq '{number, url: .html_url}' 2>/dev/null
const [version, body] = process.argv.slice(2);
process.stdout.write(JSON.stringify({
  title: `Release ${version}`,
  body,
  labels: ['release'],
}));
NODE
    then
      emit_envelope "failed" "$(empty_data_json)" "$(failure_json auth-failed "failed to create tracking issue" true gh-auth)" "prepare" "Authenticate gh and rerun prepare."
      exit 1
    fi
  fi
}

append_prepare_metadata() {
  local issue_number="$1"
  local issue_url="$2"
  local prep_sha="$3"
  local comment_body
  comment_body="$(node - "$version" "$source_sha" "$prep_sha" <<'NODE'
const [version, sourceSha, prepCommitSha] = process.argv.slice(2);
process.stdout.write(`<!-- anvil-release-metadata:v1 -->\n${JSON.stringify({ phase: 'prepare', version, sourceSha, prepCommitSha, recordedAt: new Date().toISOString() }, null, 2)}\n`);
NODE
)"

  if [[ -n "${ANVIL_RELEASE_PREPARE_FAKE_ISSUES_FILE:-}" ]]; then
    metadata_comment_url="$(node - "$ANVIL_RELEASE_PREPARE_FAKE_ISSUES_FILE" "$issue_number" "$comment_body" <<'NODE'
const fs = require('node:fs');
const [path, numberRaw, body] = process.argv.slice(2);
const state = JSON.parse(fs.readFileSync(path, 'utf8'));
const issue = state.issues.find((candidate) => candidate.number === Number(numberRaw));
if (!issue) throw new Error(`missing fake issue ${numberRaw}`);
issue.comments ??= [];
const url = `${issue.url}#issuecomment-${issue.comments.length + 1}`;
issue.comments.push({ url, body });
fs.writeFileSync(path, JSON.stringify(state, null, 2) + '\n');
process.stdout.write(url);
NODE
)"
    return 0
  fi

  metadata_comment_url="$(gh issue comment "$issue_number" --repo "$repo" --body "$comment_body" 2>/dev/null)"
}

apply_release_edits() {
  local file_version
  file_version="$(release_version_without_prefix)"

  # CIB-196: refuse a cut with nothing to promote BEFORE touching any version,
  # so a rejected prepare leaves a clean tree and the retry is not blocked on
  # half-bumped manifests.
  [[ -e CHANGELOG.md ]] || printf '%s\n' '# Changelog' >CHANGELOG.md
  local check_rc=0
  node "$SCRIPT_DIR/promote-changelog.mjs" \
    --check \
    --version "$version" \
    --date "$(date -u +%Y-%m-%d)" \
    ${ANVIL_RELEASE_ALLOW_EMPTY_CHANGELOG:+--allow-empty} || check_rc=$?
  if [[ "$check_rc" != "0" ]]; then
    return "$check_rc"
  fi

  # Bump root package.json + per-package.json files that share the pre-bump
  # root version. Writes the bumped path list to $tmp/version-bumps so the
  # caller can stage all of them.
  : >"$tmp/version-bumps"
  node - "$file_version" "$tmp/version-bumps" <<'NODE'
const fs = require('node:fs');
const path = require('node:path');
const [version, bumpListPath] = process.argv.slice(2);
const writes = [];
let priorVersion = null;
const root = 'package.json';
if (fs.existsSync(root)) {
  const doc = JSON.parse(fs.readFileSync(root, 'utf8'));
  priorVersion = doc.version || null;
  doc.version = version;
  fs.writeFileSync(root, JSON.stringify(doc, null, 2) + '\n');
  writes.push(root);
}
if (priorVersion && priorVersion !== version) {
  const IGNORED = new Set(['node_modules', '.git', '.next', 'dist', 'target', '.turbo', '.nx', '.pnpm-store']);
  const stack = ['.'];
  while (stack.length) {
    const dir = stack.pop();
    let entries;
    try { entries = fs.readdirSync(dir, { withFileTypes: true }); } catch { continue; }
    for (const entry of entries) {
      if (IGNORED.has(entry.name)) continue;
      const full = path.join(dir, entry.name);
      if (entry.isDirectory()) { stack.push(full); continue; }
      if (entry.name !== 'package.json') continue;
      if (full === root || full === `./${root}`) continue;
      let doc;
      try { doc = JSON.parse(fs.readFileSync(full, 'utf8')); } catch { continue; }
      if (doc.version !== priorVersion) continue;
      doc.version = version;
      fs.writeFileSync(full, JSON.stringify(doc, null, 2) + '\n');
      writes.push(full.replace(/^\.\//, ''));
    }
  }
}
fs.writeFileSync(bumpListPath, writes.join('\n') + (writes.length ? '\n' : ''));
NODE

  # Bump Cargo.toml workspace version + refresh Cargo.lock when present. Skip
  # silently when the workspace lacks a Cargo.toml or the workspace version is
  # already aligned.
  if [[ -e Cargo.toml ]]; then
    local cargo_rc=0
    node - "$file_version" <<'NODE' || cargo_rc=$?
const fs = require('node:fs');
const version = process.argv[2];
const text = fs.readFileSync('Cargo.toml', 'utf8');
const sectionMatch = text.match(/^\[workspace\.package\][^[]*?^version = "([^"]+)"/ms);
const simpleMatch = !sectionMatch ? text.match(/^version = "([^"]+)"/m) : null;
const match = sectionMatch || simpleMatch;
if (!match) process.exit(2);
if (match[1] === version) process.exit(1);
const updated = text.replace(match[0], match[0].replace(/version = "[^"]+"/, `version = "${version}"`));
fs.writeFileSync('Cargo.toml', updated);
process.exit(0);
NODE
    if [[ "$cargo_rc" == "0" ]]; then
      printf '%s\n' Cargo.toml >>"$tmp/version-bumps"
      if [[ -e Cargo.lock ]] && command -v cargo >/dev/null 2>&1; then
        if cargo update --workspace --offline --quiet >/dev/null 2>&1 \
          || cargo update --workspace --quiet >/dev/null 2>&1; then
          printf '%s\n' Cargo.lock >>"$tmp/version-bumps"
        fi
      fi
    fi
  fi

  # CIB-196: promote the `## [Unreleased]` draft into a real release section
  # rather than appending a metadata stub to the bottom of the file. The
  # promotion is idempotent and fails loudly when there is no draft to promote,
  # so an empty changelog is a decision the operator makes rather than a stub
  # that silently ships.
  mkdir -p docs/public/anvil/releases
  [[ -e CHANGELOG.md ]] || printf '%s\n' '# Changelog' >CHANGELOG.md
  local promote_rc=0
  node "$SCRIPT_DIR/promote-changelog.mjs" \
    --version "$version" \
    --date "$(date -u +%Y-%m-%d)" \
    ${ANVIL_RELEASE_ALLOW_EMPTY_CHANGELOG:+--allow-empty} || promote_rc=$?
  if [[ "$promote_rc" != "0" ]]; then
    return "$promote_rc"
  fi
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
    metadataCommentUrl: process.env.ANVIL_RELEASE_METADATA_COMMENT_URL || null,
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

check_prepare_fake_guard

if ! git rev-parse --show-toplevel >/dev/null 2>&1; then
  emit_envelope "failed" "$(empty_data_json)" "$(failure_json invalid-input "not inside a git repository" false run-from-repository)" "prepare" "Run prepare from a git repository."
  exit 129
fi

[[ -n "$version" ]] || fail_usage "--version is required"
[[ "$version" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[A-Za-z0-9-]+(\.[A-Za-z0-9-]+)*)?$ ]] || fail_usage "--version must look like vX.Y.Z[-suffix]"
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
for path in package.json CHANGELOG.md docs/public/anvil/releases/changelog.md Cargo.toml Cargo.lock; do
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
  issue_json="$(ensure_tracking_issue)"
  tracking_issue="$(node -e "const issue=JSON.parse(process.argv[1]); process.stdout.write(String(issue.number));" "$issue_json")"
  tracking_issue_url="$(node -e "const issue=JSON.parse(process.argv[1]); process.stdout.write(issue.url);" "$issue_json")"
  apply_release_edits
  : >"$tmp/commit-files"
  [[ -s "$tmp/version-bumps" ]] && cat "$tmp/version-bumps" >>"$tmp/commit-files"
  for path in CHANGELOG.md docs/public/anvil/releases/changelog.md; do
    [[ -e "$path" ]] && printf '%s\n' "$path" >>"$tmp/commit-files"
  done
  commit_files=()
  while IFS= read -r commit_file_line; do
    [[ -n "$commit_file_line" ]] && commit_files+=("$commit_file_line")
  done <"$tmp/commit-files"
  if [[ ${#commit_files[@]} -gt 0 ]] && [[ -n "$(git status --porcelain -- "${commit_files[@]}")" ]]; then
    git add -- "${commit_files[@]}"
    git commit -m "chore(release): prepare $version" >/dev/null
  fi
  changed_files_json="$(json_array_from_file "$tmp/commit-files")"
  prep_commit_sha="$(git rev-parse HEAD)"
  data_json="$(node - "$version" "$release_type" "$strategy" "$source_sha" "$tracking_issue_url" "$request_readiness" "$request_candidate_artifacts" "$prep_commit_sha" "$changed_files_json" <<'NODE'
const [version, releaseType, strategy, sourceSha, trackingIssueUrl, readinessRaw, artifactsRaw, prepCommitSha, changedFilesRaw] = process.argv.slice(2);
process.stdout.write(JSON.stringify({
  prepCommitSha,
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
  if ! append_prepare_metadata "$tracking_issue" "$tracking_issue_url" "$prep_commit_sha"; then
    emit_envelope "recoverable" "$data_json" "$(failure_json auth-failed "failed to append release tracking metadata" true retry-prepare-metadata)" "prepare" "Fix GitHub issue permissions or network state, then rerun prepare."
    exit 1
  fi
  ANVIL_RELEASE_METADATA_COMMENT_URL="$metadata_comment_url" emit_envelope "success" "$data_json" "[]" "promote" "Preparation committed and tracking metadata recorded."
fi
