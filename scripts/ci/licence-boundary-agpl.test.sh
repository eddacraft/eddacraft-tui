#!/usr/bin/env bash
# SDT-003 / ADR-136: the copyleft boundary must be enforced, not remembered.
#
# Anvil ships a commercial single binary. A copyleft dependency — AGPL above
# all, because TruffleHog is the tempting secret-detection engine and it is
# AGPL-3.0 — would contaminate it. ADR-136 records that the boundary is held
# by `attribution/deny.toml`'s allow-list rather than by a blocklist, and this
# file is the part that makes that claim falsifiable.
#
# Why a test rather than a `deny = [...]` entry naming AGPL:
#
#   Under cargo-deny `[licenses] version = 2` the `deny` key has been REMOVED
#   (upstream PR 611). It is not redundant-but-harmless — cargo-deny refuses to
#   validate a config that carries it and exits before checking anything, so
#   "adding AGPL to the deny list" would silently switch the licence gate OFF.
#   Probe 6 exists to stop a future reader from making that exact repair.
#
#   What actually holds the line is deny-by-omission: `version = 2` means every
#   licence not in `allow` is rejected. That is strictly stronger than a
#   blocklist — it catches copyleft nobody thought to enumerate — but it is
#   invisible, because it is a property of an absence. These probes convert it
#   into something that fails out loud when it stops being true.
#
# The realistic way to lose the boundary is not deleting a line; it is adding
# "AGPL-3.0-only" to `attribution/licences.toml`, whose expander regenerates
# the allow array. Probe 1 goes red on exactly that edit.
#
# Exit codes: 0 all probes passed; 1 a probe failed; 2 prerequisites missing.

set -euo pipefail

repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
deny_config="${repo_root}/attribution/deny.toml"

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

if [ ! -f "${deny_config}" ]; then
  echo "error: ${deny_config} not found" >&2
  exit 2
fi

# Deliberately a hard error, not a skip. A silent pass on a machine without
# cargo-deny is the fail-open shape this repository has been bitten by before:
# the suite would report green while asserting nothing at all.
if ! command -v cargo-deny >/dev/null 2>&1; then
  echo "error: cargo-deny is not on PATH; this test cannot assert anything without it." >&2
  echo "  install: cargo install --locked cargo-deny" >&2
  echo "  (CI installs it in the rust.yml 'cargo-deny' job, where this test runs)" >&2
  exit 2
fi

tmp_root=$(mktemp -d)
cleanup() { rm -rf "${tmp_root}"; }
trap cleanup EXIT

# A two-crate fixture whose ONLY variable is the dependency's `license` field.
# Holding everything else fixed is what makes the control meaningful: probe 5
# runs this same tree with MIT and must pass, so a red probe can only be caused
# by the licence, never by fixture noise or an unrelated config error.
#
# Path dependency only, so `cargo metadata` resolves with --offline and the
# probes need no network and no registry.
build_fixture() {
  local licence="$1" dir="${tmp_root}/fixture"
  rm -rf "${dir}"
  mkdir -p "${dir}/src" "${dir}/copyleft-dep/src"

  cat >"${dir}/Cargo.toml" <<'EOF'
[package]
name = "licence-boundary-fixture"
version = "0.0.0"
edition = "2021"
license = "MIT"
publish = false

[dependencies]
copyleft-dep = { path = "copyleft-dep" }

[workspace]
EOF
  : >"${dir}/src/lib.rs"

  cat >"${dir}/copyleft-dep/Cargo.toml" <<EOF
[package]
name = "copyleft-dep"
version = "0.0.0"
edition = "2021"
license = "${licence}"
publish = false
EOF
  : >"${dir}/copyleft-dep/src/lib.rs"

  printf '%s' "${dir}"
}

# Run the repository's REAL deny.toml against the fixture. Using the real
# config is the whole point: a copied or simplified config would test a fiction
# and stay green while the shipped policy rotted.
run_deny() {
  local dir="$1"
  (
    cd "${dir}"
    cargo deny --offline check --config "${deny_config}" licenses >"${tmp_root}/out.txt" 2>&1
  )
}

