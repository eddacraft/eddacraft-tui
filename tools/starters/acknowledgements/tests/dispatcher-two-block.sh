#!/usr/bin/env bash
# ATTRIB-008 dispatcher round-trip test. A `[[blocks]]` config with
# two stub-ecosystem blocks must:
#
#   1. Splice each block's content between its per-block markers
#      (`<!-- BEGIN AUTO-GENERATED <name> -->` …
#       `<!-- END AUTO-GENERATED <name> -->`)
#   2. Preserve hand-curated content above, between, and below blocks
#   3. Be idempotent: running twice in a row produces no diff
#   4. Survive partial regeneration: if only one block's source
#      changes, `--check` reports drift for that block and leaves the
#      other block byte-identical
#   5. Isolate failures: when one block's driver exits non-zero, the
#      on-disk target stays untouched (no partial clobber)
#
# Stub drivers used here are pure shell, deterministic, and live under
# the test's own fixture tree. The dispatcher locates drivers via the
# `ATTRIB_DRIVERS_DIR` env var when set, falling back to its own
# `drivers/` sibling directory. Tests rely on the env-var override so
# they do not need to plant test-only files in the production tree.
#
# Local invocation:
#   tools/starters/acknowledgements/tests/dispatcher-two-block.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
GENERATOR="$SCRIPT_DIR/../generate-acknowledgements.sh"

if [ ! -x "$GENERATOR" ]; then
  echo "error: generator script not found or not executable at $GENERATOR" >&2
  exit 1
fi

fixture_root="$(mktemp -d)"
trap 'rm -rf "$fixture_root"' EXIT

# ── Build a stub drivers directory ────────────────────────────────────
# Two stub drivers (`alpha` and `beta`) plus one that always fails
# (`brokeneco`). Each accepts `<block-config-json> <output-temp-path>`
# matching the driver-author contract.

drivers_dir="$fixture_root/drivers"
mkdir -p "$drivers_dir"

cat >"$drivers_dir/alpha.sh" <<'EOF'
#!/usr/bin/env bash
# Stub driver: emits deterministic content keyed on the block name.
set -euo pipefail
config_json="$1"
output_path="$2"
name="$(printf '%s' "$config_json" | sed -n 's/.*"name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p')"
cat >"$output_path" <<INNER
alpha-driver render for block: $name
- entry-one
- entry-two
INNER
EOF

cat >"$drivers_dir/beta.sh" <<'EOF'
#!/usr/bin/env bash
# Stub driver: emits content distinct from alpha so the test can pin
# per-block placement.
set -euo pipefail
config_json="$1"
output_path="$2"
name="$(printf '%s' "$config_json" | sed -n 's/.*"name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p')"
cat >"$output_path" <<INNER
beta-driver render for block: $name
- beta-only-line
INNER
EOF

cat >"$drivers_dir/brokeneco.sh" <<'EOF'
#!/usr/bin/env bash
# Stub driver that always exits non-zero. Used to exercise the
# dispatcher's failure-isolation contract.
set -euo pipefail
echo "brokeneco: deliberate driver failure for test" >&2
exit 1
EOF

chmod +x "$drivers_dir/alpha.sh" "$drivers_dir/beta.sh" "$drivers_dir/brokeneco.sh"

export ATTRIB_DRIVERS_DIR="$drivers_dir"

# ── Scenario 1: two-block round-trip + idempotency ────────────────────
scenario1="$fixture_root/two-block"
mkdir -p "$scenario1"
cat >"$scenario1/ACKNOWLEDGEMENTS.md" <<'EOF'
# Acknowledgements

Hand-curated intro above the first block.

## Alpha block

<!-- BEGIN AUTO-GENERATED alpha -->
stale-content-replace-me
<!-- END AUTO-GENERATED alpha -->

Hand-curated content between the two blocks.

## Beta block

<!-- BEGIN AUTO-GENERATED beta -->
also-stale
<!-- END AUTO-GENERATED beta -->

Hand-curated footer below the second block.
EOF
cat >"$scenario1/attribution.toml" <<EOF
[project]
target_path = "ACKNOWLEDGEMENTS.md"
fixit_command = "tools/starters/acknowledgements/generate-acknowledgements.sh"

[[blocks]]
name = "alpha"
ecosystem = "alpha"

[[blocks]]
name = "beta"
ecosystem = "beta"
EOF

(cd "$scenario1" && "$GENERATOR") >/dev/null

# Both blocks should now carry their respective driver output. The
# hand-curated content should be byte-identical.
if ! grep -q "alpha-driver render for block: alpha" "$scenario1/ACKNOWLEDGEMENTS.md"; then
  echo "FAIL scenario 1: alpha block did not receive alpha driver output" >&2
  cat "$scenario1/ACKNOWLEDGEMENTS.md" >&2
  exit 1
fi
if ! grep -q "beta-driver render for block: beta" "$scenario1/ACKNOWLEDGEMENTS.md"; then
  echo "FAIL scenario 1: beta block did not receive beta driver output" >&2
  cat "$scenario1/ACKNOWLEDGEMENTS.md" >&2
  exit 1
fi
for phrase in \
  "Hand-curated intro above the first block." \
  "Hand-curated content between the two blocks." \
  "Hand-curated footer below the second block."
do
  if ! grep -qF "$phrase" "$scenario1/ACKNOWLEDGEMENTS.md"; then
    echo "FAIL scenario 1: hand-curated phrase missing after splice: $phrase" >&2
    cat "$scenario1/ACKNOWLEDGEMENTS.md" >&2
    exit 1
  fi
done

