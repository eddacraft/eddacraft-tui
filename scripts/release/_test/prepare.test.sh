#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
HARNESS="$ROOT/scripts/release/_test/harness.sh"
PREPARE="$ROOT/scripts/release/prepare.sh"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

# CIB-196: a realistic changelog pair. The old fixture was a bare
# `# Changelog`, which is precisely the shape that let the metadata-stub
# behaviour pass its own tests — there was no draft for promotion to get wrong.
write_changelogs() {
  local repo="$1"
  cat >"$repo/CHANGELOG.md" <<'CHANGELOG'
# Changelog

All notable changes to this product are documented here.

## [Unreleased]

> **Draft.** This section accumulates customer-relevant changes landed on
> `main`; the version and date are set at the next release.

### Added

- A customer-facing capability that must survive promotion.

### Fixed

- A customer-facing fix that must survive promotion.

## [0.6.1-beta] — 2026-01-02

### Added

- An older entry that must keep its place and spacing.
CHANGELOG
  cat >"$repo/docs/public/anvil/releases/changelog.md" <<'PUBLIC'
# Current release notes

This page summarises the current user-visible beta release.

## 0.6.1-beta — 2 January 2026

### Added

- An older public entry.
PUBLIC
}

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
  write_changelogs "$repo"
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

invalid_version_output="$(bash "$PREPARE" --json --version v0.7.0-beta_1 --release-type beta --strategy direct 2>/dev/null || true)"
assert_contains "$invalid_version_output" '"code":"invalid-input"'

for malformed_version in v1.2.3-.1 v1.2.3-beta..1; do
  invalid_version_output="$(bash "$PREPARE" --json --version "$malformed_version" --release-type beta --strategy direct 2>/dev/null || true)"
  assert_contains "$invalid_version_output" '"code":"invalid-input"'
