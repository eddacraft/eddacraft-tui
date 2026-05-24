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

# --- Scenario 5: deterministic note wrapping (ATTRIB-016) -------------------
#
# The expander wraps long `note` fields into `#`-prefixed comment lines.
# It must wrap on **code points**, not bytes, and must not depend on any
# external `fold`: `fold` counts bytes, so a note containing multi-byte
# UTF-8 (em dashes are 3 bytes) wraps at a different word boundary across
# coreutils implementations, producing spurious `--check` drift between a
# contributor's local `fold` and CI's (bit PR #1911, fixed in `898554a6`).
#
# This scenario stands up an isolated fixture with a long em-dash note and
# asserts:
#   a. no emitted note line exceeds 75 code points and every word survives;
#   b. the output matches a byte-exact golden (em dashes intact, no trailing
#      whitespace, exact code-point wrap boundaries);
#   c. regeneration is idempotent (--check passes after a clean expand);
#   d. output is byte-identical when a *poisoned* `fold` is first on PATH
#      — proving the wrap no longer shells out to `fold` at all.

s5_dir="$(mktemp -d)"
trap 'rm -rf "$fixture_dir" "$s5_dir"' EXIT

# A note with two em dashes, long enough to wrap to multiple lines, and
# positioned so a byte-counting wrap (em dash = 3 bytes) would break at a
# different word than a code-point-counting wrap.
s5_note="OpenSSL appears here for the ring crate workaround — pulled in transitively via the TLS stack — and is retained for cargo-about compatibility across targets."

cat >"$s5_dir/licences.toml" <<EOF
[[licences]]
spdx = "MIT"
about = true
deny = true

[[licences]]
spdx = "OpenSSL"
about = true
deny = false
note = "$s5_note"
EOF

cat >"$s5_dir/about.toml" <<'EOF'
accepted = [
  # BEGIN AUTO-GENERATED FROM licences.toml — accepted
  # END AUTO-GENERATED FROM licences.toml — accepted
]
EOF

cat >"$s5_dir/deny.toml" <<'EOF'
[licenses]
allow = [
  # BEGIN AUTO-GENERATED FROM licences.toml — allow
  # END AUTO-GENERATED FROM licences.toml — allow
]
EOF

(cd "$s5_dir" && "$EXPANDER") >/dev/null

# Extract the wrapped note lines: comment lines between the markers that
# are not the four generated-header lines. The note is the only entry
# carrying a comment, so every `  # `-prefixed line that isn't a header
# belongs to the wrapped note.
mapfile -t s5_note_lines < <(
  awk '
    /BEGIN AUTO-GENERATED FROM licences.toml — accepted/ { inblock=1; next }
    /END AUTO-GENERATED FROM licences.toml — accepted/   { inblock=0 }
    inblock && /^  # / {
      if ($0 ~ /Generated from licences\.toml/) next
      if ($0 ~ /Update licences\.toml/)         next
      if ($0 ~ /^  #$/)                          next
      if ($0 ~ /Source: licences\.toml/)         next
      print
    }
  ' "$s5_dir/about.toml"
)

if [ "${#s5_note_lines[@]}" -lt 2 ]; then
  echo "FAIL scenario 5: expected the long note to wrap to >=2 lines, got ${#s5_note_lines[@]}:" >&2
  printf '    %s\n' "${s5_note_lines[@]}" >&2
  exit 1
fi

# Assertions (a) width and (b) word-integrity, checked in python3 where
# len() counts code points. The note lines are passed via a file (argv),
# not stdin — the heredoc already occupies python's stdin.
printf '%s\n' "${s5_note_lines[@]}" >"$s5_dir/note-lines.txt"
python3 - "$s5_dir/note-lines.txt" "$s5_note" <<'PY'
import sys
lines_path, expected_note = sys.argv[1], sys.argv[2]
with open(lines_path, encoding="utf-8") as f:
    lines = [ln.rstrip("\n") for ln in f if ln.strip()]
words_out = []
for ln in lines:
    assert ln.startswith("  # "), f"line missing comment prefix: {ln!r}"
    body = ln[4:]
    cp = len(body)
    assert cp <= 75, f"note line exceeds 75 code points ({cp}): {body!r}"
    words_out.extend(body.split())
if words_out != expected_note.split():
    print("FAIL scenario 5: wrapped note does not reconstruct the original.", file=sys.stderr)
    print(f"  expected words: {expected_note.split()}", file=sys.stderr)
    print(f"  got words:      {words_out}", file=sys.stderr)
    sys.exit(1)
print(f"ok scenario 5a: note wrapped to {len(lines)} lines, all <=75 code points, words intact")
PY

# (b) byte-exact golden. The `.split()` check above proves words survive
# but tolerates whitespace changes; this pins the precise output bytes —
# em dashes intact, no trailing whitespace, the exact code-point wrap
# boundaries — so any change to the wrap (whitespace handling, width,
# a reintroduced byte-counting wrap) is caught, not just word loss.
s5_golden="$s5_dir/note.golden"
cat >"$s5_golden" <<'EOF'
  # OpenSSL appears here for the ring crate workaround — pulled in transitively
  # via the TLS stack — and is retained for cargo-about compatibility across
  # targets.
EOF
if ! diff -u "$s5_golden" <(printf '%s\n' "${s5_note_lines[@]}") >/dev/null; then
  echo "FAIL scenario 5b: wrapped note bytes differ from the pinned golden." >&2
  diff -u "$s5_golden" <(printf '%s\n' "${s5_note_lines[@]}") >&2 || true
  exit 1
fi
echo "ok scenario 5b: wrapped note matches the byte-exact golden"

# (c) idempotence: --check passes immediately after the clean expand.
if ! (cd "$s5_dir" && "$EXPANDER" --check) >/dev/null 2>&1; then
  echo "FAIL scenario 5c: --check reports drift right after a clean expand of the em-dash note" >&2
  (cd "$s5_dir" && "$EXPANDER" --check) >&2 || true
  exit 1
fi
echo "ok scenario 5c: em-dash note expand is idempotent under --check"

# (d) fold-independence: put a poisoned `fold` first on PATH. If the
# expander still shells out to `fold`, the note comments become garbage
# and about.toml changes. A fold-free wrap is unaffected.
mkdir -p "$s5_dir/poison-bin"
cat >"$s5_dir/poison-bin/fold" <<'EOF'
#!/usr/bin/env bash
# Poisoned fold: any caller gets obviously-wrong output so a regression
# that reintroduces `fold` for note wrapping is caught.
echo "POISONED-FOLD-OUTPUT-SHOULD-NEVER-APPEAR"
EOF
chmod +x "$s5_dir/poison-bin/fold"

s5_clean="$(cat "$s5_dir/about.toml")"
(cd "$s5_dir" && PATH="$s5_dir/poison-bin:$PATH" "$EXPANDER") >/dev/null
s5_poisoned="$(cat "$s5_dir/about.toml")"

if [ "$s5_clean" != "$s5_poisoned" ]; then
  echo "FAIL scenario 5d: about.toml changed when a poisoned 'fold' was on PATH." >&2
  echo "    The note wrap still depends on the external 'fold' binary —" >&2
  echo "    that is exactly the byte-vs-column drift ATTRIB-016 removes." >&2
  diff <(printf '%s' "$s5_clean") <(printf '%s' "$s5_poisoned") >&2 || true
  exit 1
fi
echo "ok scenario 5d: note wrap is independent of the 'fold' binary on PATH"

echo ""
echo "ATTRIB-006/-012/-016 drift test passed: all five scenarios green."
