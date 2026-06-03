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

# CIB-037: active-lint invokes `aps lint` once per file, so the fake appends its
# argv (>>) across invocations rather than overwriting; every active file must
# appear, and a uniform non-zero child status (7) must propagate as the aggregate
# exit status.
fake_aps="$tmp/fake-aps"
cat >"$fake_aps" <<'SH'
#!/usr/bin/env bash
printf '%s\n' "$@" >>"$FAKE_APS_ARGS"
exit 7
SH
chmod +x "$fake_aps"

args_log="$tmp/aps-args.log"
set +e
FAKE_APS_ARGS="$args_log" "${CHECK[@]}" --root "$tmp" --aps-bin "$fake_aps" >/dev/null 2>&1
status=$?
set -e

if [[ "$status" -ne 7 ]]; then
  printf 'expected uniform fake aps exit status 7 to propagate, got %s\n' "$status" >&2
  exit 1
fi

require_argv() {
  local expected="$1"
  if ! grep -qx "$expected" "$args_log"; then
    printf 'missing expected aps argv: %s\nargv:\n%s\n' "$expected" "$(cat "$args_log")" >&2
    exit 1
  fi
}

require_argv 'lint'
require_argv 'plans/index.aps.md'
require_argv 'plans/issues.md'
require_argv 'plans/modules/active.aps.md'
require_argv 'plans/execution/ACTIVE-001.actions.md'

# Each active file must get its own `aps lint` invocation (CIB-037 fix): one
# `lint` token per file. The tmp fixture has 4 active files.
lint_invocations="$(grep -cx 'lint' "$args_log")"
if [[ "$lint_invocations" -ne 4 ]]; then
  printf 'expected 4 per-file lint invocations, got %s\nargv:\n%s\n' \
    "$lint_invocations" "$(cat "$args_log")" >&2
  exit 1
fi

missing_json="$tmp/missing-aps.json"
set +e
"${CHECK[@]}" --root "$tmp" --aps-bin "$tmp/does-not-exist" --json >"$missing_json" 2>/dev/null
missing_status=$?
set -e

if [[ "$missing_status" -ne 2 ]]; then
  printf 'expected missing aps exit status 2, got %s\n' "$missing_status" >&2
  exit 1
fi

node -e '
const doc = JSON.parse(require("node:fs").readFileSync(process.argv[1], "utf8"));
if (doc.status !== 2) {
  throw new Error(`expected JSON status 2 for missing aps binary, got ${doc.status}`);
}
if (!doc.error) {
  throw new Error("expected JSON error for missing aps binary");
}
' "$missing_json"
