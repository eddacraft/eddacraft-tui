#!/usr/bin/env bash
set -euo pipefail

# Tests for scripts/release/bump-homebrew.sh.
#
# Covers the offline transformation surface: input validation, class rename,
# unchanged content, and dry-run mode. Network-publishing paths (`--publish`)
# are exercised via fixtures that point GH_API_OVERRIDE at a recorded
# response stream so tests stay deterministic and air-gapped.

ROOT="$(git rev-parse --show-toplevel)"
SCRIPT="$ROOT/scripts/release/bump-homebrew.sh"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

fixture_formula() {
  cat <<'RUBY'
class EddacraftAnvil < Formula
  desc "Save-time trust for AI-generated code"
  homepage "https://anvil.eddacraft.ai"
  version "0.7.0-beta"
  url "https://github.com/eddacraft/anvil/releases/download/v0.7.0-beta/eddacraft-anvil-aarch64-apple-darwin.tar.xz"
  sha256 "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef"

  def install
    bin.install "anvil"
  end

  test do
    system "#{bin}/anvil", "--version"
  end
end
RUBY
}

assert_contains() {
  local haystack="$1"
  local needle="$2"
  if [[ "$haystack" != *"$needle"* ]]; then
    echo "expected output to contain: $needle" >&2
    echo "actual:" >&2
    echo "$haystack" >&2
    exit 1
  fi
}

assert_not_contains() {
  local haystack="$1"
  local needle="$2"
  if [[ "$haystack" == *"$needle"* ]]; then
    echo "expected output NOT to contain: $needle" >&2
    echo "actual:" >&2
    echo "$haystack" >&2
    exit 1
  fi
}

# 1. --help works.
bash "$SCRIPT" --help >"$tmp/help.out"
assert_contains "$(<"$tmp/help.out")" "Usage:"
assert_contains "$(<"$tmp/help.out")" "--release-tag"
assert_contains "$(<"$tmp/help.out")" "--formula-source"
assert_contains "$(<"$tmp/help.out")" "--publish"

# 2. Missing required args → exit 64 (EX_USAGE) and exhaustive stderr.
rc=0
bash "$SCRIPT" 2>"$tmp/missing.err" || rc=$?
if [[ "$rc" != "64" ]]; then
  echo "expected exit 64 for missing args, got $rc" >&2
  exit 1
fi
assert_contains "$(<"$tmp/missing.err")" "--release-tag is required"

# 3. Invalid release tag (not semver-ish) → exit 64.
rc=0
bash "$SCRIPT" \
  --release-tag "not-a-tag" \
  --formula-source "$tmp/missing.rb" \
  --out "$tmp/anvil.rb" 2>"$tmp/invalid-tag.err" || rc=$?
if [[ "$rc" != "64" ]]; then
  echo "expected exit 64 for invalid tag, got $rc" >&2
  exit 1
fi
assert_contains "$(<"$tmp/invalid-tag.err")" "--release-tag must look like v"

# 4. Missing formula source → exit 66 (EX_NOINPUT).
rc=0
bash "$SCRIPT" \
  --release-tag "v0.7.0-beta" \
  --formula-source "$tmp/nonexistent.rb" \
  --out "$tmp/anvil.rb" 2>"$tmp/missing-source.err" || rc=$?
if [[ "$rc" != "66" ]]; then
  echo "expected exit 66 for missing formula source, got $rc" >&2
  exit 1
fi
assert_contains "$(<"$tmp/missing-source.err")" "formula source not found"

# 5. Happy path: transform writes patched formula at --out, class renamed.
fixture_formula >"$tmp/eddacraft-anvil.rb"
bash "$SCRIPT" \
  --release-tag "v0.7.0-beta" \
  --formula-source "$tmp/eddacraft-anvil.rb" \
  --out "$tmp/anvil.rb" >"$tmp/happy.out"

if [[ ! -f "$tmp/anvil.rb" ]]; then
  echo "expected output formula at $tmp/anvil.rb" >&2
  exit 1
