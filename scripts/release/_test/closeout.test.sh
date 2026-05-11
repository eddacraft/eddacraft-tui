#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
HARNESS="$ROOT/scripts/release/_test/harness.sh"
CLOSEOUT="$ROOT/scripts/release/closeout.sh"

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
}

repo="$tmp/closeout-repo"
mkdir -p "$repo"
init_repo "$repo"
source_sha="$(git -C "$repo" rev-parse HEAD)"
record_url="https://github.com/eddacraft/anvil-001/releases/download/v0.7.0-beta/release-record.json"

bash "$HARNESS" run-contract \
  --name closeout-dry-run \
  --expected-exit 0 \
  --expected-command closeout \
  -- bash -c 'cd "$1" && bash "$2" --json --dry-run --version v0.7.0-beta --tag v0.7.0-beta --source-sha "$3" --verification-record "$4" --verification-passed --tracking-issue 1234 --cleanup-branch release/v0.7.0-beta --repo eddacraft/anvil-001' _ "$repo" "$CLOSEOUT" "$source_sha" "$record_url"

(cd "$repo" && bash "$CLOSEOUT" --json --dry-run --version v0.7.0-beta --tag v0.7.0-beta --source-sha "$source_sha" --verification-record "$record_url" --verification-passed --tracking-issue 1234 --cleanup-branch release/v0.7.0-beta) >"$tmp/closeout.json"
node - "$tmp/closeout.json" "$source_sha" "$record_url" <<'NODE'
const fs = require('node:fs');
const [path, expectedSha, expectedRecord] = process.argv.slice(2);
const doc = JSON.parse(fs.readFileSync(path, 'utf8'));
if (doc.status !== 'success') throw new Error(`expected success, got ${doc.status}`);
if (doc.releaseRecord.lifecycleState !== 'published') throw new Error('closeout should cite a published release record');
if (doc.releaseRecord.recordUrl !== expectedRecord) throw new Error('wrong release record URL');
if (doc.data.finalSummary.sourceSha !== expectedSha) throw new Error('wrong source SHA');
if (!doc.data.cleanupActions.some((action) => action.action === 'delete-release-branch')) {
  throw new Error('expected release branch cleanup action');
}
NODE

rc=0
(cd "$repo" && bash "$CLOSEOUT" --json --dry-run --version v0.7.0-beta --tag v0.7.0-beta --source-sha "$source_sha" --verification-record "$record_url" >"$tmp/no-verification.json") || rc=$?
if [[ "$rc" != "1" ]]; then
  echo "expected missing verification to exit 1, got $rc" >&2
  exit 1
fi
node - "$tmp/no-verification.json" <<'NODE'
const fs = require('node:fs');
const doc = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
if (doc.status !== 'blocked') throw new Error(`expected blocked, got ${doc.status}`);
if (!doc.failures.some((failure) => failure.code === 'operator-required')) {
  throw new Error('expected operator-required failure');
}
NODE

bash "$HARNESS" run-contract \
  --name closeout-needs-operator \
  --expected-exit 1 \
  --expected-command closeout \
  -- bash -c 'cd "$1" && bash "$2" --json --version v0.7.0-beta --tag v0.7.0-beta --source-sha "$3" --verification-record "$4" --verification-passed' _ "$repo" "$CLOSEOUT" "$source_sha" "$record_url"

bash "$HARNESS" run-contract \
  --name closeout-fake-issue \
  --expected-exit 0 \
  --expected-command closeout \
  -- bash -c 'cd "$1" && ANVIL_RELEASE_TEST_MODE=closeout-fake-issue ANVIL_RELEASE_CLOSEOUT_FAKE_ISSUE_FILE="$5" bash "$2" --json --version v0.7.0-beta --tag v0.7.0-beta --source-sha "$3" --verification-record "$4" --verification-passed --tracking-issue 1234 --close-issue' _ "$repo" "$CLOSEOUT" "$source_sha" "$record_url" "$tmp/fake-issue.json"
node - "$tmp/fake-issue.json" "$source_sha" <<'NODE'
const fs = require('node:fs');
const [path, expectedSha] = process.argv.slice(2);
const issue = JSON.parse(fs.readFileSync(path, 'utf8'));
if (issue.closed !== true) throw new Error('expected fake issue to be closed');
if (issue.sourceSha !== expectedSha) throw new Error('fake issue recorded wrong source SHA');
NODE

rc=0
(cd "$repo" && ANVIL_RELEASE_CLOSEOUT_FAKE_ISSUE_FILE="$tmp/unguarded-fake-issue.json" bash "$CLOSEOUT" --json --version v0.7.0-beta --tag v0.7.0-beta --source-sha "$source_sha" --verification-record "$record_url" --verification-passed --tracking-issue 1234 --close-issue >"$tmp/unguarded-fake.json") || rc=$?
if [[ "$rc" != "129" ]]; then
  echo "expected unguarded fake issue hook to exit 129, got $rc" >&2
  exit 1
fi
node - "$tmp/unguarded-fake.json" <<'NODE'
const fs = require('node:fs');
const doc = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
if (!doc.failures.some((failure) => failure.code === 'invalid-input')) {
  throw new Error('expected unguarded fake issue hook to report invalid-input');
}
NODE

invalid_output="$(bash "$CLOSEOUT" --json --unknown 2>/dev/null || true)"
assert_contains "$invalid_output" '"status":"failed"'
bash "$HARNESS" run-contract \
  --name closeout-invalid-args \
  --expected-exit 129 \
  --expected-command closeout \
  -- bash "$CLOSEOUT" --json --unknown

help_output="$(bash "$CLOSEOUT" --help)"
assert_contains "$help_output" 'Usage: closeout.sh'

echo "closeout.test.sh: ok"
