#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
HARNESS="$ROOT/scripts/release/_test/harness.sh"
PROMOTE="$ROOT/scripts/release/promote.sh"

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
  git -C "$repo" init -q
  git -C "$repo" config user.email relorch@example.invalid
  git -C "$repo" config user.name "RELORCH Test"
  printf '%s\n' '# Fixture' >"$repo/README.md"
  git -C "$repo" add README.md
  git -C "$repo" commit -q -m "chore: initial fixture"
  git -C "$repo" branch -M main
  git -C "$repo" checkout -q -b dev
  printf '%s\n' 'release work' >"$repo/release.txt"
  git -C "$repo" add release.txt
  git -C "$repo" commit -q -m "feat: release candidate RELORCH-006"
}

repo="$tmp/promote-repo"
mkdir -p "$repo"
init_repo "$repo"

bash "$HARNESS" run-contract \
  --name promote-dry-run \
  --expected-exit 0 \
  --expected-command promote \
  -- bash -c 'cd "$1" && bash "$2" --json --dry-run --version v0.7.0-beta --strategy direct --base main --head dev --repo eddacraft/anvil-001' _ "$repo" "$PROMOTE"

(cd "$repo" && bash "$PROMOTE" --json --dry-run --version v0.7.0-beta --strategy direct --base main --head dev) >"$tmp/promote.json"
node - "$tmp/promote.json" <<'NODE'
const fs = require('node:fs');
const doc = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
if (doc.status !== 'success') throw new Error(`expected success, got ${doc.status}`);
if (doc.mode !== 'compatibility') throw new Error(`expected compatibility mode, got ${doc.mode}`);
if (doc.data.pullRequest.base !== 'main' || doc.data.pullRequest.head !== 'dev') {
  throw new Error('expected pull request base/head from inputs');
}
if (doc.data.mergeState !== 'not-created') throw new Error(`unexpected mergeState ${doc.data.mergeState}`);
if (doc.data.operatorActionRequired !== true) throw new Error('dry-run promotion should require operator action');
NODE

target_sha="$(git -C "$repo" rev-parse HEAD)"
(cd "$repo" && bash "$PROMOTE" --json --source-sha "$target_sha" --version v0.7.0-beta) >"$tmp/target.json"
bash "$HARNESS" run-contract \
  --name promote-target-noop \
  --expected-exit 0 \
  --expected-command promote \
  -- bash -c 'cd "$1" && bash "$2" --json --source-sha "$3" --version v0.7.0-beta' _ "$repo" "$PROMOTE" "$target_sha"
node - "$tmp/target.json" "$target_sha" <<'NODE'
const fs = require('node:fs');
const [path, expectedSha] = process.argv.slice(2);
const doc = JSON.parse(fs.readFileSync(path, 'utf8'));
if (doc.status !== 'noop') throw new Error(`expected noop, got ${doc.status}`);
if (doc.mode !== 'target') throw new Error(`expected target mode, got ${doc.mode}`);
if (doc.data.mergeState !== 'not-required') throw new Error(`unexpected target mergeState ${doc.data.mergeState}`);
if (doc.data.mergedSha !== expectedSha) throw new Error(`unexpected mergedSha ${doc.data.mergedSha}`);
NODE

target_readiness_state="$tmp/promote-target-readiness.json"
bash "$HARNESS" run-contract \
  --name promote-target-readiness-fake-gh \
  --expected-exit 0 \
  --expected-command promote \
  -- bash -c 'cd "$1" && ANVIL_RELEASE_TEST_MODE=promote-fake-gh ANVIL_RELEASE_PROMOTE_FAKE_GH_FILE="$4" bash "$2" --json --source-sha "$3" --version v0.7.0-beta --strategy direct --request-readiness --channel beta --base-boundary v0.6.1-beta' _ "$repo" "$PROMOTE" "$target_sha" "$target_readiness_state"
