#!/usr/bin/env bash
# Dispatcher schema-validation tests. The new generator
# accepts either a back-compat flat `[rust]` table OR a `[[blocks]]`
# array, but never both, and validates each entry's shape before
# invoking any driver. This test pins the rejection paths so silent
# wire-up bugs in the dispatcher can't slip past review.
#
# All scenarios use bad configs that fail preflight, so no driver is
# ever invoked. cargo-about is not required to run this test.
#
# Local invocation:
#   tools/starters/acknowledgements/tests/dispatcher-schema-validation.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
GENERATOR="$SCRIPT_DIR/../generate-acknowledgements.sh"

if [ ! -x "$GENERATOR" ]; then
  echo "error: generator script not found or not executable at $GENERATOR" >&2
  exit 1
fi

fixture_root="$(mktemp -d)"
trap 'rm -rf "$fixture_root"' EXIT

# Shared template files reused across scenarios. The dispatcher should
# reject the bad configs before reading these, but they need to exist
# so a regression that postpones preflight past file-existence checks
# does not get a free pass.
make_common_files() {
  local dir="$1"
  mkdir -p "$dir"
  cat >"$dir/about.toml" <<'EOF'
accepted = ["MIT"]
EOF
  cat >"$dir/about.hbs" <<'EOF'
ignored
EOF
  cat >"$dir/ACKNOWLEDGEMENTS.md" <<'EOF'
# Acknowledgements

<!-- BEGIN AUTO-GENERATED -->
<!-- END AUTO-GENERATED -->
EOF
  # Minimal cargo workspace so the rust driver's file-existence
  # preflight (if it gets that far, which it should NOT in these
  # rejection cases) does not error first.
  mkdir -p "$dir/crate/src"
  cat >"$dir/crate/Cargo.toml" <<'EOF'
[package]
name = "crate"
version = "0.1.0"
edition = "2021"
license = "MIT"
publish = false
EOF
  echo 'fn main() {}' >"$dir/crate/src/main.rs"
}

# run_generator runs the dispatcher inside the named fixture dir and
# captures stderr. Echoes "EXIT=<code>" then prints stderr to stdout
# so the calling scenario can grep both.
run_generator() {
  local dir="$1"
  shift
  local stderr_file
  stderr_file="$(mktemp)"
  local exit_code=0
  set +e
  (
    cd "$dir"
    "$GENERATOR" "$@"
  ) >/dev/null 2>"$stderr_file"
  exit_code=$?
  set -e
  echo "EXIT=$exit_code"
  cat "$stderr_file"
  rm -f "$stderr_file"
}

# Scenarios assert: exit non-zero AND stderr contains the documented
# error fragment. Each scenario gets its own fixture directory so
# they cannot pollute each other.

# ── Scenario 1: mixed schema (flat [rust] AND [[blocks]] in same file)
scenario1="$fixture_root/mixed-schema"
make_common_files "$scenario1"
cat >"$scenario1/attribution.toml" <<EOF
[project]
target_path = "ACKNOWLEDGEMENTS.md"
fixit_command = "tools/starters/acknowledgements/generate-acknowledgements.sh"

[rust]
manifest_path = "crate/Cargo.toml"
template_path = "about.hbs"
config_path = "about.toml"

[[blocks]]
name = "rust"
ecosystem = "rust"
manifest_path = "crate/Cargo.toml"
template_path = "about.hbs"
config_path = "about.toml"
EOF

result1="$(run_generator "$scenario1" --check)"
exit1="$(echo "$result1" | awk -F= '/^EXIT=/ { print $2; exit }')"
if [ "$exit1" = "0" ]; then
  echo "FAIL scenario 1 (mixed schema): expected non-zero exit, got $exit1" >&2
  echo "$result1" >&2
  exit 1
fi
if ! echo "$result1" | grep -qi "mix\|both\|conflict\|mutually exclusive"; then
  echo "FAIL scenario 1 (mixed schema): error does not name the mixed-schema conflict" >&2
  echo "$result1" >&2
  exit 1
fi
echo "ok scenario 1: mixed flat-[rust] + [[blocks]] rejected (exit $exit1)"

# ── Scenario 2: [[blocks]] entry missing `name` field
scenario2="$fixture_root/missing-name"
make_common_files "$scenario2"
cat >"$scenario2/attribution.toml" <<EOF
[project]
target_path = "ACKNOWLEDGEMENTS.md"
fixit_command = "tools/starters/acknowledgements/generate-acknowledgements.sh"

[[blocks]]
ecosystem = "rust"
manifest_path = "crate/Cargo.toml"
template_path = "about.hbs"
config_path = "about.toml"
EOF

