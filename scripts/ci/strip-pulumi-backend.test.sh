#!/usr/bin/env bash
# Fixture tests for strip-pulumi-backend.sh (CIB-136).
#
# Pins the awk backend-strip behaviour against sample Pulumi.yaml shapes so a
# future reformat that silently defeats the strip is caught here (or, failing
# that, by the script's own fail-fast assertion) instead of leaking the CI PR
# preview back onto production azblob state.

set -euo pipefail

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
stripper="${script_dir}/strip-pulumi-backend.sh"

tmp_dir=$(mktemp -d)
cleanup() {
  rm -rf "${tmp_dir}"
}
trap cleanup EXIT

fail() {
  echo "FAIL: $1" >&2
  exit 1
}

# Writes $2.. as a Pulumi.yaml under name $1, returns its path.
make_yaml() {
  local name="$1"
  shift
  local path="${tmp_dir}/${name}.yaml"
  printf '%s\n' "$@" >"${path}"
  echo "${path}"
}

# --- Case 1: normal backend block at EOF strips cleanly -----------------------
normal=$(make_yaml normal \
  'name: anvil-iac' \
  'runtime:' \
  '  name: nodejs' \
  'description: Infrastructure as Code for the Anvil monorepo' \
  'backend:' \
  '  url: azblob://pulumi-state')

bash "${stripper}" "${normal}"
grep -q '^backend:' "${normal}" && fail 'case1: backend: survived the strip'
grep -q 'azblob://' "${normal}" && fail 'case1: azblob URL survived the strip'
grep -q '^name: anvil-iac' "${normal}" || fail 'case1: dropped unrelated top-level key'
grep -q '^description:' "${normal}" || fail 'case1: dropped description key'

# --- Case 2: backend block in the MIDDLE, followed by another top-level key ----
middle=$(make_yaml middle \
  'name: anvil-iac' \
  'backend:' \
  '  url: azblob://pulumi-state' \
  'runtime:' \
  '  name: nodejs')

bash "${stripper}" "${middle}"
grep -q '^backend:' "${middle}" && fail 'case2: backend: survived the strip'
grep -q 'azblob://' "${middle}" && fail 'case2: azblob URL survived the strip'
grep -q '^runtime:' "${middle}" || fail 'case2: dropped the key following backend'
grep -q '  name: nodejs' "${middle}" || fail 'case2: dropped runtime child after backend'

# --- Case 3: inline flow-mapping backend on a single line ---------------------
inline=$(make_yaml inline \
  'name: anvil-iac' \
  'backend: { url: "azblob://pulumi-state" }' \
  'runtime:' \
  '  name: nodejs')

bash "${stripper}" "${inline}"
grep -q '^backend:' "${inline}" && fail 'case3: inline backend survived the strip'
grep -q 'azblob://' "${inline}" && fail 'case3: azblob URL survived inline strip'
grep -q '^runtime:' "${inline}" || fail 'case3: dropped key after inline backend'

# --- Case 4: a reformat the awk pattern misses MUST trip the assertion ---------
# A quoted top-level key `"backend":` is valid YAML but is not matched by the
# `^backend:` awk rule, so the azblob URL survives — the fail-fast must fire.
quoted=$(make_yaml quoted \
  'name: anvil-iac' \
  '"backend":' \
  '  url: azblob://pulumi-state')

if bash "${stripper}" "${quoted}" 2>"${tmp_dir}/quoted.err"; then
  fail 'case4: expected non-zero exit when the backend could not be stripped'
fi
grep -q 'refusing credential-free preview' "${tmp_dir}/quoted.err" ||
  fail 'case4: missing fail-fast error message'

# --- Case 5: missing file fails closed ----------------------------------------
if bash "${stripper}" "${tmp_dir}/does-not-exist.yaml" 2>/dev/null; then
  fail 'case5: expected non-zero exit for a missing Pulumi.yaml'
fi

echo "strip-pulumi-backend.test.sh: all cases passed"