node - "$target_readiness_state" <<'NODE'
const fs = require('node:fs');
const state = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
if (!state.calls.some((call) => call.command === 'workflow-run' && call.sourceSha)) throw new Error('expected target readiness workflow dispatch');
NODE

invalid_output="$(bash "$PROMOTE" --json --unknown 2>/dev/null || true)"
assert_contains "$invalid_output" '"status":"failed"'
bash "$HARNESS" run-contract \
  --name promote-invalid-args \
  --expected-exit 129 \
  --expected-command promote \
  -- bash "$PROMOTE" --json --unknown

help_output="$(bash "$PROMOTE" --help)"
assert_contains "$help_output" 'Usage: promote.sh'

fake_state="$tmp/promote-fake-gh.json"
bash "$HARNESS" run-contract \
  --name promote-create-pr-fake-gh \
  --expected-exit 0 \
  --expected-command promote \
  -- bash -c 'cd "$1" && ANVIL_RELEASE_TEST_MODE=promote-fake-gh ANVIL_RELEASE_PROMOTE_FAKE_GH_FILE="$4" bash "$2" --json --version v0.7.0-beta --strategy direct --base main --head dev --tracking-issue 1234' _ "$repo" "$PROMOTE" unused "$fake_state"
node - "$fake_state" <<'NODE'
const fs = require('node:fs');
const state = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
if (state.prs.length !== 1) throw new Error('expected one fake PR');
if (!state.calls.some((call) => call.command === 'pr-create')) throw new Error('expected pr-create call');
NODE

bash "$HARNESS" run-contract \
  --name promote-resume-open-pr-fake-gh \
  --expected-exit 0 \
  --expected-command promote \
  -- bash -c 'cd "$1" && ANVIL_RELEASE_TEST_MODE=promote-fake-gh ANVIL_RELEASE_PROMOTE_FAKE_GH_FILE="$4" bash "$2" --json --version v0.7.0-beta --strategy direct --base main --head dev --tracking-issue 1234' _ "$repo" "$PROMOTE" unused "$fake_state"
node - "$fake_state" <<'NODE'
const fs = require('node:fs');
const state = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
if (state.prs.length !== 1) throw new Error('resume should not create another fake PR');
if (state.calls.filter((call) => call.command === 'pr-create').length !== 1) throw new Error('resume should not call pr-create again');
NODE

conflict_state="$tmp/promote-conflict.json"
node - "$conflict_state" <<'NODE'
const fs = require('node:fs');
fs.writeFileSync(process.argv[2], JSON.stringify({ nextPr: 1401, prs: [{ number: 1400, url: 'https://github.com/eddacraft/anvil-001/pull/1400', title: 'Release v0.7.0-beta', base: 'main', head: 'dev', state: 'OPEN', mergeStateStatus: 'DIRTY', reviewDecision: 'REVIEW_REQUIRED', mergeCommit: null }], runs: [], calls: [] }, null, 2));
NODE
bash "$HARNESS" run-contract \
  --name promote-conflict-fake-gh \
  --expected-exit 1 \
  --expected-command promote \
  -- bash -c 'cd "$1" && ANVIL_RELEASE_TEST_MODE=promote-fake-gh ANVIL_RELEASE_PROMOTE_FAKE_GH_FILE="$4" bash "$2" --json --version v0.7.0-beta --strategy direct --base main --head dev --tracking-issue 1234' _ "$repo" "$PROMOTE" unused "$conflict_state"

review_state="$tmp/promote-review.json"
node - "$review_state" <<'NODE'
const fs = require('node:fs');
fs.writeFileSync(process.argv[2], JSON.stringify({ nextPr: 1401, prs: [{ number: 1400, url: 'https://github.com/eddacraft/anvil-001/pull/1400', title: 'Release v0.7.0-beta', base: 'main', head: 'dev', state: 'OPEN', mergeStateStatus: 'CLEAN', reviewDecision: 'CHANGES_REQUESTED', mergeCommit: null }], runs: [], calls: [] }, null, 2));
NODE
bash "$HARNESS" run-contract \
  --name promote-review-block-fake-gh \
  --expected-exit 1 \
  --expected-command promote \
  -- bash -c 'cd "$1" && ANVIL_RELEASE_TEST_MODE=promote-fake-gh ANVIL_RELEASE_PROMOTE_FAKE_GH_FILE="$4" bash "$2" --json --version v0.7.0-beta --strategy direct --base main --head dev --tracking-issue 1234' _ "$repo" "$PROMOTE" unused "$review_state"

