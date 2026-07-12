#!/usr/bin/env bash

set -euo pipefail

die() {
  echo "normalise-minisign-secret-key: $*" >&2
  exit 1
}

[ "$#" -eq 2 ] || die "usage: $0 <decoded-input> <normalised-output>"
input="$1"
output="$2"

[ -f "$input" ] || die "decoded key input does not exist"
[ -n "$output" ] || die "normalised output path is empty"

has_nul=$(LC_ALL=C od -An -v -tu1 "$input" | awk '
  {
    for (i = 1; i <= NF; i++) {
      if ($i == 0) {
        print "yes"
        exit
      }
    }
  }
')

if [ "$has_nul" = "yes" ]; then
  comment="untrusted comment: legacy raw minisign secret key material"
  key_material=$(base64 < "$input" | tr -d '\r\n')
else
  lines=()
  while IFS= read -r line || [ -n "$line" ]; do
    lines+=("${line%$'\r'}")
  done < "$input"
  case "${#lines[@]}" in
    1)
      comment="untrusted comment: legacy minisign secret key material"
      key_material="${lines[0]}"
      ;;
    2)
      comment="${lines[0]}"
      key_material="${lines[1]}"
      [[ "$comment" =~ ^untrusted\ comment: ]] \
        || die "two-line key must start with an untrusted comment"
      ;;
    *)
      die "decoded key must contain a comment plus key material, legacy one-line key material, or a raw key blob"
      ;;
  esac
fi

[ -n "$key_material" ] || die "decoded key material is empty"

raw=$(mktemp)
tmp_output=""
cleanup() {
  rm -f "$raw"
  [ -z "$tmp_output" ] || rm -f "$tmp_output"
}
trap cleanup EXIT
if printf '%s' "$key_material" | base64 -d > "$raw" 2>/dev/null; then
  :
elif printf '%s' "$key_material" | base64 -D > "$raw" 2>/dev/null; then
  :
else
  die "decoded key material is not valid base64"
fi

decoded_size=$(stat -c '%s' "$raw" 2>/dev/null || stat -f '%z' "$raw")
[ "$decoded_size" -ge 64 ] \
  || die "decoded minisign key material is ${decoded_size} bytes; looks truncated"

umask 077
output_dir=$(dirname "$output")
output_name=$(basename "$output")
tmp_output=$(mktemp "${output_dir}/.${output_name}.XXXXXX")
printf '%s\n%s\n' "$comment" "$key_material" > "$tmp_output"
chmod 600 "$tmp_output"
mv -f -- "$tmp_output" "$output"
tmp_output=""
