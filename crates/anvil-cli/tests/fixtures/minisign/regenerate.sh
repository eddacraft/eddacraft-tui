#!/usr/bin/env bash
# Regenerate the test minisign keypair used by signature verification tests.
#
# The keypair is TEST-ONLY. It is committed so test fixtures stay
# deterministic without each contributor having to regenerate them. The
# production release pipeline uses a separate, never-committed keypair
# whose public key is injected at compile time via ANVIL_RELEASE_PUBLIC_KEY
# (see ADR-045).
#
# Usage: tests/fixtures/minisign/regenerate.sh
#   (run from the repo root)
#
# Requires: `rsign` (cargo install rsign2).

set -euo pipefail

DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$DIR"

if ! command -v rsign >/dev/null 2>&1; then
  echo "rsign not found — install with: cargo install rsign2" >&2
  exit 2
fi

rm -f anvil-test.key anvil-test.pub anvil-test.pub.b64

# -W generates an unencrypted secret key; the test suite cannot prompt for
# a password and the key has no protective value (it is committed).
echo "" | rsign generate -W -p anvil-test.pub -s anvil-test.key

# Extract the base64 line and write it without the comment so the
# `DEV_PUBLIC_KEY` constant in signature.rs can be compared verbatim.
tail -n 1 anvil-test.pub > anvil-test.pub.b64

cat <<EOF

Generated test keypair. Public key:
$(cat anvil-test.pub.b64)

→ Update DEV_PUBLIC_KEY in crates/anvil-cli/src/commands/update/signature.rs
  to match this value, then re-run the signature tests.
EOF
