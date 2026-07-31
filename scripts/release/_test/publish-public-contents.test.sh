#!/usr/bin/env bash
# Fake-gh integration coverage for publish-public-contents.sh (GH #3310).
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
SCRIPT="$ROOT/scripts/release/publish-public-contents.sh"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

fail() {
  echo "publish-public-contents.test.sh: $*" >&2
  exit 1
}

[[ -x "$SCRIPT" || -f "$SCRIPT" ]] || fail "script missing: $SCRIPT"
chmod +x "$SCRIPT"

# Minimal fake `gh` that records invocations and returns canned content SHA.
cat >"$tmp/gh" <<'FAKE'
#!/usr/bin/env bash
set -euo pipefail
log="${ANVIL_FAKE_GH_LOG:?}"
printf '%s\n' "$*" >>"$log"
if [[ "${1:-}" == "api" ]]; then
  # Honour --jq for GET path (real gh filters server-side; fake does it client-side).
  jq_filter=""
  prev=""
  for arg in "$@"; do
    if [[ "$prev" == "--jq" || "$prev" == "-q" ]]; then
      jq_filter="$arg"
    fi
    prev="$arg"
  done
  # GET contents → optional existing sha
  if [[ "$*" != *"--method"* && "$*" != *"-X"* && "$*" != *"--input"* ]]; then
    if [[ -n "${ANVIL_FAKE_GH_EXISTING_SHA:-}" ]]; then
      body=$(printf '{"sha":"%s"}' "$ANVIL_FAKE_GH_EXISTING_SHA")
      if [[ -n "$jq_filter" ]]; then
        printf '%s\n' "$body" | jq -r "$jq_filter"
      else
        printf '%s\n' "$body"
      fi
      exit 0
    fi
    echo "HTTP 404" >&2
    exit 1
  fi
  # PUT with --input
  if [[ "$*" == *"--input"* ]]; then
    input_file=""
    prev=""
    for arg in "$@"; do
      if [[ "$prev" == "--input" ]]; then
        input_file="$arg"
      fi
      prev="$arg"
    done
    [[ -n "$input_file" && -f "$input_file" ]] || {
      echo "fake-gh: missing --input file" >&2
      exit 1
    }
    # Payload must be JSON with message + content (base64).
    jq -e '.message and .content' "$input_file" >/dev/null \
      || { echo "fake-gh: payload missing message/content" >&2; exit 1; }
    if [[ -n "${ANVIL_FAKE_GH_REQUIRE_SHA:-}" ]]; then
      jq -e --arg sha "$ANVIL_FAKE_GH_REQUIRE_SHA" '.sha == $sha' "$input_file" >/dev/null \
        || { echo "fake-gh: expected update sha $ANVIL_FAKE_GH_REQUIRE_SHA" >&2; exit 1; }
    fi
    if [[ -n "${ANVIL_FAKE_GH_FAIL_PUT:-}" ]]; then
      echo "fake-gh: simulated API failure" >&2
      exit 1
    fi
    # Assert content is not on argv (must use --input file).
    if printf '%s' "$*" | grep -q 'content='; then
      echo "fake-gh: content leaked onto argv" >&2
      exit 1
    fi
    printf '{"content":{"path":"ok"}}\n'
    exit 0
  fi
fi
echo "fake-gh: unhandled: $*" >&2
exit 1
FAKE
chmod +x "$tmp/gh"

export PATH="$tmp:$PATH"
export ANVIL_FAKE_GH_LOG="$tmp/gh.log"
file="$tmp/body.md"
printf 'hello acknowledgements\n' >"$file"

# Success: create
: >"$ANVIL_FAKE_GH_LOG"
unset ANVIL_FAKE_GH_EXISTING_SHA ANVIL_FAKE_GH_REQUIRE_SHA ANVIL_FAKE_GH_FAIL_PUT
"$SCRIPT" \
  --repo eddacraft/anvil \
  --path ACKNOWLEDGEMENTS.md \
  --file "$file" \
  --message "chore(release): update ACKNOWLEDGEMENTS.md" \
  >/dev/null || fail "create path failed"
grep -Fq 'contents/ACKNOWLEDGEMENTS.md' "$ANVIL_FAKE_GH_LOG" || fail "GET/PUT path not logged"

# Success: update existing
: >"$ANVIL_FAKE_GH_LOG"
export ANVIL_FAKE_GH_EXISTING_SHA="deadbeef"
export ANVIL_FAKE_GH_REQUIRE_SHA="deadbeef"
"$SCRIPT" \
  --repo eddacraft/anvil \
  --path ACKNOWLEDGEMENTS.md \
  --file "$file" \
  --message "chore(release): update ACKNOWLEDGEMENTS.md" \
  >/dev/null || fail "update path failed"

# Oversized content is refused before gh
big="$tmp/big.md"
dd if=/dev/zero of="$big" bs=1024 count=2 status=none 2>/dev/null || dd if=/dev/zero of="$big" bs=1024 count=2
if "$SCRIPT" \
  --repo eddacraft/anvil \
  --path big.md \
  --file "$big" \
  --message "too big" \
  --max-bytes 100 \
  >/dev/null 2>"$tmp/oversize.err"
then
  fail "expected oversized file to fail"
fi
grep -Fq 'max-bytes' "$tmp/oversize.err" || fail "oversized error missing context"

# API failure remains blocking
: >"$ANVIL_FAKE_GH_LOG"
unset ANVIL_FAKE_GH_EXISTING_SHA ANVIL_FAKE_GH_REQUIRE_SHA
export ANVIL_FAKE_GH_FAIL_PUT=1
if "$SCRIPT" \
  --repo eddacraft/anvil \
  --path ACKNOWLEDGEMENTS.md \
  --file "$file" \
  --message "will fail" \
  >/dev/null 2>"$tmp/put.err"
then
  fail "expected API failure to block"
fi

echo "publish-public-contents.test.sh: ok"
