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

# Scenarios 6+ exercise gates that fire *after* block resolution, so they
# need a driver that exists and is executable. A stub driver via
# ATTRIB_DRIVERS_DIR (the documented test seam) keeps them independent of
# cargo-about and friends.
stub_drivers="$fixture_root/stub-drivers"
mkdir -p "$stub_drivers"
cat >"$stub_drivers/stub.sh" <<'EOF'
#!/usr/bin/env bash
printf 'STUB-GENERATED-ROW\n' >"$2"
EOF
chmod +x "$stub_drivers/stub.sh"

# ── Scenario 6: marker pair present but END precedes BEGIN
# The count gate passes (exactly one of each), but splicing a file in
# this state deletes everything from BEGIN to EOF. The dispatcher must
# refuse, and must leave the on-disk target byte-identical.
scenario6="$fixture_root/reversed-markers"
make_common_files "$scenario6"
cat >"$scenario6/attribution.toml" <<EOF
[project]
target_path = "ACKNOWLEDGEMENTS.md"
fixit_command = "tools/starters/acknowledgements/generate-acknowledgements.sh"

[[blocks]]
name = "stub"
ecosystem = "stub"
EOF
cat >"$scenario6/ACKNOWLEDGEMENTS.md" <<'EOF'
# Acknowledgements

<!-- END AUTO-GENERATED stub -->

Curated middle content.

<!-- BEGIN AUTO-GENERATED stub -->

## Hand-written tail

Curated tail that must survive.
EOF
before6="$(mktemp)"
cp "$scenario6/ACKNOWLEDGEMENTS.md" "$before6"

result6="$(ATTRIB_DRIVERS_DIR="$stub_drivers" run_generator "$scenario6")"
exit6="$(echo "$result6" | awk -F= '/^EXIT=/ { print $2; exit }')"
if [ "$exit6" = "0" ]; then
  echo "FAIL scenario 6 (reversed markers): expected non-zero exit, got $exit6" >&2
  echo "$result6" >&2
  exit 1
fi
if ! echo "$result6" | grep -qiE "order|before|precede|reversed"; then
  echo "FAIL scenario 6 (reversed markers): error does not name the ordering problem" >&2
  echo "$result6" >&2
  exit 1
fi
if ! diff -q "$before6" "$scenario6/ACKNOWLEDGEMENTS.md" >/dev/null; then
  echo "FAIL scenario 6 (reversed markers): on-disk target was modified" >&2
  diff -u "$before6" "$scenario6/ACKNOWLEDGEMENTS.md" >&2 || true
  exit 1
fi
rm -f "$before6"
echo "ok scenario 6: reversed marker pair rejected, target untouched (exit $exit6)"

# ── Scenario 7: orphaned marker pair (no matching block in config)
# A block removed or renamed in attribution.toml leaves its markers
# holding stale generated content forever. --check must not report green
# over it, and write mode must refuse rather than silently retain it.
scenario7="$fixture_root/orphan-markers"
make_common_files "$scenario7"
cat >"$scenario7/attribution.toml" <<EOF
[project]
target_path = "ACKNOWLEDGEMENTS.md"
fixit_command = "tools/starters/acknowledgements/generate-acknowledgements.sh"

[[blocks]]
name = "stub"
ecosystem = "stub"
EOF
cat >"$scenario7/ACKNOWLEDGEMENTS.md" <<'EOF'
# Acknowledgements

<!-- BEGIN AUTO-GENERATED stub -->
STUB-GENERATED-ROW
<!-- END AUTO-GENERATED stub -->

<!-- BEGIN AUTO-GENERATED retired-block -->
| stale-dep | 0.0.1 | GPL-3.0 | never-updated |
<!-- END AUTO-GENERATED retired-block -->
EOF

for mode7 in "--check" ""; do
  label7="${mode7:-write}"
  # shellcheck disable=SC2086 # intentional word-split: empty means write mode
  result7="$(ATTRIB_DRIVERS_DIR="$stub_drivers" run_generator "$scenario7" $mode7)"
  exit7="$(echo "$result7" | awk -F= '/^EXIT=/ { print $2; exit }')"
  if [ "$exit7" = "0" ]; then
    echo "FAIL scenario 7 ($label7): orphaned marker pair accepted, got exit $exit7" >&2
    echo "$result7" >&2
    exit 1
  fi
  if ! echo "$result7" | grep -q "retired-block"; then
    echo "FAIL scenario 7 ($label7): error does not name the orphaned block" >&2
    echo "$result7" >&2
    exit 1
  fi
  echo "ok scenario 7 ($label7): orphaned marker pair rejected by name (exit $exit7)"
done

# ── Scenario 8: a `#` inside a quoted value is data, not a comment
# TOML basic strings may contain `#`. A parser that strips inline
# comments before handling quotes truncates the value and hands the
# driver a path that silently points somewhere else.
scenario8="$fixture_root/hash-in-quoted-value"
make_common_files "$scenario8"
mkdir -p "$scenario8/vendor/c#sharp"
touch "$scenario8/vendor/c#sharp/Cargo.toml"
cat >"$scenario8/attribution.toml" <<EOF
[project]
target_path = "ACKNOWLEDGEMENTS.md"
fixit_command = "tools/starters/acknowledgements/generate-acknowledgements.sh"

