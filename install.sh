#!/bin/sh
# Anvil CLI installer
# Downloads the pre-built native binary for your platform.
#
# Usage:
#   curl --proto '=https' --tlsv1.2 -LsSf https://github.com/EddaCraft/anvil/releases/latest/download/anvil-cli-installer.sh | sh
#
# For Windows (PowerShell):
#   irm https://github.com/EddaCraft/anvil/releases/latest/download/anvil-cli-installer.ps1 | iex
#
# This script fetches and runs the cargo-dist generated installer from the
# latest release on the public EddaCraft/anvil repository.

set -e

INSTALLER_URL="https://github.com/EddaCraft/anvil/releases/latest/download/anvil-cli-installer.sh"

echo ""
echo "  Anvil CLI Installer"
echo "  ==================="
echo ""

if ! command -v curl >/dev/null 2>&1; then
  echo "[!] curl is required. Install curl and try again." >&2
  exit 1
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

sh "$TMPFILE"
