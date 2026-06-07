#!/usr/bin/env bash
# Regression test: the acknowledgements generator must fail
# hard when a workspace crate is missing the `license` (or
# `license-file`) field. Without this, cargo-about emits a warning and
# the crate silently slips out of the generated ACKNOWLEDGEMENTS.md.
#
# The test sets up a minimal Cargo workspace with two crates:
#   - `good-crate`   — has `license = "MIT"` (compliant)
#   - `missing-crate` — has no `license` and no `license-file`
#
# It then runs `generate-acknowledgements.sh` and asserts:
#   1. The script exits non-zero.
#   2. The error output names the offending crate so the operator
#      can act on it.
#
# Local invocation:
#   tools/starters/acknowledgements/tests/strict-license-field.sh
#
# CI wires this into the Rust workflow's licence-lint job.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
GENERATOR="$SCRIPT_DIR/../generate-acknowledgements.sh"

if ! command -v cargo-about >/dev/null 2>&1; then
  echo "skip: cargo-about not installed; CI installs the pinned version" >&2
  exit 0
fi

if [ ! -x "$GENERATOR" ]; then
  echo "error: generator script not found or not executable at $GENERATOR" >&2
  exit 1
fi

# Stand up a self-contained workspace under a tmp dir; remove on exit
# regardless of outcome.
fixture_dir="$(mktemp -d)"
trap 'rm -rf "$fixture_dir"' EXIT

# Workspace Cargo.toml with two members.
cat >"$fixture_dir/Cargo.toml" <<'EOF'
[workspace]
resolver = "2"
members = ["good-crate", "missing-crate"]
EOF

mkdir -p "$fixture_dir/good-crate/src" "$fixture_dir/missing-crate/src"

# `good-crate` declares a licence — passes cargo-about cleanly.
cat >"$fixture_dir/good-crate/Cargo.toml" <<'EOF'
[package]
name = "good-crate"
version = "0.1.0"
edition = "2021"
license = "MIT"
publish = false
EOF
echo 'fn main() {}' >"$fixture_dir/good-crate/src/main.rs"

# `missing-crate` deliberately omits the licence field — must trip
# the strict lint.
cat >"$fixture_dir/missing-crate/Cargo.toml" <<'EOF'
[package]
name = "missing-crate"
version = "0.1.0"
edition = "2021"
publish = false

[dependencies]
good-crate = { path = "../good-crate" }
EOF
echo 'fn main() {}' >"$fixture_dir/missing-crate/src/main.rs"

# Minimal cargo-about config: accept MIT so `good-crate` is fine.
cat >"$fixture_dir/about.toml" <<'EOF'
accepted = ["MIT"]
EOF

# Minimal handlebars template — content doesn't matter; we only care
# about cargo-about's exit code.
cat >"$fixture_dir/about.hbs" <<'EOF'
{{#each overview}}
{{name}}
{{/each}}
EOF

# Splice target with the marker pair the generator expects.
cat >"$fixture_dir/ACKNOWLEDGEMENTS.md" <<'EOF'
# Acknowledgements

<!-- BEGIN AUTO-GENERATED -->
<!-- END AUTO-GENERATED -->
EOF

# attribution.toml pointing at the fixture workspace.
cat >"$fixture_dir/attribution.toml" <<EOF
[project]
target_path = "ACKNOWLEDGEMENTS.md"
fixit_command = "tools/starters/acknowledgements/generate-acknowledgements.sh"

[rust]
manifest_path = "missing-crate/Cargo.toml"
template_path = "about.hbs"
config_path = "about.toml"
EOF

# Run the generator inside the fixture dir; capture stderr separately
# so we can assert on the diagnostic.
stderr_file="$(mktemp)"
trap 'rm -rf "$fixture_dir" "$stderr_file"' EXIT

set +e
(
  cd "$fixture_dir"
  "$GENERATOR" --check
) >/dev/null 2>"$stderr_file"
exit_code=$?
set -e

if [ "$exit_code" -eq 0 ]; then
  echo "FAIL: generator exited 0 despite missing-crate having no license field" >&2
  echo "      expected a non-zero exit and an error mentioning missing-crate." >&2
  cat "$stderr_file" >&2
  exit 1
fi

if ! grep -qi "missing-crate" "$stderr_file"; then
  echo "FAIL: generator exited $exit_code but the error output did not name" >&2
  echo "      the offending crate. Operators won't know what to fix." >&2
  echo "----- captured stderr -----" >&2
  cat "$stderr_file" >&2
  echo "---------------------------" >&2
  exit 1
fi

# Discriminator: the script's pre-existing "empty file" fallback ALSO
# trips this case (cargo-about exits 0 silently and produces empty
# output). That path is a fragile coincidence — it depends on the
# template producing no output rather than on cargo-about enforcing
# anything. This test wants the actual cargo-about strict-mode
# diagnostic, which makes cargo-about itself exit non-zero. When
# cargo-about exits early, the script's empty-file check never fires.
# Pin "the empty-file sentinel did NOT fire" so a regression that
# drops `--fail` is caught here even though both code paths produce
# the WARN line that names the offending crate.
if grep -q "produced an empty file" "$stderr_file"; then
  echo "FAIL: generator failed via the empty-output fallback rather than" >&2
  echo "      cargo-about's strict-mode diagnostic. The contract requires" >&2
  echo "      passing --fail to cargo-about so missing license fields are" >&2
  echo "      caught at the canonical layer (cargo-about's own error)," >&2
  echo "      not the script's empty-file sentinel — that sentinel is" >&2
  echo "      a fragile coincidence that only fires when the template" >&2
  echo "      produces empty output." >&2
  echo "----- captured stderr -----" >&2
  cat "$stderr_file" >&2
  echo "---------------------------" >&2
  exit 1
fi

# Positive assertion: cargo-about's --fail mode emits the canonical
# diagnostic phrase "errors resolving licenses". Without --fail, the
# WARN lines contain "no `license` specified" but NOT this phrase.
if ! grep -q "errors resolving licenses" "$stderr_file"; then
  echo "FAIL: generator exited $exit_code but stderr does not include" >&2
  echo "      cargo-about's strict-mode 'errors resolving licenses'" >&2
  echo "      summary line. That summary is only emitted under --fail;" >&2
  echo "      its absence means cargo-about ran without --fail and the" >&2
  echo "      failure came from somewhere else." >&2
  echo "----- captured stderr -----" >&2
  cat "$stderr_file" >&2
  echo "---------------------------" >&2
  exit 1
fi

echo "ok: missing license field triggered cargo-about's strict diagnostic (exit $exit_code)"
