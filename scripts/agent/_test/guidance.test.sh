#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
GUIDANCE="$ROOT/scripts/agent/guidance.sh"

tmp="$(mktemp -d)"
branch_fixture="$ROOT/scripts/agent/_test/guidance-branch-fixture.sh"
trap 'rm -rf "$tmp" "$branch_fixture"' EXIT

write_files() {
  local name="$1"
  shift
  printf '%s\n' "$@" > "$tmp/$name"
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

write_files release \
  ".github/workflows/release.yml" \
  "docs/guides/release-runbook.md"
release_json="$($GUIDANCE --files-from "$tmp/release" --json)"
printf '%s' "$release_json" | node -e 'JSON.parse(require("node:fs").readFileSync(0, "utf8"))'
assert_contains "$release_json" '"advisory":true'
assert_contains "$release_json" '"enforcement":"none"'
assert_contains "$release_json" '"riskClass":"release"'
assert_contains "$release_json" '"reviewTier":"mini"'
assert_contains "$release_json" 'release-readiness-impact'

write_files ci \
  ".github/workflows/ci.yml"
ci_json="$($GUIDANCE --files-from "$tmp/ci" --json)"
printf '%s' "$ci_json" | node -e 'JSON.parse(require("node:fs").readFileSync(0, "utf8"))'
assert_contains "$ci_json" '"riskClass":"ci"'
assert_contains "$ci_json" 'CI path/change detection impact'

write_files aps \
  "plans/modules/operating-model-migration.aps.md" \
  "plans/index.aps.md"
aps_text="$($GUIDANCE --files-from "$tmp/aps")"
assert_contains "$aps_text" 'Risk: aps'
assert_contains "$aps_text" 'plans/aps-rules.md'
assert_contains "$aps_text" 'pnpm lint:md'

write_files source \
  "packages/anvil/core/src/index.ts" \
  "crates/anvil-cli/src/main.rs"
source_json="$($GUIDANCE --files-from "$tmp/source" --json)"
printf '%s' "$source_json" | node -e 'JSON.parse(require("node:fs").readFileSync(0, "utf8"))'
assert_contains "$source_json" '"riskClass":"source"'
assert_contains "$source_json" 'pnpm typecheck'
assert_contains "$source_json" 'cargo test --workspace'

printf '%s\n' 'branch fixture' > "$branch_fixture"
branch_json="$($GUIDANCE --branch --base HEAD --json)"
printf '%s' "$branch_json" | node -e '
const doc = JSON.parse(require("node:fs").readFileSync(0, "utf8"));
if (doc.source !== "git-branch") throw new Error(`unexpected source: ${doc.source}`);
if (!doc.riskClasses.includes("agent-workflow")) throw new Error("missing agent-workflow risk");
if (!doc.changedFiles.includes("scripts/agent/_test/guidance-branch-fixture.sh")) throw new Error("missing branch fixture path");
'

if $GUIDANCE --branch --base refs/heads/guidance-test-missing-base >/dev/null 2>&1; then
  echo "expected branch mode with missing base to fail" >&2
  exit 1
fi

mkdir -p "$tmp/bin"
printf '%s\n' '#!/usr/bin/env bash' 'exit 1' > "$tmp/bin/gh"
chmod +x "$tmp/bin/gh"
pr_json="$(PATH="$tmp/bin:$PATH" $GUIDANCE --pr --base HEAD --json)"
printf '%s' "$pr_json" | node -e '
const doc = JSON.parse(require("node:fs").readFileSync(0, "utf8"));
if (doc.source !== "branch-fallback") throw new Error(`unexpected source: ${doc.source}`);
if (!doc.warnings.includes("PR diff unavailable; guidance fell back to branch/local git diff.")) throw new Error("missing fallback warning");
if (!doc.riskClasses.includes("agent-workflow")) throw new Error("missing agent-workflow risk");
if (!doc.changedFiles.includes("scripts/agent/_test/guidance-branch-fixture.sh")) throw new Error("missing fallback fixture path");
'

control_path=$'scripts/agent/control-\v-\e-path.sh'
write_files control "$control_path"
control_json="$($GUIDANCE --files-from "$tmp/control" --json)"
printf '%s' "$control_json" | node -e '
const doc = JSON.parse(require("node:fs").readFileSync(0, "utf8"));
if (!doc.changedFiles.some((path) => path.includes("control-"))) throw new Error("missing control-character path");
'

if $GUIDANCE --base >/dev/null 2>&1; then
  echo "expected --base without ref to fail" >&2
  exit 1
fi

if $GUIDANCE --files-from --json >/dev/null 2>&1; then
  echo "expected --files-from without path to fail" >&2
  exit 1
fi

write_files agent \
  ".claude/commands/council.md" \
  "scripts/agent/guidance.sh"
agent_text="$($GUIDANCE --files-from "$tmp/agent")"
assert_contains "$agent_text" 'Risk: agent-workflow'
assert_contains "$agent_text" 'review/council alignment'

echo "guidance.test.sh: ok"