merged_state="$tmp/promote-merged.json"
merge_sha="$(git -C "$repo" rev-parse dev)"
node - "$merged_state" "$merge_sha" <<'NODE'
const fs = require('node:fs');
const mergeSha = process.argv[3];
fs.writeFileSync(process.argv[2], JSON.stringify({ nextPr: 1401, prs: [{ number: 1400, url: 'https://github.com/eddacraft/anvil-001/pull/1400', title: 'Release v0.7.0-beta', base: 'main', head: 'dev', state: 'MERGED', mergeStateStatus: 'CLEAN', reviewDecision: 'APPROVED', mergeCommit: { oid: mergeSha } }], runs: [], calls: [] }, null, 2));
NODE
bash "$HARNESS" run-contract \
  --name promote-readiness-fake-gh \
  --expected-exit 0 \
  --expected-command promote \
  -- bash -c 'cd "$1" && ANVIL_RELEASE_TEST_MODE=promote-fake-gh ANVIL_RELEASE_PROMOTE_FAKE_GH_FILE="$4" bash "$2" --json --version v0.7.0-beta --strategy direct --base main --head dev --tracking-issue 1234 --request-readiness --channel beta --base-boundary v0.6.1-beta' _ "$repo" "$PROMOTE" unused "$merged_state"
node - "$merged_state" <<'NODE'
const fs = require('node:fs');
const state = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
if (!state.calls.some((call) => call.command === 'workflow-run' && call.mode === 'readiness')) throw new Error('expected readiness workflow dispatch');
NODE

bash "$HARNESS" run-contract \
  --name promote-readiness-resume-fake-gh \
  --expected-exit 0 \
  --expected-command promote \
  -- bash -c 'cd "$1" && ANVIL_RELEASE_TEST_MODE=promote-fake-gh ANVIL_RELEASE_PROMOTE_FAKE_GH_FILE="$4" bash "$2" --json --version v0.7.0-beta --strategy direct --base main --head dev --tracking-issue 1234 --request-readiness --channel beta --base-boundary v0.6.1-beta' _ "$repo" "$PROMOTE" unused "$merged_state"
node - "$merged_state" <<'NODE'
const fs = require('node:fs');
const state = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
if (state.calls.filter((call) => call.command === 'workflow-run').length !== 1) throw new Error('resume should not redispatch readiness');
NODE

bash "$HARNESS" run-contract \
  --name promote-readiness-missing-boundary \
  --expected-exit 129 \
  --expected-command promote \
  -- bash -c 'cd "$1" && bash "$2" --json --version v0.7.0-beta --strategy direct --base main --head dev --request-readiness --channel beta' _ "$repo" "$PROMOTE"

rc=0
(cd "$repo" && ANVIL_RELEASE_PROMOTE_FAKE_GH_FILE="$tmp/unguarded-promote.json" bash "$PROMOTE" --json --version v0.7.0-beta --strategy direct --base main --head dev >"$tmp/unguarded-promote.out") || rc=$?
if [[ "$rc" != "129" ]]; then
  echo "expected unguarded promote fake gh hook to exit 129, got $rc" >&2
  exit 1
fi
node - "$tmp/unguarded-promote.out" <<'NODE'
const fs = require('node:fs');
const doc = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
if (!doc.failures.some((failure) => failure.code === 'invalid-input')) {
  throw new Error('expected unguarded fake gh hook to report invalid-input');
}
NODE

echo "promote.test.sh: ok"
