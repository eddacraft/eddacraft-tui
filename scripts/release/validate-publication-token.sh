#!/usr/bin/env bash

# Fail a release *before* it starts when the publication credential cannot
# publish.
#
# The v0.9.0-beta cut stalled ~6h on an expired `ANVIL_RELEASES_TOKEN`. Nothing
# checked it, and the failure surfaced at the cross-repo publish step — after
# prep, version bumps, tagging, and artefact builds had all run. PR #3309
# hardened what happens *after* a publication failure; this closes the other
# half by refusing to start.
#
# The credential is load-bearing in a way that hides its own absence:
# `.github/workflows/release.yml` resolves it as
# `secrets.ANVIL_RELEASES_TOKEN || secrets.GITHUB_TOKEN`. An unset secret
# silently falls back to `GITHUB_TOKEN`, which has no cross-repo write — so the
# run looks healthy until it tries to publish to the public repo or the tap.
#
# Never prints the token. Failures name the check, not the value.

set -euo pipefail

DEFAULT_PUBLIC_REPO="eddacraft/anvil"
DEFAULT_TAP_REPO="eddacraft/homebrew-tap"
DEFAULT_MIN_DAYS=14

usage() {
  cat <<'USAGE'
Usage: validate-publication-token.sh [options]

Validates that the release publication credential in ANVIL_RELEASES_TOKEN can
actually publish, before a cut begins.

Options:
  --public-repo <owner/name>  Public release repo   (default eddacraft/anvil)
  --tap-repo <owner/name>     Homebrew tap repo     (default eddacraft/homebrew-tap)
  --min-days <n>              Fail if the token expires within n days (default 14)
  --json                      Emit a machine-readable result
  -h, --help                  Show this help

Exit codes:
  0  the credential is present, valid, permitted, and not expiring soon
  1  usage error
  2  the credential cannot publish (absent, invalid, unpermitted, or expiring)
USAGE
}

die() {
  echo "validate-publication-token: $*" >&2
  exit 1
}

fail() {
  echo "validate-publication-token: $*" >&2
  exit 2
}

# --- pure decision logic (sourced by the tests) -----------------------------

# Parse an instant to a Unix epoch, GNU or BSD.
#
# GitHub sends the expiry as `2026-08-30 00:00:00 UTC`; callers may also pass
# ISO-8601. Normalise both to `YYYY-MM-DD HH:MM:SS`, then try GNU `date -d`
# and fall back to BSD `date -j -f` — the same fallback shape the repo already
# uses for `stat -c` / `stat -f`. Operators run this locally on macOS.
to_epoch() {
  local normalised="$1"
  normalised="${normalised% UTC}"
  normalised="${normalised/T/ }"
  normalised="${normalised%Z}"
  date -u -d "${normalised} UTC" +%s 2>/dev/null && return 0
  date -u -j -f '%Y-%m-%d %H:%M:%S' "$normalised" +%s 2>/dev/null && return 0
  return 1
}

# Whole days between two instants, truncated toward zero. Prints the count.
days_between() {
  local from_s until_s
  from_s="$(to_epoch "$1")" || return 1
  until_s="$(to_epoch "$2")" || return 1
  echo $(((until_s - from_s) / 86400))
}

# Decide on an expiry header. Args: <expiry-header> <now-iso> <min-days>
# Echoes "ok <days>", "expiring <days>", "expired <days>", or "unknown".
#
# GitHub omits the header for credentials that cannot expire (GitHub App
# installation tokens, classic PATs with no expiry). That is not a failure —
# it is the good case — so it reports `unknown` and the caller does not block.
classify_expiry() {
  local expiry="$1" now="$2" min_days="$3" days
  if [ -z "$expiry" ]; then
    echo "unknown"
    return 0
  fi
  if ! days="$(days_between "$now" "$expiry")"; then
    echo "unknown"
    return 0
  fi
  if [ "$days" -lt 0 ]; then
    echo "expired $days"
  elif [ "$days" -lt "$min_days" ]; then
    echo "expiring $days"
  else
    echo "ok $days"
  fi
}

# --- main ------------------------------------------------------------------

