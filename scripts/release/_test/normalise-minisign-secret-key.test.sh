#!/usr/bin/env bash

set -euo pipefail

repo_root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
normaliser="${repo_root}/scripts/release/normalise-minisign-secret-key.sh"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

fail() {
  echo "normalise-minisign-secret-key.test.sh: $*" >&2
  exit 1
}

material=$(head -c 96 /dev/zero | base64 | tr -d '\n')

printf 'untrusted comment: minisign secret key 0123456789ABCDEF\n%s\n' "$material" > "$tmp/full.key"
"$normaliser" "$tmp/full.key" "$tmp/full.out"
[ "$(sed -n '1p' "$tmp/full.out")" = "untrusted comment: minisign secret key 0123456789ABCDEF" ] \
  || fail "full key comment was not preserved"
[ "$(sed -n '2p' "$tmp/full.out")" = "$material" ] || fail "full key material changed"

printf '%s\n' "$material" > "$tmp/bare.key"
"$normaliser" "$tmp/bare.key" "$tmp/bare.out"
grep -Fqx "untrusted comment: legacy minisign secret key material" "$tmp/bare.out" \
  || fail "bare key material was not wrapped"
[ "$(sed -n '2p' "$tmp/bare.out")" = "$material" ] || fail "bare key material changed"
bare_mode=$(stat -c '%a' "$tmp/bare.out" 2>/dev/null || stat -f '%Lp' "$tmp/bare.out")
[ "$bare_mode" = "600" ] || fail "normalised key permissions are not 0600"

printf 'sentinel\n' > "$tmp/symlink-target"
ln -s "$tmp/symlink-target" "$tmp/symlink.out"
"$normaliser" "$tmp/bare.key" "$tmp/symlink.out"
[ ! -L "$tmp/symlink.out" ] || fail "normalised output remained a symlink"
[ "$(cat "$tmp/symlink-target")" = "sentinel" ] || fail "normaliser followed the output symlink"
[ "$(sed -n '2p' "$tmp/symlink.out")" = "$material" ] \
  || fail "normalised replacement output is invalid"

printf 'not base64!\n' > "$tmp/invalid.key"
if "$normaliser" "$tmp/invalid.key" "$tmp/invalid.out" >/dev/null 2>&1; then
  fail "invalid base64 key material passed"
fi

short_material=$(head -c 16 /dev/zero | base64 | tr -d '\n')
printf '%s\n' "$short_material" > "$tmp/short.key"
if "$normaliser" "$tmp/short.key" "$tmp/short.out" >/dev/null 2>&1; then
  fail "short decoded key material passed"
fi

printf 'untrusted comment: test\n%s\nextra\n' "$material" > "$tmp/extra.key"
if "$normaliser" "$tmp/extra.key" "$tmp/extra.out" >/dev/null 2>&1; then
  fail "three-line key passed"
fi

printf 'normalise-minisign-secret-key.test.sh: ok\n'
