#Requires -Version 5.1
# Anvil package-manager dual-install guard.
#
# This file ships inside the public PowerShell installer, comments and all,
# so it carries no internal tracker ids — see the guard test beside it.
#
# INJECT SHAPE (critical): this fragment is inserted into
# eddacraft-anvil-installer.ps1 *after* cargo-dist's top-level `param(...)`
# block (see .github/workflows/release.yml). Do not prepend a second `param`
# block, and never `exit 0` on the clean path — that terminated the whole
# installer before the cargo-dist body ran (v0.9.2-beta regression).
#
# Exit codes (whole installer process):
#   (fall-through) — no package-manager anvil; cargo-dist body continues
#   2              — WinGet/Scoop anvil on PATH; refuse dual install
#
# Force: if the host script bound a -Force switch (or ANVIL_INSTALL_FORCE=1),
# dual-install is allowed with a warning.

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Get-AnvilCommands {
    try {
        return @(Get-Command anvil -All -ErrorAction Stop)
    } catch {
        return @()
    }
}

function Test-IsWingetPath {
    param([string]$Path)
    if ([string]::IsNullOrWhiteSpace($Path)) { return $false }
    $normalised = $Path.Replace('/', '\')
    return $normalised -match '(?i)\\WindowsApps\\' `
        -or $normalised -match '(?i)\\Microsoft\\WinGet\\' `
        -or $normalised -match '(?i)\\Winget\\Packages\\'
}

function Test-IsScoopPath {
    param([string]$Path)
    if ([string]::IsNullOrWhiteSpace($Path)) { return $false }
    $normalised = $Path.Replace('/', '\')
    return $normalised -match '(?i)\\scoop\\shims\\' `
        -or $normalised -match '(?i)\\scoop\\apps\\anvil\\'
}

function Test-AnvilInstallForce {
    # Prefer a bound -Force from the host installer param block when present.
    if (Get-Variable -Name Force -Scope 0 -ErrorAction SilentlyContinue) {
        try {
            if ($Force) { return $true }
        } catch { }
    }
    if (Get-Variable -Name Force -Scope 1 -ErrorAction SilentlyContinue) {
        try {
            if ($Force) { return $true }
        } catch { }
    }
    if ($env:ANVIL_INSTALL_FORCE -eq '1') { return $true }
    return $false
}

$commands = Get-AnvilCommands
$wingetHits = @($commands | Where-Object { Test-IsWingetPath $_.Source })
$scoopHits = @($commands | Where-Object { Test-IsScoopPath $_.Source })

if ($wingetHits.Count -eq 0 -and $scoopHits.Count -eq 0) {
    # Clean PATH: fall through into the cargo-dist installer body.
    # Do not exit 0 here.
} elseif (Test-AnvilInstallForce) {
    Write-Warning "Package-manager anvil install detected, but Force was set; continuing cargo-dist install."
} else {
    Write-Host ""
    Write-Host "Anvil is already installed via a Windows package manager." -ForegroundColor Yellow
    Write-Host "Installing a second copy with this PowerShell installer would leave two binaries on PATH."
    Write-Host ""

    if ($wingetHits.Count -gt 0) {
        Write-Host "Detected WinGet install(s):"
        $wingetHits | ForEach-Object { Write-Host "  - $($_.Source)" }
        Write-Host ""
        Write-Host "Upgrade with:"
        Write-Host "  winget upgrade --id eddacraft.anvil"
    }

    if ($scoopHits.Count -gt 0) {
        Write-Host "Detected Scoop install(s):"
        $scoopHits | ForEach-Object { Write-Host "  - $($_.Source)" }
        Write-Host ""
        Write-Host "Upgrade with:"
        Write-Host "  scoop update anvil"
    }

    Write-Host ""
    Write-Host "To force a standalone cargo-dist install anyway, set ANVIL_INSTALL_FORCE=1 and re-run (not recommended)."
    Write-Host ""
    exit 2
}