fi
patched="$(<"$tmp/anvil.rb")"
assert_contains "$patched" "class Anvil < Formula"
assert_not_contains "$patched" "class EddacraftAnvil"
# Body preserved (sha256 line is unchanged).
assert_contains "$patched" "sha256 \"deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef\""
assert_contains "$(<"$tmp/happy.out")" "wrote $tmp/anvil.rb"

# 6. Source formula already has 'class Anvil' (idempotency / unusual upstream).
cat >"$tmp/already-renamed.rb" <<'RUBY'
class Anvil < Formula
  desc "Save-time trust"
  version "0.7.0-beta"
end
RUBY
bash "$SCRIPT" \
  --release-tag "v0.7.0-beta" \
  --formula-source "$tmp/already-renamed.rb" \
  --out "$tmp/anvil2.rb" >"$tmp/already.out"
assert_contains "$(<"$tmp/anvil2.rb")" "class Anvil < Formula"

# 7. Refuse to write garbage: if the patched formula does not contain
#    'class Anvil < Formula', exit 70 (EX_SOFTWARE).
cat >"$tmp/broken.rb" <<'RUBY'
class SomethingElse < Formula
  version "0.7.0-beta"
end
RUBY
rc=0
bash "$SCRIPT" \
  --release-tag "v0.7.0-beta" \
  --formula-source "$tmp/broken.rb" \
  --out "$tmp/broken.out.rb" 2>"$tmp/broken.err" || rc=$?
if [[ "$rc" != "70" ]]; then
  echo "expected exit 70 for unpatchable formula, got $rc" >&2
  exit 1
fi
assert_contains "$(<"$tmp/broken.err")" "patched formula is missing"
# Output should NOT have been written.
if [[ -f "$tmp/broken.out.rb" ]]; then
  echo "expected $tmp/broken.out.rb to not exist after failed patch" >&2
  exit 1
fi

# 8. --dry-run with --publish: skip the network call, report what would be
#    sent. Useful for CI dry-run on candidate SHA (per DISTRIB-003 validation).
fixture_formula >"$tmp/eddacraft-anvil.rb"
bash "$SCRIPT" \
  --release-tag "v0.7.0-beta" \
  --formula-source "$tmp/eddacraft-anvil.rb" \
  --out "$tmp/anvil-dry.rb" \
  --publish \
  --tap-repo "eddacraft/homebrew-tap" \
  --dry-run >"$tmp/dry.out"
assert_contains "$(<"$tmp/dry.out")" "DRY-RUN"
assert_contains "$(<"$tmp/dry.out")" "eddacraft/homebrew-tap"
assert_contains "$(<"$tmp/dry.out")" "Formula/anvil.rb"
assert_contains "$(<"$tmp/dry.out")" "anvil v0.7.0-beta"
# Patched formula must still exist on disk (dry-run does not skip the
# transform — only the network publish).
if [[ ! -f "$tmp/anvil-dry.rb" ]]; then
  echo "expected dry-run to still write the patched formula locally" >&2
  exit 1
fi

# 9. --publish without --tap-repo defaults to eddacraft/homebrew-tap.
bash "$SCRIPT" \
  --release-tag "v0.7.0-beta" \
  --formula-source "$tmp/eddacraft-anvil.rb" \
  --out "$tmp/anvil-default-tap.rb" \
  --publish \
  --dry-run >"$tmp/default-tap.out"
assert_contains "$(<"$tmp/default-tap.out")" "eddacraft/homebrew-tap"

# 10. --publish without GH_TOKEN (or override) and without --dry-run → exit 78
#     (EX_CONFIG). The CI workflow exports GH_TOKEN; this guards local misuse.
rc=0
env -u GH_TOKEN -u ANVIL_RELEASES_TOKEN \
  bash "$SCRIPT" \
    --release-tag "v0.7.0-beta" \
    --formula-source "$tmp/eddacraft-anvil.rb" \
    --out "$tmp/anvil-notoken.rb" \
    --publish 2>"$tmp/notoken.err" || rc=$?
if [[ "$rc" != "78" ]]; then
  echo "expected exit 78 when GH_TOKEN is unset and --publish is set, got $rc" >&2
  exit 1
fi
assert_contains "$(<"$tmp/notoken.err")" "GH_TOKEN"

echo "bump-homebrew.test.sh: all assertions passed"
