#!/usr/bin/env bash
# Fixture tests for scripts/docs/adr-integrity.sh.
#
# Each case builds a throwaway tree under $TMPDIR with a fake plans/decisions
# layout, runs the script against it, and asserts on exit code + output.

set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
target="${script_dir}/adr-integrity.sh"

tmp_root=$(mktemp -d)
trap 'rm -rf "${tmp_root}"' EXIT

failures=0
pass() { printf '  ok: %s\n' "$1"; }
fail() { printf '  FAIL: %s\n' "$1"; failures=$((failures + 1)); }

# Helper: run the script in a sandbox repo and capture (status, stdout)
run_case() {
  local name="$1"
  local case_dir="${tmp_root}/${name}"
  mkdir -p "${case_dir}/plans/decisions"
  # Symlink-copy the script into the sandbox so its repo-root resolution lands
  # on the sandbox, not this repo.
  mkdir -p "${case_dir}/scripts/docs"
  cp "${target}" "${case_dir}/scripts/docs/adr-integrity.sh"
  echo "${case_dir}"
}

make_log() {
  # $1 = case dir, $2..N = ADR-ID values to index (e.g. 001 002 011a)
  local case_dir="$1"
  shift
  local log="${case_dir}/plans/decisions/DECISION-LOG.md"
  printf '# Decision Log\n\n' >"${log}"
  for id in "$@"; do
    printf -- '- [%s](%s-fake.md) | fake | Accepted\n' "${id}" "${id}" >>"${log}"
  done
}

make_adr() {
  # $1 = case dir, $2 = filename basename (e.g. 001-foo.md)
  local case_dir="$1"
  : >"${case_dir}/plans/decisions/$2"
}

# Case 1: clean repo → exit 0
echo "case 1: clean"
c1=$(run_case clean)
make_adr "${c1}" "001-a.md"
make_adr "${c1}" "002-b.md"
make_log "${c1}" 001 002
if out=$(bash "${c1}/scripts/docs/adr-integrity.sh" 2>&1) && echo "${out}" | grep -q "OK:"; then
  pass "clean tree exits 0"
else
  fail "clean tree should exit 0; got: ${out}"
fi

# Case 2: duplicate number → exit 1
echo "case 2: duplicate"
c2=$(run_case duplicate)
make_adr "${c2}" "001-a.md"
make_adr "${c2}" "001-b.md"
make_log "${c2}" 001
if ! out=$(bash "${c2}/scripts/docs/adr-integrity.sh" 2>&1) && echo "${out}" | grep -q "duplicate ADR numbers"; then
  pass "duplicate number exits non-zero with FAIL message"
else
  fail "duplicate should fail; got: ${out}"
fi

# Case 3: file missing from log → exit 1
echo "case 3: file not indexed"
c3=$(run_case orphan-file)
make_adr "${c3}" "001-a.md"
make_adr "${c3}" "002-b.md"
make_log "${c3}" 001
if ! out=$(bash "${c3}/scripts/docs/adr-integrity.sh" 2>&1) && echo "${out}" | grep -q "not referenced in DECISION-LOG"; then
  pass "unindexed file exits non-zero"
else
  fail "unindexed file should fail; got: ${out}"
fi

# Case 4: log entry missing a file → exit 1
echo "case 4: log entry without file"
c4=$(run_case orphan-log)
make_adr "${c4}" "001-a.md"
make_log "${c4}" 001 002
if ! out=$(bash "${c4}/scripts/docs/adr-integrity.sh" 2>&1) && echo "${out}" | grep -q "no ADR file"; then
  pass "stale log entry exits non-zero"
else
  fail "stale log entry should fail; got: ${out}"
fi

# Case 5: next-available skips suffixed variants
echo "case 5: next-available with variant"
c5=$(run_case next-with-variant)
make_adr "${c5}" "000-a.md"
make_adr "${c5}" "011a-b.md"
make_log "${c5}" 000 011a
out=$(bash "${c5}/scripts/docs/adr-integrity.sh" 2>&1 || true)
if echo "${out}" | grep -q "next available ADR number: 001"; then
  pass "next-available reports 001 when 000 and 011a are taken"
else
  fail "expected next: 001 with 011a treated as occupying 011; got: ${out}"
fi

if [[ "${failures}" -gt 0 ]]; then
  echo "${failures} test case(s) failed"
  exit 1
fi
echo "all cases passed"