# Idempotency: a second run must produce no diff.
checksum_before="$(sha256sum "$scenario1/ACKNOWLEDGEMENTS.md" | awk '{print $1}')"
(cd "$scenario1" && "$GENERATOR") >/dev/null
checksum_after="$(sha256sum "$scenario1/ACKNOWLEDGEMENTS.md" | awk '{print $1}')"
if [ "$checksum_before" != "$checksum_after" ]; then
  echo "FAIL scenario 1: second run produced a different file (not idempotent)" >&2
  exit 1
fi

# --check on the unchanged file must exit 0.
if ! (cd "$scenario1" && "$GENERATOR" --check) >/dev/null 2>&1; then
  echo "FAIL scenario 1: --check exited non-zero on a freshly generated file" >&2
  exit 1
fi
echo "ok scenario 1: two-block round-trip + idempotency + --check green"

# ── Scenario 2: partial-regeneration leaves the other block untouched ─
# Mutate ONE block's driver output, run --check, expect drift detected.
# Then write, then verify the OTHER block's content is byte-identical
# to what it was before the rewrite.
scenario2="$fixture_root/partial"
cp -r "$scenario1" "$scenario2"
beta_section_before="$(awk '
  /<!-- BEGIN AUTO-GENERATED beta -->/ { capture=1 }
  capture { print }
  /<!-- END AUTO-GENERATED beta -->/ { capture=0 }
' "$scenario2/ACKNOWLEDGEMENTS.md")"

# Swap the alpha driver for one that produces different content. The
# beta driver stays the same.
cat >"$drivers_dir/alpha.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
output_path="$2"
cat >"$output_path" <<INNER
alpha-driver UPDATED content
- new-entry
INNER
EOF
chmod +x "$drivers_dir/alpha.sh"

# --check must report drift now.
if (cd "$scenario2" && "$GENERATOR" --check) >/dev/null 2>&1; then
  echo "FAIL scenario 2: --check returned 0 after alpha driver changed (no drift detected)" >&2
  exit 1
fi

# Regenerate; alpha should update, beta should be untouched.
(cd "$scenario2" && "$GENERATOR") >/dev/null
beta_section_after="$(awk '
  /<!-- BEGIN AUTO-GENERATED beta -->/ { capture=1 }
  capture { print }
  /<!-- END AUTO-GENERATED beta -->/ { capture=0 }
' "$scenario2/ACKNOWLEDGEMENTS.md")"
if [ "$beta_section_before" != "$beta_section_after" ]; then
  echo "FAIL scenario 2: beta block content changed when only the alpha driver was updated" >&2
  echo "----- beta before -----" >&2
  printf '%s\n' "$beta_section_before" >&2
  echo "----- beta after -----" >&2
  printf '%s\n' "$beta_section_after" >&2
  exit 1
fi
if ! grep -q "alpha-driver UPDATED content" "$scenario2/ACKNOWLEDGEMENTS.md"; then
  echo "FAIL scenario 2: alpha block did not pick up the updated driver output" >&2
  exit 1
fi
echo "ok scenario 2: partial regeneration left beta byte-identical while alpha updated"

# Restore the alpha driver for the next scenario.
cat >"$drivers_dir/alpha.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
config_json="$1"
output_path="$2"
name="$(printf '%s' "$config_json" | sed -n 's/.*"name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p')"
cat >"$output_path" <<INNER
alpha-driver render for block: $name
- entry-one
- entry-two
INNER
EOF
chmod +x "$drivers_dir/alpha.sh"

# ── Scenario 3: driver failure leaves on-disk target untouched ────────
# A two-block config where the SECOND block's driver always fails.
# The first block has already produced output into a temp; the
# dispatcher must abort before the atomic mv so the live target is
# unmodified.
scenario3="$fixture_root/failure-isolation"
mkdir -p "$scenario3"
cat >"$scenario3/ACKNOWLEDGEMENTS.md" <<'EOF'
# Acknowledgements

PRISTINE intro that must not change.

<!-- BEGIN AUTO-GENERATED alpha -->
PRISTINE alpha placeholder
<!-- END AUTO-GENERATED alpha -->

PRISTINE middle that must not change.

<!-- BEGIN AUTO-GENERATED brokeneco -->
PRISTINE brokeneco placeholder
<!-- END AUTO-GENERATED brokeneco -->

PRISTINE footer that must not change.
EOF
cat >"$scenario3/attribution.toml" <<EOF
[project]
target_path = "ACKNOWLEDGEMENTS.md"
fixit_command = "tools/starters/acknowledgements/generate-acknowledgements.sh"

[[blocks]]
name = "alpha"
ecosystem = "alpha"

[[blocks]]
name = "brokeneco"
ecosystem = "brokeneco"
EOF

target_before_checksum="$(sha256sum "$scenario3/ACKNOWLEDGEMENTS.md" | awk '{print $1}')"

set +e
(cd "$scenario3" && "$GENERATOR") >/dev/null 2>&1
exit_code=$?
set -e
if [ "$exit_code" -eq 0 ]; then
  echo "FAIL scenario 3: generator exited 0 despite brokeneco driver failure" >&2
  exit 1
fi

target_after_checksum="$(sha256sum "$scenario3/ACKNOWLEDGEMENTS.md" | awk '{print $1}')"
if [ "$target_before_checksum" != "$target_after_checksum" ]; then
  echo "FAIL scenario 3: target file changed despite driver failure (clobber!)" >&2
  diff <(echo "$target_before_checksum") <(echo "$target_after_checksum") >&2 || true
  exit 1
fi
echo "ok scenario 3: driver failure left on-disk target byte-identical (exit $exit_code)"

echo ""
echo "ATTRIB-008 dispatcher two-block tests passed: 3/3 scenarios green."
