#!/usr/bin/env bash
# Air-gapped command harness (MLP-017).
#
# Runs `$@` in a child process with the network namespace stripped so
# DNS, TCP, UDP, ICMP, and raw-socket attempts fail at the OS layer
# rather than depending on the harness to recognise them. Linux only;
# the script exits 77 (skip) on platforms where the necessary
# primitive is missing.
#
# Usage:
#   tools/test-harness/network-blocked/run.sh <command> [args...]
#
# Exit codes:
#   0   command succeeded under the air-gapped sandbox
#   64  usage error (no command supplied)
#   77  sandbox primitive unavailable (treat as skip in CI)
#   *   propagated from the command under test
#
# The script picks the strongest available isolation primitive:
#
#   1. `unshare -n -r` — Linux user + network namespace, no sudo.
#      Available on most Linux distros from 2018 onward (util-linux
#      >= 2.32). This is the preferred path; the resulting child has
#      no `lo`, no routes, no DNS resolver path.
#
#   2. Fallback (none implemented yet) — macOS / BSD do not have an
#      equivalent unprivileged primitive. The test harness skips
#      with exit 77 on those platforms so CI doesn't flake.
#
# Reviewers: keep this script dependency-free so it can run on a bare
# CI image without `nix-shell` / `apt-get install` preamble.

set -euo pipefail

if [ $# -lt 1 ]; then
    echo "usage: $0 <command> [args...]" >&2
    exit 64
fi

# Detect the unshare primitive. The flags we need are -n (network
# namespace) and -r (map current uid to root inside the namespace so
# the child can configure its own lo). Both have been stable for
# years; we still check rather than assume.
if [ "$(uname -s)" = "Linux" ] && command -v unshare >/dev/null 2>&1; then
    # Probe quickly with a no-op invocation; if the kernel forbids
    # user namespaces (e.g. some Docker hosts), bail with the skip
    # code rather than failing. We probe with shell `true`, which is
    # a bash builtin and so always available — no `/usr/bin/env` or
    # any other on-disk binary required.
    if ! unshare -n -r true >/dev/null 2>&1; then
        echo "anvil-air-gapped: unshare -n -r refused by this kernel; skipping" >&2
        exit 77
    fi
    exec unshare -n -r "$@"
fi

echo "anvil-air-gapped: no network-namespace primitive available on this platform; skipping" >&2
exit 77