expect_rejected() {
  local licence="$1" dir status
  dir=$(build_fixture "${licence}")
  status=0
  run_deny "${dir}" || status=$?

  if [ "${status}" -eq 0 ]; then
    echo "--- cargo-deny output ---" >&2
    cat "${tmp_root}/out.txt" >&2
    fail "a '${licence}' dependency was ACCEPTED by attribution/deny.toml." \
      $'\n  The copyleft boundary ADR-136 depends on is gone. Most likely cause:' \
      $'\n  the licence was added to attribution/licences.toml, whose expander' \
      $'\n  regenerates deny.toml'"'"$'s allow array.'
  fi

  # Distinguish "rejected the licence" from "could not read the config".
  # Without this, corrupting deny.toml would make every rejection probe pass
  # for the worst possible reason — the gate failing to start.
  if ! grep -q 'failed to satisfy license requirements' "${tmp_root}/out.txt"; then
    echo "--- cargo-deny output ---" >&2
    cat "${tmp_root}/out.txt" >&2
    fail "'${licence}' produced exit ${status}, but not a licence rejection." \
      $'\n  cargo-deny failed for some other reason (likely an invalid config),' \
      $'\n  which would mean the licence gate is not running at all.'
  fi

  echo "ok: ${licence} rejected by attribution/deny.toml"
}

# --- Probes 1-3: the AGPL identifiers, current and deprecated ----------------
#
# All three spellings must be rejected. TruffleHog's own manifest declares the
# deprecated bare `AGPL-3.0`, so testing only the modern `-only` / `-or-later`
# forms would miss the single most likely real-world import.

expect_rejected "AGPL-3.0-only"
expect_rejected "AGPL-3.0-or-later"
expect_rejected "AGPL-3.0"

# --- Probe 4: the boundary is an allow-list, not an AGPL blocklist -----------
#
# GPL is not named anywhere in the config. It is rejected purely because it was
# never allowed. If this probe ever needs a config change to stay green, the
# posture has silently become enumeration.

expect_rejected "GPL-3.0-only"

# --- Probe 5: vacuity control ------------------------------------------------
#
# The same fixture with an allowed licence MUST pass. Without this, probes 1-4
# would still be green if the fixture were malformed, the config unreadable, or
# cargo-deny rejecting everything unconditionally.

control_dir=$(build_fixture "MIT")
control_status=0
run_deny "${control_dir}" || control_status=$?
if [ "${control_status}" -ne 0 ]; then
  echo "--- cargo-deny output ---" >&2
  cat "${tmp_root}/out.txt" >&2
  fail "the MIT control fixture was REJECTED (exit ${control_status})." \
    $'\n  The rejection probes above therefore prove nothing: this fixture' \
    $'\n  fails regardless of the licence under test.'
fi
echo "ok: MIT control accepted (rejections above are caused by the licence)"

# --- Probe 6: no [licenses].deny array ---------------------------------------
#
# The trap this test exists to prevent. `deny` is a removed key under
# version = 2: cargo-deny reports `error[deprecated]: this key has been
# removed` and refuses the config, so a well-meant "make the AGPL boundary
# explicit" commit turns the licence gate off entirely while looking like it
# strengthened it.
#
# Scoped to the [licenses] table on purpose — [bans].deny is a legitimate key
# this repository may use, and a bare grep would forbid it.
if awk '
  /^[[:space:]]*\[/ { section = $0 }
  section ~ /^\[licenses\]/ && /^[[:space:]]*deny[[:space:]]*=/ { found = 1 }
  END { exit(found ? 0 : 1) }
' "${deny_config}"; then
  fail "attribution/deny.toml declares a [licenses].deny array." \
    $'\n  That key was REMOVED in cargo-deny v2. Its presence makes cargo-deny' \
    $'\n  reject the whole config, so the licence gate stops running and every' \
    $'\n  licence passes unchecked. Delete it: under version = 2 anything absent' \
    $'\n  from `allow` is already denied. See ADR-136.'
fi
echo "ok: no [licenses].deny array (the gate-disabling trap is absent)"

echo
echo "PASS: licence boundary holds — AGPL/GPL rejected, allowed licences unaffected."
