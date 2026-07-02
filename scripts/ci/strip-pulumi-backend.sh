#!/usr/bin/env bash
# CIB-136: strip the top-level `backend:` mapping from a Pulumi.yaml so the CI
# PR preview can log in to a throwaway *local* Pulumi backend instead of the
# production azblob state. Reading the azblob state would need a storage-account
# key — a secret we must keep away from PR-controlled preview code.
#
# Fails closed. If the `backend:` block (or any residual `azblob://` state URL)
# survives the strip — for example a future Pulumi.yaml reformat the awk pattern
# no longer matches — the script errors out rather than letting the preview run
# against production state via ambient runner credentials on a self-hosted
# runner. Refusing beats silently reconnecting to prod.
#
# Usage: strip-pulumi-backend.sh <path-to-Pulumi.yaml>

set -euo pipefail

file="${1:?usage: strip-pulumi-backend.sh <Pulumi.yaml>}"

if [[ ! -f "${file}" ]]; then
  echo "::error::${file} not found — cannot prepare a credential-free preview" >&2
  exit 1
fi

tmp="${file}.ci-stripped"

# Drop the top-level `backend:` key (quoted or unquoted) and every line
# indented beneath it (including blank lines within the block). A nested
# `backend:` under another key is intentionally left alone — only the
# project-level backend pins state.
awk '
  /^["'\'']?backend["'\'']?[[:space:]]*:/ { skip = 1; next }
  skip == 1 && /^$/     { next }
  skip == 1 && /^[[:space:]]/ { next }
  { skip = 0; print }
' "${file}" >"${tmp}"
mv "${tmp}" "${file}"

# Fail-fast belt-and-braces. grep exits 0 on a match, so a match here means the
# strip did NOT remove the backend and we must refuse the preview. The first
# check refuses ANY surviving top-level backend key — quoted or unquoted,
# whatever the URL scheme; the second independently catches an azblob state
# URL surviving anywhere in the file.
if grep -Eq "^[\"']?backend[\"']?[[:space:]]*:" "${file}"; then
  echo "::error::backend block still present in ${file} after strip — refusing credential-free preview" >&2
  exit 1
fi
if grep -q 'azblob://' "${file}"; then
  echo "::error::azblob state URL still present in ${file} after strip — refusing credential-free preview" >&2
  exit 1
fi
