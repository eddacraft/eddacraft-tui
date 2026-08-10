#!/usr/bin/env bash
# Publish a local file to a public GitHub repository via the Contents API.
#
# Extracted from release.yml so recovery and CI can exercise the GitHub
# interaction boundary with a fake `gh` (GH #3310) without live network calls.
#
# Usage:
#   publish-public-contents.sh \
#     --repo owner/name \
#     --path path/in/repo.md \
#     --file /local/file.md \
#     --message "chore(release): update file" \
#     [--max-bytes N]
#
# Requires: gh, jq, base64. Honours GH_TOKEN / PATH (for fake-gh harness).
set -euo pipefail

usage() {
  sed -n '2,20p' "$0"
}

die() {
  echo "publish-public-contents: $*" >&2
  exit 1
}

require_value() {
  case "${2:-}" in
    "" | --*) die "$1 requires a value" ;;
  esac
}

repo=""
remote_path=""
local_file=""
message=""
max_bytes=1048576

while [ "$#" -gt 0 ]; do
  case "$1" in
    --repo)
      require_value "$@"
      repo="$2"
      shift 2
      ;;
    --path)
      require_value "$@"
      remote_path="$2"
      shift 2
      ;;
    --file)
      require_value "$@"
      local_file="$2"
      shift 2
      ;;
    --message)
      require_value "$@"
      message="$2"
      shift 2
      ;;
    --max-bytes)
      require_value "$@"
      max_bytes="$2"
      shift 2
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *) die "unknown argument: $1" ;;
  esac
done

[ -n "$repo" ] || die "--repo is required"
[ -n "$remote_path" ] || die "--path is required"
[ -n "$local_file" ] || die "--file is required"
[ -n "$message" ] || die "--message is required"
[ -f "$local_file" ] || die "local file does not exist: $local_file"
[[ "$max_bytes" =~ ^[0-9]+$ ]] || die "--max-bytes must be numeric"

size=$(wc -c <"$local_file" | tr -d ' ')
if [ "$size" -gt "$max_bytes" ]; then
  die "file exceeds max-bytes ($size > $max_bytes); refusing oversized Contents API payload"
fi

command -v gh >/dev/null 2>&1 || die "gh is required"
command -v jq >/dev/null 2>&1 || die "jq is required"
command -v base64 >/dev/null 2>&1 || die "base64 is required"

# Existing SHA for update-in-place; empty means create.
existing_sha=""
if existing_sha=$(gh api "repos/${repo}/contents/${remote_path}" --jq '.sha' 2>&1); then
  :
else
  lookup_error="$existing_sha"
  not_found_pattern='HTTP[[:space:]]+404([^0-9]|$)'
  if [[ "$lookup_error" =~ $not_found_pattern ]]; then
    existing_sha=""
  else
    printf '%s\n' "$lookup_error" >&2
    die "failed to look up ${repo}:${remote_path}; refusing to create"
  fi
fi

payload="$(mktemp)"
content_b64_file="$(mktemp)"
trap 'rm -f "$payload" "$content_b64_file"' EXIT

# Encode via a temp file so neither base64 nor jq put the body on argv
# (ACKNOWLEDGEMENTS.md can exceed ARG_MAX when passed as --arg).
if base64 --help 2>&1 | grep -q -- '-w'; then
  base64 -w 0 "$local_file" >"$content_b64_file"
else
  base64 <"$local_file" | tr -d '\n' >"$content_b64_file"
fi

# --rawfile keeps content out of argv; --input below keeps it out of gh argv.
jq -n \
  --arg message "$message" \
  --rawfile content "$content_b64_file" \
  '{message: $message, content: ($content | gsub("\n"; ""))}' >"$payload"

if [ -n "$existing_sha" ]; then
  jq --arg sha "$existing_sha" '. + {sha: $sha}' "$payload" >"${payload}.tmp"
  mv "${payload}.tmp" "$payload"
fi

gh api --method PUT "repos/${repo}/contents/${remote_path}" --input "$payload" >/dev/null
echo "publish-public-contents: ok ${repo}:${remote_path}"
