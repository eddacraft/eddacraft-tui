#!/usr/bin/env bash
# Migration entrypoint. The rolling candidate is now the explicit anvil-main
# channel. Legacy options are intentionally rejected by promote-main.sh.
set -euo pipefail

printf 'run-candidate.sh is deprecated; using promote-main.sh\n' >&2
exec "$(dirname "$0")/promote-main.sh" "$@"
