#!/usr/bin/env python3
"""Insert the Windows package-manager dual-install guard after cargo-dist's param block.

Prepending the guard (second param + exit 0) made clean `irm | iex` installs
a silent no-op. Inject *after* the first top-level `param (...)`.

The banner written here lands in the public installer, so it — like the guard
itself — carries no internal tracker ids.

Also initialise `$Args` when `powershell -File` left it unset. cargo-dist later
does `Install-Binary "$Args"` after `Set-StrictMode`; `irm | iex` already has
a caller `$args` and is left unchanged.
"""
from __future__ import annotations

import re
import sys
from pathlib import Path


def inject(ps1_path: Path, guard_path: Path) -> None:
    ps1 = ps1_path.read_text(encoding="utf-8")
    guard = guard_path.read_text(encoding="utf-8")
    match = re.search(r"(?ms)^(\s*param\s*\(.*?\))\s*\r?\n", ps1)
    if not match:
        raise SystemExit(
            f"cargo-dist PowerShell installer has no param block: {ps1_path}"
        )
    head, rest = match.group(0), ps1[match.end() :]
    out = (
        head
        + "\n"
        + "# powershell -File does not populate $Args after param;\n"
        + "# cargo-dist later reads it under Set-StrictMode.\n"
        + "if (-not (Get-Variable -Name Args -ErrorAction SilentlyContinue)) {\n"
        + "    $Args = @()\n"
        + "}\n"
        + "\n# --- begin anvil package-manager dual-install guard ---\n"
        + "# Injected after cargo-dist param (insert_after_param; not prepended).\n"
        + guard
        + "\n# --- end anvil package-manager dual-install guard ---\n\n"
        + rest
    )
    ps1_path.write_text(out, encoding="utf-8")
    print(f"Injected Windows package-manager guard after cargo-dist param into {ps1_path}")


def main() -> None:
    if len(sys.argv) != 3:
        raise SystemExit(f"usage: {sys.argv[0]} <installer.ps1> <guard.ps1>")
    inject(Path(sys.argv[1]), Path(sys.argv[2]))


if __name__ == "__main__":
    main()
