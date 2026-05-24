#!/usr/bin/env bash
# ATTRIB-013 Go-driver preflight test.
#
# Exercises the driver-author contract rule 1 (actionable errors for
# missing tools or missing state). Invokes `drivers/go.sh` directly —
# no dispatcher, no module download — so this test runs fast and does
# not require a real `go` / `go-licenses` (both are stubbed). Each
# scenario asserts a non-zero exit AND a substring match in stderr that
# proves the error is actionable.
#
# Scenarios:
#   1. Wrong argv count (driver-author contract: exactly two args)
#   2. module_path directory does not exist
#   3. go_allow_path file does not exist
#   4. module_path exists but no go.mod up the tree (not a Go module)
#   5. go-licenses not on PATH (controlled PATH)
#
# Local invocation:
#   tools/starters/acknowledgements/tests/go-driver-preflight.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
DRIVER="$SCRIPT_DIR/../drivers/go.sh"

if [ ! -x "$DRIVER" ]; then
  echo "error: driver script not found or not executable at $DRIVER" >&2
  exit 1
fi

fixture_root="$(mktemp -d)"
trap 'rm -rf "$fixture_root"' EXIT

allow_list="$fixture_root/licences.go-allow.txt"
printf '# BEGIN AUTO-GENERATED FROM licences.toml — go-allow\nMIT,Apache-2.0\n# END AUTO-GENERATED FROM licences.toml — go-allow\n' >"$allow_list"

# Stub `go` and `go-licenses` so the tool preflight passes for the
# file/state scenarios. They are never executed — those scenarios fail
# at a file/state check (or the pure-bash go.mod walk) before any tool
# invocation.
stub_bin_dir="$fixture_root/stub-bin"
mkdir -p "$stub_bin_dir"
for tool in go go-licenses; do
  cat >"$stub_bin_dir/$tool" <<STUB
#!/usr/bin/env bash
echo "stub $tool invoked — preflight tests should not reach tool execution" >&2
exit 99
STUB
  chmod +x "$stub_bin_dir/$tool"
done
export PATH="$stub_bin_dir:$PATH"

# ── Scenario 1: wrong argv count ─────────────────────────────────────
exit1=0
out1="$("$DRIVER" 2>&1 >/dev/null)" || exit1=$?
if [ "$exit1" -eq 0 ]; then
  echo "fail scenario 1: driver accepted 0 arguments instead of erroring" >&2
  exit 1
fi
if ! printf '%s' "$out1" | grep -q "expected 2 arguments"; then
  echo "fail scenario 1: stderr lacks 'expected 2 arguments' (got: $out1)" >&2
  exit 1
fi
echo "ok scenario 1: wrong argv count rejected (exit $exit1)"

# ── Scenario 2: module_path directory does not exist ─────────────────
config2_json="$(jq -n \
  --arg module "$fixture_root/does-not-exist/cmd" \
  --arg allow  "$allow_list" \
  '{name: "go", ecosystem: "go", module_path: $module, go_allow_path: $allow}')"
exit2=0
out2="$("$DRIVER" "$config2_json" "$fixture_root/out2.md" 2>&1 >/dev/null)" || exit2=$?
if [ "$exit2" -eq 0 ]; then
  echo "fail scenario 2: driver accepted nonexistent module_path" >&2
  exit 1
fi
if ! printf '%s' "$out2" | grep -q "module_path"; then
  echo "fail scenario 2: stderr lacks 'module_path' (got: $out2)" >&2
  exit 1
fi
echo "ok scenario 2: missing module_path rejected (exit $exit2)"

# ── Scenario 3: go_allow_path file does not exist ────────────────────
mod3="$fixture_root/mod3"
mkdir -p "$mod3"
config3_json="$(jq -n \
  --arg module "$mod3" \
  --arg allow  "$fixture_root/no-such-allow.txt" \
  '{name: "go", ecosystem: "go", module_path: $module, go_allow_path: $allow}')"
exit3=0
out3="$("$DRIVER" "$config3_json" "$fixture_root/out3.md" 2>&1 >/dev/null)" || exit3=$?
if [ "$exit3" -eq 0 ]; then
  echo "fail scenario 3: driver accepted nonexistent go_allow_path" >&2
  exit 1
fi
if ! printf '%s' "$out3" | grep -q "go_allow_path"; then
  echo "fail scenario 3: stderr lacks 'go_allow_path' (got: $out3)" >&2
  exit 1
fi
echo "ok scenario 3: missing go_allow_path rejected (exit $exit3)"

# ── Scenario 4: module_path exists but no go.mod up the tree ─────────
mod4="$fixture_root/mod4/cmd"
mkdir -p "$mod4"
config4_json="$(jq -n \
  --arg module "$mod4" \
  --arg allow  "$allow_list" \
  '{name: "go", ecosystem: "go", module_path: $module, go_allow_path: $allow}')"
exit4=0
out4="$("$DRIVER" "$config4_json" "$fixture_root/out4.md" 2>&1 >/dev/null)" || exit4=$?
if [ "$exit4" -eq 0 ]; then
  echo "fail scenario 4: driver accepted a module_path with no go.mod ancestor" >&2
  exit 1
fi
if ! printf '%s' "$out4" | grep -q "go.mod"; then
  echo "fail scenario 4: stderr lacks 'go.mod' (got: $out4)" >&2
  exit 1
fi
echo "ok scenario 4: missing go.mod rejected (exit $exit4)"

# ── Scenario 5: go-licenses not on PATH ──────────────────────────────
# Stage a Go module so the go.mod check passes; strip go-licenses from
# PATH so the tool preflight fails. Keep go (stub), jq, bash + coreutils.
mod5="$fixture_root/mod5"
mkdir -p "$mod5/cmd"
printf 'module example.com/mod5\n\ngo 1.21\n' >"$mod5/go.mod"
config5_json="$(jq -n \
  --arg module "$mod5/cmd" \
  --arg allow  "$allow_list" \
  '{name: "go", ecosystem: "go", module_path: $module, go_allow_path: $allow}')"

jq_bin="$(command -v jq)"
bash_bin="$(command -v bash)"
path_dir="$fixture_root/path-without-go-licenses"
mkdir -p "$path_dir"
ln -sf "$jq_bin"             "$path_dir/jq"
ln -sf "$bash_bin"           "$path_dir/bash"
ln -sf "$stub_bin_dir/go"    "$path_dir/go"
for tool in cat sed dirname basename mktemp rm sort awk grep cut head tr; do
  src="$(command -v "$tool" 2>/dev/null || true)"
  [ -n "$src" ] && ln -sf "$src" "$path_dir/$tool"
done

exit5=0
out5="$(PATH="$path_dir" "$DRIVER" "$config5_json" "$fixture_root/out5.md" 2>&1 >/dev/null)" || exit5=$?
if [ "$exit5" -eq 0 ]; then
  echo "fail scenario 5: driver ran with no go-licenses on PATH" >&2
  exit 1
fi
if ! printf '%s' "$out5" | grep -q "go-licenses"; then
  echo "fail scenario 5: stderr lacks 'go-licenses' (got: $out5)" >&2
  exit 1
fi
echo "ok scenario 5: missing go-licenses rejected (exit $exit5)"

echo
echo "ATTRIB-013 Go-driver preflight tests passed: 5/5 scenarios green."
