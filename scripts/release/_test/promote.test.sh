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

bash "$HARNESS" run-contract \
  --name promote-needs-operator \
  --expected-exit 1 \
  --expected-command promote \
  -- bash -c 'cd "$1" && bash "$2" --json --version v0.7.0-beta --strategy direct --base main --head dev' _ "$repo" "$PROMOTE"
(cd "$repo" && bash "$PROMOTE" --json --version v0.7.0-beta --strategy direct --base main --head dev >"$tmp/non-dry-run.json") || rc=$?
rc="${rc:-0}"
if [[ "$rc" != "1" ]]; then
  echo "expected non-dry-run promotion to exit 1, got $rc" >&2
  exit 1
fi
node - "$tmp/non-dry-run.json" <<'NODE'
const fs = require('node:fs');
const doc = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
if (doc.status !== 'needs-operator') throw new Error(`expected needs-operator, got ${doc.status}`);
if (!doc.failures.some((failure) => failure.code === 'operator-required')) {
  throw new Error('expected operator-required failure');
}
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

echo "promote.test.sh: ok"