[[blocks]]
name = "stub"
ecosystem = "stub"
manifest_path = "vendor/c#sharp/Cargo.toml"   # trailing comment must still be stripped
EOF
cat >"$scenario8/ACKNOWLEDGEMENTS.md" <<'EOF'
# Acknowledgements

<!-- BEGIN AUTO-GENERATED stub -->
<!-- END AUTO-GENERATED stub -->
EOF

# This stub records what the dispatcher actually passed it.
echo_drivers="$fixture_root/echo-drivers"
mkdir -p "$echo_drivers"
cat >"$echo_drivers/stub.sh" <<EOF
#!/usr/bin/env bash
printf '%s' "\$(printf '%s' "\$1" | jq -r '.manifest_path')" >"$fixture_root/seen-manifest-path"
printf 'STUB-GENERATED-ROW\n' >"\$2"
EOF
chmod +x "$echo_drivers/stub.sh"

result8="$(ATTRIB_DRIVERS_DIR="$echo_drivers" run_generator "$scenario8")"
exit8="$(echo "$result8" | awk -F= '/^EXIT=/ { print $2; exit }')"
if [ "$exit8" != "0" ]; then
  echo "FAIL scenario 8 (# in quoted value): expected exit 0, got $exit8" >&2
  echo "$result8" >&2
  exit 1
fi
seen8="$(cat "$fixture_root/seen-manifest-path" 2>/dev/null || echo "<nothing>")"
if [ "$seen8" != "$scenario8/vendor/c#sharp/Cargo.toml" ]; then
  echo "FAIL scenario 8 (# in quoted value): driver received a truncated path" >&2
  echo "  expected: $scenario8/vendor/c#sharp/Cargo.toml" >&2
  echo "  actual:   $seen8" >&2
  exit 1
fi
echo "ok scenario 8: '#' inside a quoted value survives, trailing comment stripped"

# ── Scenario 9: the orphan gate must not fire on documentation
# A target may legitimately document the marker syntax — in a fenced
# code block, or with a marker-shaped string that is not a real marker.
# Treating those as live orphans would fail a consumer's build over their
# own prose, in both modes, with no in-tool way out. Regression guard for
# the false positive found in review.
scenario9="$fixture_root/documented-markers"
make_common_files "$scenario9"
cat >"$scenario9/attribution.toml" <<EOF
[project]
target_path = "ACKNOWLEDGEMENTS.md"
fixit_command = "tools/starters/acknowledgements/generate-acknowledgements.sh"

[[blocks]]
name = "stub"
ecosystem = "stub"
EOF
cat >"$scenario9/ACKNOWLEDGEMENTS.md" <<'MDEOF'
# Acknowledgements

## How to add a block

Declare the block in `attribution.toml`, then add its marker pair here:

```markdown
<!-- BEGIN AUTO-GENERATED my-new-block -->
<!-- END AUTO-GENERATED my-new-block -->
```

<!-- BEGIN AUTO-GENERATED stub -->
STUB-GENERATED-ROW
<!-- END AUTO-GENERATED stub -->
MDEOF

for mode9 in "--check" ""; do
  label9="${mode9:-write}"
  # shellcheck disable=SC2086 # intentional word-split: empty means write mode
  result9="$(ATTRIB_DRIVERS_DIR="$stub_drivers" run_generator "$scenario9" $mode9)"
  exit9="$(echo "$result9" | awk -F= '/^EXIT=/ { print $2; exit }')"
  if [ "$exit9" != "0" ]; then
    echo "FAIL scenario 9 ($label9): fenced marker documentation treated as a live block" >&2
    echo "$result9" >&2
    exit 1
  fi
  echo "ok scenario 9 ($label9): fenced marker documentation ignored (exit $exit9)"
done

# ── Scenario 10: custom markers whose END text extends the BEGIN text
# With marker_begin "<!-- gen -->" and marker_end "<!-- gen end -->", the
# begin-marker stem is a prefix of every end marker too. Extracting a
# name from an end marker must not invent a bogus block name.
scenario10="$fixture_root/overlapping-markers"
make_common_files "$scenario10"
cat >"$scenario10/attribution.toml" <<EOF
[project]
target_path = "ACKNOWLEDGEMENTS.md"
fixit_command = "tools/starters/acknowledgements/generate-acknowledgements.sh"
marker_begin = "<!-- gen -->"
marker_end = "<!-- gen end -->"

[[blocks]]
name = "stub"
ecosystem = "stub"
EOF
cat >"$scenario10/ACKNOWLEDGEMENTS.md" <<'MDEOF'
# Acknowledgements

<!-- gen stub -->
STUB-GENERATED-ROW
<!-- gen end stub -->
MDEOF

result10="$(ATTRIB_DRIVERS_DIR="$stub_drivers" run_generator "$scenario10" --check)"
exit10="$(echo "$result10" | awk -F= '/^EXIT=/ { print $2; exit }')"
if [ "$exit10" != "0" ]; then
  echo "FAIL scenario 10: overlapping custom marker stems produced a bogus orphan" >&2
  echo "$result10" >&2
  exit 1
fi
echo "ok scenario 10: overlapping custom marker stems parse cleanly (exit $exit10)"

echo ""
echo "dispatcher schema-validation tests passed: 10/10 scenarios green."