result2="$(run_generator "$scenario2" --check)"
exit2="$(echo "$result2" | awk -F= '/^EXIT=/ { print $2; exit }')"
if [ "$exit2" = "0" ]; then
  echo "FAIL scenario 2 (missing name): expected non-zero exit, got $exit2" >&2
  echo "$result2" >&2
  exit 1
fi
if ! echo "$result2" | grep -qi "name"; then
  echo "FAIL scenario 2 (missing name): error does not mention the missing 'name' field" >&2
  echo "$result2" >&2
  exit 1
fi
echo "ok scenario 2: [[blocks]] entry without name rejected (exit $exit2)"

# ── Scenario 3: [[blocks]] entry missing `ecosystem` field
scenario3="$fixture_root/missing-ecosystem"
make_common_files "$scenario3"
cat >"$scenario3/attribution.toml" <<EOF
[project]
target_path = "ACKNOWLEDGEMENTS.md"
fixit_command = "tools/starters/acknowledgements/generate-acknowledgements.sh"

[[blocks]]
name = "rust"
manifest_path = "crate/Cargo.toml"
template_path = "about.hbs"
config_path = "about.toml"
EOF

result3="$(run_generator "$scenario3" --check)"
exit3="$(echo "$result3" | awk -F= '/^EXIT=/ { print $2; exit }')"
if [ "$exit3" = "0" ]; then
  echo "FAIL scenario 3 (missing ecosystem): expected non-zero exit, got $exit3" >&2
  echo "$result3" >&2
  exit 1
fi
if ! echo "$result3" | grep -qi "ecosystem"; then
  echo "FAIL scenario 3 (missing ecosystem): error does not mention the missing 'ecosystem' field" >&2
  echo "$result3" >&2
  exit 1
fi
echo "ok scenario 3: [[blocks]] entry without ecosystem rejected (exit $exit3)"

# ── Scenario 4: unknown ecosystem (no drivers/<ecosystem>.sh exists)
scenario4="$fixture_root/unknown-ecosystem"
make_common_files "$scenario4"
cat >"$scenario4/attribution.toml" <<EOF
[project]
target_path = "ACKNOWLEDGEMENTS.md"
fixit_command = "tools/starters/acknowledgements/generate-acknowledgements.sh"

[[blocks]]
name = "unknown"
ecosystem = "nonexistent-eco"
manifest_path = "crate/Cargo.toml"
template_path = "about.hbs"
config_path = "about.toml"
EOF

result4="$(run_generator "$scenario4" --check)"
exit4="$(echo "$result4" | awk -F= '/^EXIT=/ { print $2; exit }')"
if [ "$exit4" = "0" ]; then
  echo "FAIL scenario 4 (unknown ecosystem): expected non-zero exit, got $exit4" >&2
  echo "$result4" >&2
  exit 1
fi
if ! echo "$result4" | grep -qiE "nonexistent-eco|no driver|drivers/nonexistent-eco.sh"; then
  echo "FAIL scenario 4 (unknown ecosystem): error does not name the missing driver" >&2
  echo "$result4" >&2
  exit 1
fi
echo "ok scenario 4: unknown ecosystem rejected (exit $exit4)"

# ── Scenario 5: duplicate `name` across [[blocks]] entries
scenario5="$fixture_root/duplicate-name"
make_common_files "$scenario5"
cat >"$scenario5/attribution.toml" <<EOF
[project]
target_path = "ACKNOWLEDGEMENTS.md"
fixit_command = "tools/starters/acknowledgements/generate-acknowledgements.sh"

[[blocks]]
name = "rust"
ecosystem = "rust"
manifest_path = "crate/Cargo.toml"
template_path = "about.hbs"
config_path = "about.toml"

[[blocks]]
name = "rust"
ecosystem = "rust"
manifest_path = "crate/Cargo.toml"
template_path = "about.hbs"
config_path = "about.toml"
EOF

result5="$(run_generator "$scenario5" --check)"
exit5="$(echo "$result5" | awk -F= '/^EXIT=/ { print $2; exit }')"
if [ "$exit5" = "0" ]; then
  echo "FAIL scenario 5 (duplicate name): expected non-zero exit, got $exit5" >&2
  echo "$result5" >&2
  exit 1
fi
if ! echo "$result5" | grep -qiE "duplicate|collision|same name|'rust'"; then
  echo "FAIL scenario 5 (duplicate name): error does not name the collision" >&2
  echo "$result5" >&2
  exit 1
fi
echo "ok scenario 5: duplicate block name rejected (exit $exit5)"

echo ""
echo "dispatcher schema-validation tests passed: 5/5 scenarios green."
