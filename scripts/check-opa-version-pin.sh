#!/bin/sh
# check-opa-version-pin.sh
#
# Assert that the pinned OPA version (`DEFAULT_OPA_VERSION` from
# packages/anvil/policy/src/opa-binary-manager.ts) only appears in an explicit
# allowlist of files. A new hit outside the allowlist means a contributor
# duplicated the version string without updating the bump runbook — fail CI so
# the next bump doesn't rot silently.
#
# Usage:
#   ./scripts/check-opa-version-pin.sh
#
# Exit codes:
#   0 — all hits accounted for
#   1 — unknown file contains the pinned version
#   2 — could not read DEFAULT_OPA_VERSION

set -eu

ROOT="$(git rev-parse --show-toplevel)"
BM="$ROOT/packages/anvil/policy/src/opa-binary-manager.ts"

VERSION="$(awk -F"'" '/const DEFAULT_OPA_VERSION/ { print $2 }' "$BM")"
if [ -z "${VERSION:-}" ]; then
  echo "check-opa-version-pin: could not read DEFAULT_OPA_VERSION from $BM" >&2
  exit 2
fi

# Files allowed to hard-code the pinned OPA version literal. When the bump
# runbook in docs/guides/opa-policy-testing.md changes, update this list too.
# Keep paths repo-relative, one per line, no leading/trailing whitespace.
ALLOWLIST="
packages/anvil/policy/src/opa-binary-manager.ts
packages/anvil/policy/src/opa-binary-manager.test.ts
.github/workflows/ci.yml
.github/workflows/rust.yml
docs/guides/opa-policy-testing.md
docs/archive/planning/opa-policy-engine.md
crates/anvil-policy/tests/opa_real_binary.rs
AGENTS.md
scripts/check-opa-version-pin.sh
"

hits="$(git grep -l -F "$VERSION" -- \
  ':(exclude)plans' \
  ':(exclude)CHANGELOG.md' \
  ':(exclude)**/*.lock' \
  ':(exclude)**/pnpm-lock.yaml' \
  ':(exclude)target' || true)"

unknown=""
for f in $hits; do
  if ! printf '%s\n' "$ALLOWLIST" | grep -qxF "$f"; then
    unknown="$unknown$f
"
  fi
done

if [ -n "$unknown" ]; then
  printf 'ERROR: pinned OPA version %s found outside the allowlist:\n' "$VERSION" >&2
  printf '%s' "$unknown" >&2
  echo '' >&2
  echo "If the reference is intentional, add the file to ALLOWLIST in" >&2
  echo "scripts/check-opa-version-pin.sh and update the bump runbook in" >&2
  echo "docs/guides/opa-policy-testing.md." >&2
  exit 1
fi

count="$(printf '%s\n' "$hits" | grep -c . || true)"
echo "check-opa-version-pin: ok (version=$VERSION, referenced in $count files)"
