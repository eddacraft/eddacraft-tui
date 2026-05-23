#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
CHECK=(node "$ROOT/scripts/aps/active-lint.mjs")

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

mkdir -p "$tmp/plans/modules" "$tmp/plans/archive/modules" "$tmp/plans/execution" "$tmp/plans/archive/legacy/aps-phases"

touch "$tmp/plans/index.aps.md"
touch "$tmp/plans/issues.md"
touch "$tmp/plans/modules/active.aps.md"
touch "$tmp/plans/execution/ACTIVE-001.actions.md"
touch "$tmp/plans/execution/ACTIVE-001.steps.md"
touch "$tmp/plans/archive/modules/old.aps.md"
touch "$tmp/plans/archive/legacy/aps-phases/phase-0-foundation.aps.md"

files="$(${CHECK[@]} --root "$tmp" --list-files)"

require_file() {
  local expected="$1"
  if ! grep -qx "$expected" <<<"$files"; then
    printf 'missing expected active lint file: %s\nfiles:\n%s\n' "$expected" "$files" >&2
    exit 1
  fi
}

reject_file() {
  local unexpected="$1"
  if grep -qx "$unexpected" <<<"$files"; then
    printf 'unexpected active lint file: %s\nfiles:\n%s\n' "$unexpected" "$files" >&2
    exit 1
  fi
}

require_file 'plans/index.aps.md'
require_file 'plans/issues.md'
require_file 'plans/modules/active.aps.md'
require_file 'plans/execution/ACTIVE-001.actions.md'

reject_file 'plans/execution/ACTIVE-001.steps.md'
reject_file 'plans/archive/modules/old.aps.md'
reject_file 'plans/archive/legacy/aps-phases/phase-0-foundation.aps.md'

json="$(${CHECK[@]} --root "$tmp" --list-files --json)"
printf '%s' "$json" | node -e '
const doc = JSON.parse(require("node:fs").readFileSync(0, "utf8"));
if (!Array.isArray(doc.files)) throw new Error("files must be an array");
if (!doc.files.includes("plans/modules/active.aps.md")) {
  throw new Error("JSON output omitted active module");
}
'
