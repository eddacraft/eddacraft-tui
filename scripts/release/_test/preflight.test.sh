#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
HARNESS="$ROOT/scripts/release/_test/harness.sh"
PREFLIGHT="$ROOT/scripts/release/preflight.sh"

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

bash "$PREFLIGHT" --help >"$tmp/help.out"
assert_contains "$(<"$tmp/help.out")" "Usage:"
assert_contains "$(<"$tmp/help.out")" "--json"

ANVIL_RELEASE_PREFLIGHT_FIXTURE=pass \
  bash "$HARNESS" run-contract \
    --name preflight-pass \
    --expected-exit 0 \
    --expected-command preflight \
    -- bash "$PREFLIGHT" --json --base main --head dev --repo eddacraft/anvil-001

# The pass fixture must expose the cargo workspace version-match gate so a
# release engineer who forgets the Cargo.toml bump is caught (issue #1871).
ANVIL_RELEASE_PREFLIGHT_FIXTURE=pass \
  bash "$PREFLIGHT" --json --base main --head dev >"$tmp/pass.json"
node - "$tmp/pass.json" <<'NODE'
const fs = require('node:fs');
const doc = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
const ids = doc.data.gates.map((gate) => gate.id);
if (!ids.includes('cargo-version')) {
  throw new Error(`expected cargo-version gate, got: ${ids.join(',')}`);
}
const gate = doc.data.gates.find((g) => g.id === 'cargo-version');
if (gate.status !== 'pass') throw new Error(`expected cargo-version to pass, got ${gate.status}`);
NODE

# --version is accepted and threaded into inputs.version.
ANVIL_RELEASE_PREFLIGHT_FIXTURE=pass \
  bash "$PREFLIGHT" --json --version v9.9.9 >"$tmp/version-input.json"
node - "$tmp/version-input.json" <<'NODE'
const fs = require('node:fs');
const doc = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
if (doc.inputs.version !== 'v9.9.9') {
  throw new Error(`expected inputs.version v9.9.9, got ${doc.inputs.version}`);
}
NODE

# An invalid --version value is rejected as invalid-input.
version_invalid_json="$(bash "$PREFLIGHT" --json --version not-a-version 2>/dev/null || true)"
assert_contains "$version_invalid_json" '"code":"invalid-input"'

ANVIL_RELEASE_PREFLIGHT_FIXTURE=pass \
  bash "$PREFLIGHT" --base main --head dev >"$tmp/human.out"
assert_contains "$(<"$tmp/human.out")" "Preflight summary"
assert_contains "$(<"$tmp/human.out")" "All preflight gates passed"

ANVIL_RELEASE_PREFLIGHT_FIXTURE=fail \
ANVIL_RELEASE_PREFLIGHT_FIXTURE_FAILURES="cargo-test,pnpm-lint" \
  bash "$HARNESS" run-contract \
    --name preflight-failures \
    --expected-exit 2 \
    --expected-command preflight \
    -- bash "$PREFLIGHT" --json --base main --head dev

rc=0
ANVIL_RELEASE_PREFLIGHT_FIXTURE=fail \
ANVIL_RELEASE_PREFLIGHT_FIXTURE_FAILURES="cargo-test,pnpm-lint" \
  bash "$PREFLIGHT" --json >"$tmp/failures.json" 2>"$tmp/failures.err" || rc=$?
rc="${rc:-0}"
if [[ "$rc" != "2" ]]; then
  echo "expected fixture failure exit 2, got $rc" >&2
  exit 1
fi
if [[ -s "$tmp/failures.err" ]]; then
  echo "expected --json stderr to be empty for normal gate failures" >&2
  exit 1
fi
node - "$tmp/failures.json" <<'NODE'
const fs = require('node:fs');
const doc = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
const failedIds = doc.data.gates.filter((gate) => gate.status === 'fail').map((gate) => gate.id);
if (doc.status !== 'failed') throw new Error(`expected failed status, got ${doc.status}`);
if (doc.data.failedGateCount !== 2) throw new Error(`expected 2 failed gates, got ${doc.data.failedGateCount}`);
if (!doc.data.gates.every((gate) => typeof gate.durationMs === 'number')) throw new Error('expected durationMs on each gate');
for (const tool of ['git', 'gh', 'cargo', 'node', 'opa', 'pnpm']) {
  if (!(tool in doc.data.toolVersions)) throw new Error(`missing toolVersions.${tool}`);
  if (doc.data.toolVersions[tool] !== null && typeof doc.data.toolVersions[tool] !== 'string') {
    throw new Error(`expected scalar toolVersions.${tool}`);
  }
}
if (doc.next.command !== 'preflight') throw new Error(`expected failed preflight to point back to preflight, got ${doc.next.command}`);
if (!failedIds.includes('cargo-test') || !failedIds.includes('pnpm-lint')) {
  throw new Error(`missing expected failed gate ids: ${failedIds.join(',')}`);
}
NODE

invalid_json="$(bash "$PREFLIGHT" --json --unknown 2>/dev/null || true)"
assert_contains "$invalid_json" '"code":"invalid-input"'
invalid_json_reordered="$(bash "$PREFLIGHT" --unknown --json 2>/dev/null || true)"
assert_contains "$invalid_json_reordered" '"code":"invalid-input"'
bash "$HARNESS" run-contract \
  --name preflight-invalid-json \
  --expected-exit 129 \
  --expected-command preflight \
  -- bash "$PREFLIGHT" --json --unknown

ANVIL_RELEASE_PREFLIGHT_FIXTURE=missing-tool \
  bash "$HARNESS" run-contract \
    --name preflight-missing-tool \
    --expected-exit 127 \
    --expected-command preflight \
    -- bash "$PREFLIGHT" --json

rc=0
ANVIL_RELEASE_PREFLIGHT_FIXTURE=missing-tool \
  bash "$PREFLIGHT" --json >"$tmp/missing-tool.json" || rc=$?
rc="${rc:-0}"
if [[ "$rc" != "127" ]]; then
  echo "expected missing tool exit 127, got $rc" >&2
  exit 1
fi
node - "$tmp/missing-tool.json" <<'NODE'
const fs = require('node:fs');
const doc = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
if (!doc.failures.some((failure) => failure.code === 'infra-failed')) {
  throw new Error('expected missing tool failures to use infra-failed');
}
NODE

rc=0
ANVIL_RELEASE_PREFLIGHT_FIXTURE=version-mismatch \
  bash "$PREFLIGHT" --json >"$tmp/version.json" || rc=$?
rc="${rc:-0}"
# version-mismatch fails hakari-version, deny-version, and cargo-version (3 gates).
if [[ "$rc" != "3" ]]; then
  echo "expected version mismatch exit 3, got $rc" >&2
  exit 1
fi
node - "$tmp/version.json" <<'NODE'
const fs = require('node:fs');
const doc = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
const hakari = doc.data.toolVersions.cargoHakari;
const deny = doc.data.toolVersions.cargoDeny;
if (hakari.status !== 'mismatch' || deny.status !== 'mismatch') {
  throw new Error('expected explicit tool version mismatches');
}
if (typeof hakari.expected !== 'string' || typeof hakari.installed !== 'string') {
  throw new Error('expected hakari expected/installed versions');
}
const cargoVersionGate = doc.data.gates.find((g) => g.id === 'cargo-version');
if (!cargoVersionGate || cargoVersionGate.status !== 'fail') {
  throw new Error('expected cargo-version gate to fail under version-mismatch fixture');
}
NODE

echo "preflight.test.sh: ok"
