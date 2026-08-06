#!/bin/sh
# Anvil CLI installer
# Downloads the pre-built native binary for your platform.
#
# Usage:
#   curl --proto '=https' --tlsv1.2 -LsSf https://github.com/eddacraft/anvil/releases/latest/download/eddacraft-anvil-installer.sh | sh
#
# For Windows (PowerShell):
#   irm https://github.com/eddacraft/anvil/releases/latest/download/eddacraft-anvil-installer.ps1 | iex
#
# This script fetches and runs the cargo-dist generated installer from the
# latest release on the public eddacraft/anvil repository.

set -e

# Colour support — disabled when NO_COLOR is set, stdout is not a tty, or TERM is dumb.
setup_colours() {
  EMBER="" BOLD="" DIM="" RESET=""
  if [ -n "${NO_COLOR:-}" ]; then return; fi
  if [ "${TERM:-dumb}" = "dumb" ]; then return; fi
  if ! [ -t 1 ]; then return; fi
  EMBER='\033[38;2;204;85;0m'
  BOLD='\033[1m'
  DIM='\033[2m'
  RESET='\033[0m'
}
setup_colours

INSTALLER_URL="https://github.com/eddacraft/anvil/releases/latest/download/eddacraft-anvil-installer.sh"

homebrew_prefixes() {
  if [ -n "${HOMEBREW_PREFIX:-}" ]; then
    printf '%s\n' "$HOMEBREW_PREFIX"
  fi
  printf '%s\n' /opt/homebrew /usr/local /home/linuxbrew/.linuxbrew
}

is_homebrew_anvil_path() {
  case "$1" in
    */Cellar/anvil/* | */Cellar/eddacraft-anvil/*) return 0 ;;
  esac

  while IFS= read -r prefix; do
    [ -n "$prefix" ] || continue
    case "$1" in
      "$prefix"/bin/anvil | "$prefix"/Cellar/anvil/* | "$prefix"/Cellar/eddacraft-anvil/*) return 0 ;;
    esac
  done <<EOF
$(homebrew_prefixes)
EOF

  return 1
}

detect_existing_homebrew_anvil() {
  if ! command -v anvil >/dev/null 2>&1; then
    return 1
  fi

  anvil_path=$(command -v anvil)
  if is_homebrew_anvil_path "$anvil_path"; then
    printf '%s\n' "$anvil_path"
    return 0
  fi

  if [ -L "$anvil_path" ]; then
    link_target=$(readlink "$anvil_path" 2>/dev/null || true)
    if [ -n "$link_target" ]; then
      case "$link_target" in
        /*) resolved_target=$link_target ;;
        *) resolved_target=$(dirname "$anvil_path")/$link_target ;;
      esac
      if is_homebrew_anvil_path "$resolved_target"; then
        printf '%s\n' "$anvil_path"
        return 0
      fi
    fi
  fi

  return 1
}

echo ""
echo "  Anvil CLI Installer"
echo "  ==================="
echo ""

if ! command -v curl >/dev/null 2>&1; then
  echo "[!] curl is required. Install curl and try again." >&2
  exit 1
fi

if HOMEBREW_ANVIL_PATH=$(detect_existing_homebrew_anvil); then
  echo "[!] Anvil is already installed via Homebrew at $HOMEBREW_ANVIL_PATH."
  echo "    Use Homebrew to update it instead:"
  echo ""
  echo "      brew upgrade eddacraft/tap/anvil"
  echo ""
  echo "    To switch to the standalone installer, uninstall the Homebrew formula first."
  exit 0
fi

if TMPFILE=$(mktemp -t anvil-installer 2>/dev/null); then
  :
elif TMPFILE=$(mktemp "${TMPDIR:-/tmp}/anvil-installer.XXXXXX" 2>/dev/null); then
  :
else
  echo "[!] Failed to create temporary file." >&2
  exit 1
fi
trap 'rm -f "$TMPFILE"' EXIT

if ! curl --proto '=https' --tlsv1.2 -LsSf "$INSTALLER_URL" -o "$TMPFILE"; then
  echo "[!] Failed to download installer." >&2
  exit 1
fi

# Run the cargo-dist installer; capture its exit code so we can print follow-up
# guidance before exiting on failure.
set +e
sh "$TMPFILE"
INSTALL_EXIT=$?
set -e

if [ "$INSTALL_EXIT" -ne 0 ]; then
  echo ""
  echo "  [!] Installer exited with code $INSTALL_EXIT."
  echo "  If the install failed, try Homebrew instead:"
  echo ""
  echo "    brew install eddacraft/tap/anvil"
  echo ""
  exit "$INSTALL_EXIT"
fi

# Detect installed version — may fail if PATH not yet updated in this shell.
ANVIL_VERSION=""
if command -v anvil >/dev/null 2>&1; then
  ANVIL_VERSION=$(anvil --version 2>/dev/null | head -1 | sed 's/^[^0-9]*//')
fi
if [ -n "$ANVIL_VERSION" ]; then
  VERSION_LINE="  anvil v${ANVIL_VERSION} installed successfully!"
else
  VERSION_LINE="  anvil installed successfully!"
fi

printf "\n"
printf "  ${EMBER}████         ████${RESET}\n"
printf "  ${EMBER}██             ██${RESET}\n"
printf "  ${EMBER}██  █████████  ██${RESET}\n"
printf "  ${EMBER}██     ███     ██${RESET}   ${EMBER}${BOLD}a n v i l${RESET}\n"
printf "  ${EMBER}██  █████████  ██${RESET}\n"
printf "  ${EMBER}██             ██${RESET}\n"
printf "  ${EMBER}████         ████${RESET}\n"
printf "\n"
printf "  ${DIM}Structural governance for AI-assisted development${RESET}\n"
printf "\n"
printf "%s\n" "$VERSION_LINE"
printf "\n"
# CIB-288: `anvil welcome` leads because it is the ungated demo surface
# (ADR-080) — `start` is in CLI_GATED_COMMANDS, so pointing a brand-new,
# unauthenticated reader at it first dead-ends at the auth wall. The `start`
# gloss describes the activation it performs, matching the CIB-260 wording in
# `welcome.rs`; a bare `anvil start` does not attach save-time coverage, so
# the banner no longer promises it.
printf "  Get started:\n"
printf "    cd your-project/\n"
printf "    anvil welcome    ${DIM}see what Anvil finds in your repo${RESET}\n"
printf "    anvil start      ${DIM}activate this repo (sign-in required)${RESET}\n"
printf "\n"
printf "  Or run anvil --help for all commands.\n"
printf "  ${DIM}https://docs.eddacraft.ai${RESET}\n"
printf "\n"
printf "                        ${DIM}[ ■ ] e d d a c r a f t${RESET}\n"
printf "\n"