main() {
  local public_repo="$DEFAULT_PUBLIC_REPO"
  local tap_repo="$DEFAULT_TAP_REPO"
  local min_days="$DEFAULT_MIN_DAYS"
  local json=false

  while [ "$#" -gt 0 ]; do
    case "$1" in
      --public-repo) public_repo="${2:-}"; [ -n "$public_repo" ] || die "--public-repo requires a value"; shift 2 ;;
      --tap-repo) tap_repo="${2:-}"; [ -n "$tap_repo" ] || die "--tap-repo requires a value"; shift 2 ;;
      --min-days) min_days="${2:-}"; [ -n "$min_days" ] || die "--min-days requires a value"; shift 2 ;;
      --json) json=true; shift ;;
      -h | --help) usage; exit 0 ;;
      *) die "unknown argument $1" ;;
    esac
  done

  case "$min_days" in
    '' | *[!0-9]*) die "--min-days must be a non-negative integer" ;;
  esac

  local token="${ANVIL_RELEASES_TOKEN:-}"
  if [ -z "$token" ]; then
    fail "ANVIL_RELEASES_TOKEN is empty or unset.
  release.yml falls back to GITHUB_TOKEN, which cannot write to ${public_repo}
  or ${tap_repo}, so the run would fail at the cross-repo publish step rather
  than here. Set the secret before cutting — see
  docs/runbooks/release-token-scope.md."
  fi

  local api="${GITHUB_API_URL:-https://api.github.com}"
  local headers status expiry
  headers="$(mktemp)"
  # shellcheck disable=SC2064
  trap "rm -f '$headers'" EXIT

  # `/rate_limit`, not `/user`: a GitHub App installation token does not
  # authenticate as a user and would 403 on `/user`, wrongly blocking a
  # perfectly good credential. `/rate_limit` accepts both kinds and carries the
  # same `github-authentication-token-expiration` header.
  status="$(curl -sS -o /dev/null -D "$headers" -w '%{http_code}' \
    -H "Authorization: Bearer ${token}" \
    -H 'Accept: application/vnd.github+json' \
    "${api}/rate_limit" || echo '000')"

  if [ "$status" = "401" ]; then
    fail "the credential was rejected (HTTP 401) — expired or revoked.
  Rotate ANVIL_RELEASES_TOKEN before cutting; see docs/runbooks/release-token-scope.md
  for the required scopes and docs/runbooks/secret-rotation.md for the schedule."
  fi
  if [ "$status" != "200" ]; then
    fail "could not verify the credential (HTTP ${status} from ${api}/rate_limit)."
  fi

  # Header name is case-insensitive on the wire; normalise before matching.
  expiry="$(tr -d '\r' <"$headers" \
    | awk 'BEGIN{IGNORECASE=1} /^github-authentication-token-expiration:/ {sub(/^[^:]*:[[:space:]]*/,""); print; exit}')"

  local now verdict state days
  now="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  verdict="$(classify_expiry "$expiry" "$now" "$min_days")"
  state="${verdict%% *}"
  days="${verdict#* }"
  [ "$state" = "unknown" ] && days=""

  case "$state" in
    expired) fail "the credential expired ${days#-} day(s) ago (${expiry}). Rotate it before cutting." ;;
    expiring) fail "the credential expires in ${days} day(s) (${expiry}), inside the ${min_days}-day margin.
  Rotate it now rather than mid-cut — a v0.9-style stall costs hours at the worst moment." ;;
  esac

  # Presence and validity are not enough: the token must be able to write to
  # the repos the publish steps target. `permissions.push` is what a release
  # upload needs.
  local repo
  for repo in "$public_repo" "$tap_repo"; do
    local push
    push="$(curl -sS -H "Authorization: Bearer ${token}" \
      -H 'Accept: application/vnd.github+json' \
      "${api}/repos/${repo}" 2>/dev/null \
      | tr -d ' \n' | grep -o '"push":\(true\|false\)' | head -1 | cut -d: -f2 || true)"
    case "$push" in
      true) ;;
      false) fail "the credential has no write access to ${repo}; publication would fail at the cross-repo step." ;;
      *) fail "could not read permissions for ${repo} — the credential may lack visibility of it." ;;
    esac
  done

  if [ "$json" = true ]; then
    printf '{"credential":"ok","expiry":"%s","daysRemaining":"%s","minDays":%s,"repos":["%s","%s"]}\n' \
      "$expiry" "$days" "$min_days" "$public_repo" "$tap_repo"
  else
    if [ "$state" = "unknown" ]; then
      echo "validate-publication-token: ok — credential valid and permitted for ${public_repo} and ${tap_repo}; no expiry advertised."
    else
      echo "validate-publication-token: ok — credential valid and permitted for ${public_repo} and ${tap_repo}; expires in ${days} day(s)."
    fi
  fi
}

# Sourced by the tests to exercise the decision logic without network access;
# executed directly in CI and by operators. Same guard as preflight.sh.
if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
  main "$@"
fi
