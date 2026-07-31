#Requires -Version 5.1
<#
.SYNOPSIS
  Refuse a cargo-dist PowerShell install when WinGet or Scoop already owns anvil.

.DESCRIPTION
  GH #2885: installing via the cargo-dist PowerShell installer after
  `winget install eddacraft.anvil` (or Scoop) leaves two binaries on PATH.
  Call this at the top of eddacraft-anvil-installer.ps1 (injected at release
  publish time) so users are directed back to the package manager.

  Exit codes:
    0 — no package-manager install detected; installer may continue
    2 — package-manager install detected; refuse dual install
#>
[CmdletBinding()]
param(
    [switch]$Force
)

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

$commands = Get-AnvilCommands
$wingetHits = @($commands | Where-Object { Test-IsWingetPath $_.Source })
$scoopHits = @($commands | Where-Object { Test-IsScoopPath $_.Source })

if ($wingetHits.Count -eq 0 -and $scoopHits.Count -eq 0) {
    exit 0
}

if ($Force) {
    Write-Warning "Package-manager anvil install detected, but -Force was passed; continuing cargo-dist install."
    exit 0
}

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
Write-Host "To force a standalone cargo-dist install anyway, re-run with -Force (not recommended)."
Write-Host ""
exit 2
