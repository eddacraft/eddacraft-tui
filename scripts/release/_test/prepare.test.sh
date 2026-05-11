#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
HARNESS="$ROOT/scripts/release/_test/harness.sh"
PREPARE="$ROOT/scripts/release/prepare.sh"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

assert_contains() {
  local haystack="$1"
  local needle="$2"
  if [[ "$haystack" != *"$needle"* ]]; then
    echo "expected output to contain: $needle" >&2
    echo "actual output:" >&2
    echo "$haystack" >&2
    exit 1
  fi
}

init_repo() {
  local repo="$1"
  mkdir -p "$repo/docs/public/anvil/releases"
  git -C "$repo" init -q
  git -C "$repo" config user.email relorch@example.invalid
  git -C "$repo" config user.name "RELORCH Test"
  printf '%s\n' '{"version":"0.6.1-beta"}' >"$repo/package.json"
  printf '%s\n' '# Changelog' >"$repo/CHANGELOG.md"
  printf '%s\n' '# Public changelog' >"$repo/docs/public/anvil/releases/changelog.md"
  git -C "$repo" add .
  git -C "$repo" commit -q -m "chore: initial fixture"
}

repo="$tmp/prepare-repo"
init_repo "$repo"

bash "$HARNESS" run-contract \
  --name prepare-dry-run \
  --expected-exit 0 \
  --expected-command prepare \
  -- bash -c 'cd "$1" && bash "$2" --json --dry-run --version v0.7.0-beta --release-type beta --strategy direct --repo eddacraft/anvil-001' _ "$repo" "$PREPARE"

(cd "$repo" && bash "$PREPARE" --json --dry-run --version v0.7.0-beta --release-type beta --strategy direct) >"$tmp/prepare.json"
node - "$tmp/prepare.json" "$(git -C "$repo" rev-parse HEAD)" <<'NODE'
const fs = require('node:fs');
const [path, expectedSha] = process.argv.slice(2);
const doc = JSON.parse(fs.readFileSync(path, 'utf8'));
if (doc.status !== 'success') throw new Error(`expected success, got ${doc.status}`);
if (doc.data.prepCommitSha !== null) throw new Error('dry-run must not produce a prep commit');
if (!Array.isArray(doc.data.changedFiles) || !doc.data.changedFiles.includes('package.json')) {
  throw new Error(`expected package.json in changedFiles, got ${doc.data.changedFiles}`);
}
if (doc.data.trackingIssueUrl !== null) throw new Error('dry-run without tracking issue should not invent one');
if (doc.data.candidateMetadata.version !== 'v0.7.0-beta') throw new Error('wrong candidate metadata version');
if (doc.data.candidateMetadata.sourceSha !== expectedSha) throw new Error('wrong candidate source SHA');
if (typeof doc.data.idempotencyKey !== 'string' || !doc.data.idempotencyKey.includes('v0.7.0-beta')) {
  throw new Error('missing deterministic idempotency key');
}
NODE

printf '%s\n' 'dirty' >"$repo/dirty.txt"
rc=0
(cd "$repo" && bash "$PREPARE" --json --version v0.7.0-beta --release-type beta --strategy direct >"$tmp/dirty.json") || rc=$?
if [[ "$rc" != "1" ]]; then
  echo "expected dirty worktree exit 1, got $rc" >&2
  exit 1
fi
node - "$tmp/dirty.json" <<'NODE'
const fs = require('node:fs');
const doc = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
if (!doc.failures.some((failure) => failure.code === 'dirty-worktree')) {
  throw new Error('expected dirty-worktree failure');
}
NODE

invalid_output="$(bash "$PREPARE" --json --unknown 2>/dev/null || true)"
assert_contains "$invalid_output" '"status":"failed"'
bash "$HARNESS" run-contract \
  --name prepare-invalid-args \
  --expected-exit 129 \
  --expected-command prepare \
  -- bash "$PREPARE" --json --unknown

bash "$HARNESS" run-kill9-rerun \
  --name prepare-kill-rerun \
  --state-file "$tmp/prepare-kill.state" \
  -- bash -c 'cd "$1" && ANVIL_RELEASE_TEST_MODE=kill-rerun ANVIL_RELEASE_TEST_TIMEOUT_SECONDS=5 ANVIL_RELEASE_PREPARE_KILL_STATE="$3" bash "$2" --json --dry-run --version v0.7.0-beta --release-type beta --strategy direct' _ "$repo" "$PREPARE" "$tmp/prepare-kill.state"

