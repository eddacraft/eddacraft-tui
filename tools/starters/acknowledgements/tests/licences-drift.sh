#!/usr/bin/env bash
# ATTRIB-006 regression test: `expand-licences.sh --check` must detect
# drift between licences.toml and either consumer file (about.toml or
# deny.toml).
#
# The test stands up a self-contained fixture (a licences.toml plus
# stub about.toml, deny.toml, and licences.node-allow.txt with the
# BEGIN/END markers) and walks four scenarios:
#
#   1. Files match licences.toml → --check exits 0.
#   2. Add a licence to licences.toml without re-expanding → --check
#      exits non-zero and the diff names the new entry.
#   3. Hand-edit about.toml inside the markers → --check exits
#      non-zero and the diff names the hand-edit.
#   4. Hand-edit licences.node-allow.txt inside the markers →
#      --check exits non-zero and the diff names the hand-edit.
#      (ATTRIB-012 Node-fragment drift detection.)
#
# Local invocation:
#   tools/starters/acknowledgements/tests/licences-drift.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
EXPANDER="$SCRIPT_DIR/../expand-licences.sh"

if [ ! -x "$EXPANDER" ]; then
  echo "error: expander not found or not executable at $EXPANDER" >&2
  exit 1
fi

fixture_dir="$(mktemp -d)"
trap 'rm -rf "$fixture_dir"' EXIT

# Minimal licences.toml with two entries: one for both consumers, one
# about-only. Single-line notes only (matches the canonical schema).
cat >"$fixture_dir/licences.toml" <<'EOF'
[[licences]]
spdx = "MIT"
about = true
deny = true

[[licences]]
spdx = "OpenSSL"
about = true
deny = false
note = "About-only entry."
EOF

# Stub about.toml with the BEGIN/END markers in the right place.
cat >"$fixture_dir/about.toml" <<'EOF'
accepted = [
  # BEGIN AUTO-GENERATED FROM licences.toml — accepted
  # END AUTO-GENERATED FROM licences.toml — accepted
]
EOF

# Stub deny.toml mirror.
cat >"$fixture_dir/deny.toml" <<'EOF'
[licenses]
allow = [
  # BEGIN AUTO-GENERATED FROM licences.toml — allow
  # END AUTO-GENERATED FROM licences.toml — allow
]
EOF

# ATTRIB-012: stub licences.node-allow.txt. Its presence flips the
# expander into emitting the Node fragment (back-compat shape: absent
# file means "no Node block here", expander stays silent). Same
# marker-driven splice contract as about.toml / deny.toml.
cat >"$fixture_dir/licences.node-allow.txt" <<'EOF'
# BEGIN AUTO-GENERATED FROM licences.toml — node-allow
# END AUTO-GENERATED FROM licences.toml — node-allow
EOF

# --- Scenario 1: clean expand, then --check should pass --------------------

(cd "$fixture_dir" && "$EXPANDER") >/dev/null
if ! (cd "$fixture_dir" && "$EXPANDER" --check) >/dev/null 2>&1; then
  echo "FAIL: --check reports drift immediately after a clean expand" >&2
  (cd "$fixture_dir" && "$EXPANDER" --check) >&2 || true
  exit 1
fi
echo "ok scenario 1: clean expand → --check passes"

# --- Scenario 2: add a licence to licences.toml without re-expanding -------

cat >>"$fixture_dir/licences.toml" <<'EOF'

[[licences]]
spdx = "Apache-2.0"
about = true
deny = true
EOF

set +e
output_s2="$(cd "$fixture_dir" && "$EXPANDER" --check 2>&1)"
exit_s2=$?
set -e

if [ "$exit_s2" -eq 0 ]; then
  echo "FAIL scenario 2: added Apache-2.0 to licences.toml without re-expanding," >&2
  echo "    but --check exited 0. Drift was not detected." >&2
  exit 1
fi
if ! grep -q "Apache-2.0" <<<"$output_s2"; then
  echo "FAIL scenario 2: drift detected (exit $exit_s2) but the diff did not" >&2
  echo "    name the new licence. Operators won't know what to fix." >&2
  echo "----- output -----" >&2
  echo "$output_s2" >&2
  echo "------------------" >&2
  exit 1
fi
echo "ok scenario 2: new licence in licences.toml → --check detects drift"

# Re-expand to restore the clean baseline for scenario 3.
(cd "$fixture_dir" && "$EXPANDER") >/dev/null

# --- Scenario 3: hand-edit about.toml inside the markers --------------------

# Inject a bogus entry between the markers in about.toml.
python3 - "$fixture_dir/about.toml" <<'PY'
import sys, pathlib
p = pathlib.Path(sys.argv[1])
text = p.read_text()
marker = "# END AUTO-GENERATED FROM licences.toml — accepted"
text = text.replace(marker, '  "Bogus-1.0",\n  ' + marker, 1)
p.write_text(text)
PY

set +e
output_s3="$(cd "$fixture_dir" && "$EXPANDER" --check 2>&1)"
exit_s3=$?
set -e

if [ "$exit_s3" -eq 0 ]; then
  echo "FAIL scenario 3: hand-edited Bogus-1.0 into about.toml but --check" >&2
  echo "    exited 0. Hand-edits inside the markers must be detected as drift." >&2
  exit 1
fi
if ! grep -q "Bogus-1.0" <<<"$output_s3"; then
  echo "FAIL scenario 3: drift detected (exit $exit_s3) but the diff did not" >&2
  echo "    name the offending entry." >&2
  echo "----- output -----" >&2
  echo "$output_s3" >&2
  echo "------------------" >&2
  exit 1
fi
echo "ok scenario 3: hand-edit inside markers → --check detects drift"

# Re-expand to restore the clean baseline for scenario 4.
(cd "$fixture_dir" && "$EXPANDER") >/dev/null

# --- Scenario 4: hand-edit licences.node-allow.txt inside markers ---------

# Inject a bogus entry between the markers in licences.node-allow.txt.
# The Node fragment is a single semicolon-joined line, so inject the
# bogus SPDX into that line.
python3 - "$fixture_dir/licences.node-allow.txt" <<'PY'
import sys, pathlib
p = pathlib.Path(sys.argv[1])
text = p.read_text()
marker = "# END AUTO-GENERATED FROM licences.toml — node-allow"
# Insert a bogus SPDX directly above the END marker.
text = text.replace(marker, "Bogus-9.9\n" + marker, 1)
p.write_text(text)
PY

set +e
output_s4="$(cd "$fixture_dir" && "$EXPANDER" --check 2>&1)"
exit_s4=$?
set -e

if [ "$exit_s4" -eq 0 ]; then
  echo "FAIL scenario 4: hand-edited Bogus-9.9 into licences.node-allow.txt" >&2
  echo "    but --check exited 0. Hand-edits inside the Node markers must be" >&2
  echo "    detected as drift (ATTRIB-012 single-source guarantee)." >&2
  exit 1
fi
if ! grep -q "Bogus-9.9" <<<"$output_s4"; then
  echo "FAIL scenario 4: drift detected (exit $exit_s4) but the diff did not" >&2
  echo "    name the offending entry." >&2
  echo "----- output -----" >&2
  echo "$output_s4" >&2
  echo "------------------" >&2
  exit 1
fi
echo "ok scenario 4: hand-edit inside Node markers → --check detects drift"

echo ""
echo "ATTRIB-006/-012 drift test passed: all four scenarios green."
