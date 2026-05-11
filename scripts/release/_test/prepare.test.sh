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
  -- bash -c 'cd "$1" && ANVIL_RELEASE_PREPARE_KILL_STATE="$3" bash "$2" --json --dry-run --version v0.7.0-beta --release-type beta --strategy direct' _ "$repo" "$PREPARE" "$tmp/prepare-kill.state"

help_output="$(bash "$PREPARE" --help)"
assert_contains "$help_output" 'Usage: prepare.sh'

echo "prepare.test.sh: ok"