rc=0
(cd "$repo" && ANVIL_RELEASE_PREPARE_KILL_STATE="$tmp/unguarded.state" bash "$PREPARE" --json --dry-run --version v0.7.0-beta --release-type beta --strategy direct >"$tmp/unguarded.json") || rc=$?
if [[ "$rc" != "129" ]]; then
  echo "expected unguarded kill-state hook to exit 129, got $rc" >&2
  exit 1
fi
node - "$tmp/unguarded.json" <<'NODE'
const fs = require('node:fs');
const doc = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
if (!doc.failures.some((failure) => failure.code === 'invalid-input')) {
  throw new Error('expected unguarded kill-state hook to report invalid-input');
}
NODE

help_output="$(bash "$PREPARE" --help)"
assert_contains "$help_output" 'Usage: prepare.sh'

repo_non_dry="$tmp/prepare-real-repo"
init_repo "$repo_non_dry"
fake_issues="$tmp/prepare-fake-issues.json"
bash "$HARNESS" run-contract \
  --name prepare-non-dry-run-fake-issue \
  --expected-exit 0 \
  --expected-command prepare \
  -- bash -c 'cd "$1" && ANVIL_RELEASE_TEST_MODE=prepare-fake-gh ANVIL_RELEASE_PREPARE_FAKE_ISSUES_FILE="$3" bash "$2" --json --version v0.7.0-beta --release-type beta --strategy direct --request-readiness --repo eddacraft/anvil-001' _ "$repo_non_dry" "$PREPARE" "$fake_issues"

node - "$repo_non_dry" "$fake_issues" <<'NODE'
const fs = require('node:fs');
const [repo, issuePath] = process.argv.slice(2);
const pkg = JSON.parse(fs.readFileSync(`${repo}/package.json`, 'utf8'));
if (pkg.version !== '0.7.0-beta') throw new Error(`wrong package version ${pkg.version}`);
for (const path of ['CHANGELOG.md', 'docs/public/anvil/releases/changelog.md']) {
  const text = fs.readFileSync(`${repo}/${path}`, 'utf8');
  if (!text.includes('## v0.7.0-beta')) throw new Error(`${path} missing release section`);
}
const state = JSON.parse(fs.readFileSync(issuePath, 'utf8'));
if (state.issues.length !== 1) throw new Error(`expected one issue, got ${state.issues.length}`);
if (state.issues[0].comments.length !== 1) throw new Error('expected one metadata comment');
NODE
if [[ -n "$(git -C "$repo_non_dry" status --porcelain)" ]]; then
  echo "expected non-dry-run prepare to leave clean worktree" >&2
  exit 1
fi

bash "$HARNESS" run-contract \
  --name prepare-resume-existing-fake-issue \
  --expected-exit 0 \
  --expected-command prepare \
  -- bash -c 'cd "$1" && ANVIL_RELEASE_TEST_MODE=prepare-fake-gh ANVIL_RELEASE_PREPARE_FAKE_ISSUES_FILE="$3" bash "$2" --json --version v0.7.0-beta --release-type beta --strategy direct --tracking-issue 1234 --repo eddacraft/anvil-001' _ "$repo_non_dry" "$PREPARE" "$fake_issues"
node - "$fake_issues" <<'NODE'
const fs = require('node:fs');
const state = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
if (state.issues.length !== 1) throw new Error('resume should not create another issue');
if (state.issues[0].comments.length !== 2) throw new Error('resume should append metadata comment');
NODE

rc=0
(cd "$repo_non_dry" && ANVIL_RELEASE_PREPARE_FAKE_ISSUES_FILE="$tmp/unguarded-issues.json" bash "$PREPARE" --json --version v0.7.0-beta --release-type beta --strategy direct >"$tmp/unguarded-gh.json") || rc=$?
if [[ "$rc" != "129" ]]; then
  echo "expected unguarded fake gh hook to exit 129, got $rc" >&2
  exit 1
fi
node - "$tmp/unguarded-gh.json" <<'NODE'
const fs = require('node:fs');
const doc = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
if (!doc.failures.some((failure) => failure.code === 'invalid-input')) {
  throw new Error('expected unguarded fake gh hook to report invalid-input');
}
NODE

echo "prepare.test.sh: ok"
