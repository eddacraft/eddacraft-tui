#!/usr/bin/env bash
# generate-licence-keypair.sh — Generate an ES256 keypair for licence signing.
#
# Usage:
#   bash scripts/generate-licence-keypair.sh
#
# Output:
#   Prints PEM-encoded private and public keys to stdout.
#   Copy them into your environment variables:
#     LICENSE_SIGNING_KEY — private key (API only, never commit)
#     LICENSE_PUBLIC_KEY  — public key (API + baked into CLI)

set -euo pipefail

TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

openssl ecparam -genkey -name prime256v1 -noout 2>/dev/null \
  | openssl pkcs8 -topk8 -nocrypt -out "$TMPDIR/private.pem" 2>/dev/null
openssl ec -in "$TMPDIR/private.pem" -pubout -out "$TMPDIR/public.pem" 2>/dev/null

echo "=== LICENSE_SIGNING_KEY (private — API env var only) ==="
echo ""
cat "$TMPDIR/private.pem"
echo ""
echo "=== LICENSE_PUBLIC_KEY (public — baked into CLI + API env var) ==="
echo ""
cat "$TMPDIR/public.pem"
echo ""
echo "Copy these into your environment. NEVER commit the private key."