done

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
// CIB-196: the promoted section must carry the whole Unreleased draft, sit
// where the newest release belongs, and leave no metadata stub behind.
for (const path of ['CHANGELOG.md', 'docs/public/anvil/releases/changelog.md']) {
  const text = fs.readFileSync(`${repo}/${path}`, 'utf8');
  const heading = path === 'CHANGELOG.md' ? '## [0.7.0-beta] — ' : '## 0.7.0-beta — ';
  if (!text.includes(heading)) throw new Error(`${path} missing release section: ${text}`);
  if (!text.includes(`\n\n${heading}`)) throw new Error(`${path} missing blank line before release heading`);
  if (text.includes('Release preparation metadata generated'))
    throw new Error(`${path} still carries the metadata stub`);
  for (const entry of ['A customer-facing capability that must survive promotion', 'A customer-facing fix that must survive promotion']) {
    if (!text.includes(entry)) throw new Error(`${path} dropped a draft entry: ${entry}`);
  }
  // The new section leads; the previous release keeps its place below it.
  if (text.indexOf(heading) > text.indexOf('0.6.1-beta'))
    throw new Error(`${path} put the new release below the older one`);
  if (!/\n## .*0\.6\.1-beta.*\n\n### Added/.test(text))
    throw new Error(`${path} lost the blank line under the older heading`);
  if (text.endsWith('\n\n')) throw new Error(`${path} has trailing blank line; oxfmt --check will fail`);
  if (!text.endsWith('\n')) throw new Error(`${path} missing final newline`);
}
{
  // Unreleased survives, emptied back to its standing note, ready for the next
  // cycle — and it must not still hold the entries we just promoted.
  const main = fs.readFileSync(`${repo}/CHANGELOG.md`, 'utf8');
  const unreleased = main.slice(main.indexOf('## [Unreleased]'), main.indexOf('## [0.7.0-beta]'));
  if (!unreleased.includes('> **Draft.**')) throw new Error('Unreleased lost its standing draft note');
  if (unreleased.includes('must survive promotion')) throw new Error('Unreleased still holds promoted entries');
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

repo_cargo="$tmp/prepare-cargo-repo"
mkdir -p "$repo_cargo/crates/anvil-cli" "$repo_cargo/packages/anvil/core" "$repo_cargo/packages/anvil/runtime" "$repo_cargo/docs/public/anvil/releases"
git -C "$repo_cargo" init -q
git -C "$repo_cargo" config user.email relorch@example.invalid
git -C "$repo_cargo" config user.name "RELORCH Test"
printf '%s\n' '{"version":"0.6.1-beta","name":"@test/root"}' >"$repo_cargo/package.json"
printf '%s\n' '{"version":"0.6.1-beta","name":"@test/core"}' >"$repo_cargo/packages/anvil/core/package.json"
printf '%s\n' '{"version":"0.6.1-beta","name":"@test/runtime"}' >"$repo_cargo/packages/anvil/runtime/package.json"
printf '%s\n' '{"version":"0.5.0","name":"@test/unaligned"}' >"$repo_cargo/packages/anvil/runtime/sub.package.json"
cat >"$repo_cargo/Cargo.toml" <<'CARGOTOML'
[workspace]
members = ["crates/anvil-cli"]

[workspace.package]
version = "0.6.1-beta"
edition = "2021"
CARGOTOML
cat >"$repo_cargo/crates/anvil-cli/Cargo.toml" <<'CRATETOML'
[package]
name = "anvil-cli"
version.workspace = true
edition.workspace = true
CRATETOML
write_changelogs "$repo_cargo"
git -C "$repo_cargo" add .
git -C "$repo_cargo" commit -q -m "chore: cargo fixture"

cargo_fake_issues="$tmp/prepare-cargo-issues.json"
(cd "$repo_cargo" && ANVIL_RELEASE_TEST_MODE=prepare-fake-gh ANVIL_RELEASE_PREPARE_FAKE_ISSUES_FILE="$cargo_fake_issues" bash "$PREPARE" --json --version v0.7.0-beta --release-type beta --strategy direct --repo eddacraft/anvil-001 >"$tmp/prepare-cargo.json")
node - "$repo_cargo" "$tmp/prepare-cargo.json" <<'NODE'
const fs = require('node:fs');
const [repo, jsonPath] = process.argv.slice(2);
const doc = JSON.parse(fs.readFileSync(jsonPath, 'utf8'));
if (doc.status !== 'success') throw new Error(`expected success, got ${doc.status}`);
const must = ['package.json', 'CHANGELOG.md', 'docs/public/anvil/releases/changelog.md', 'Cargo.toml', 'packages/anvil/core/package.json', 'packages/anvil/runtime/package.json'];
for (const path of must) {
  if (!doc.data.changedFiles.includes(path)) throw new Error(`expected ${path} in changedFiles, got ${JSON.stringify(doc.data.changedFiles)}`);
}
if (doc.data.changedFiles.includes('packages/anvil/runtime/sub.package.json')) throw new Error('unaligned package.json should not be bumped');
const cargo = fs.readFileSync(`${repo}/Cargo.toml`, 'utf8');
if (!/\[workspace\.package\][\s\S]*?version = "0\.7\.0-beta"/.test(cargo)) throw new Error(`Cargo.toml workspace version not bumped: ${cargo}`);
for (const pkg of ['packages/anvil/core', 'packages/anvil/runtime']) {
  const v = JSON.parse(fs.readFileSync(`${repo}/${pkg}/package.json`, 'utf8')).version;
  if (v !== '0.7.0-beta') throw new Error(`${pkg}/package.json version not bumped (got ${v})`);
}
const unaligned = JSON.parse(fs.readFileSync(`${repo}/packages/anvil/runtime/sub.package.json`, 'utf8')).version;
if (unaligned !== '0.5.0') throw new Error(`unaligned sub.package.json should be untouched (got ${unaligned})`);
NODE

# CIB-196: an empty Unreleased draft must stop the cut rather than quietly
# emit a stub section. Shipping "Release preparation metadata generated." as a
# release's customer-facing notes is the failure this replaces.
repo_empty="$tmp/prepare-repo-empty"
init_repo "$repo_empty"
cat >"$repo_empty/CHANGELOG.md" <<'EMPTYLOG'
# Changelog

## [Unreleased]

> **Draft.** Nothing has landed yet.

## [0.6.1-beta] — 2026-01-02

### Added

- An older entry.
EMPTYLOG
git -C "$repo_empty" add . && git -C "$repo_empty" commit -q -m "chore: empty draft"

set +e
(cd "$repo_empty" && ANVIL_RELEASE_TEST_MODE=prepare-fake-gh ANVIL_RELEASE_PREPARE_FAKE_ISSUES_FILE="$tmp/empty-issues.json" bash "$PREPARE" --json --version v0.7.0-beta --release-type beta --strategy direct --repo eddacraft/anvil-001) >"$tmp/prepare-empty.json" 2>"$tmp/prepare-empty.err"
empty_rc=$?
set -e
if [[ "$empty_rc" == "0" ]]; then
  echo "expected prepare to fail on an empty Unreleased draft" >&2
  exit 1
fi
assert_contains "$(cat "$tmp/prepare-empty.err")" 'no entries to promote'
if grep -F 'Release preparation metadata generated' "$repo_empty/CHANGELOG.md" >/dev/null 2>&1; then
  echo "empty draft must not produce a metadata stub" >&2
  exit 1
fi

# The escape hatch stays explicit, for a genuinely internal-only patch.
(cd "$repo_empty" && ANVIL_RELEASE_TEST_MODE=prepare-fake-gh ANVIL_RELEASE_PREPARE_FAKE_ISSUES_FILE="$tmp/empty-issues2.json" ANVIL_RELEASE_ALLOW_EMPTY_CHANGELOG=1 bash "$PREPARE" --json --version v0.7.0-beta --release-type beta --strategy direct --repo eddacraft/anvil-001) >/dev/null
assert_contains "$(cat "$repo_empty/CHANGELOG.md")" '## [0.7.0-beta] — '

echo "prepare.test.sh: ok"
