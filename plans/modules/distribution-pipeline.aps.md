<!--
APS Module: Distribution Pipeline
===================================
Public repo, release binaries, install script, DNS, Homebrew, WinGet.
Replaces RCLI-024 with a full module.

Scopes: DIST (main)
-->

# Distribution Pipeline

| ID   | Owner | Status |
| ---- | ----- | ------ |
| DIST | —     | Ready  |

## Purpose

Ship the `anvil` binary to users via a public repo (`eddacraft/anvil`),
GitHub Releases, an install script at `install.eddacraft.ai`, a Homebrew
tap, and a WinGet manifest.

**Why:** The Rust CLI is feature-complete for beta but users can't install it.
The binary is built in the private monorepo (`anvil-001`) and needs a public
distribution surface with platform binaries, an install script, and DNS
routing.

**IP model (ADR-018):** Anvil is closed-source / free-at-base-tier. The
source never leaves the private monorepo. Distribution is binary-only. This
rules out crates.io as an install path (publishing requires source
disclosure) and shapes the channel mix below toward manifest-based package
managers (Homebrew, WinGet, scoop) that point at GitHub Release binaries.

## In Scope

- Create `eddacraft/anvil` public repo (release binaries + install docs)
- GitHub Actions release workflow in `anvil-001` that cross-compiles and
  pushes binaries to `eddacraft/anvil` Releases
- Shell install script (Linux/macOS) and PowerShell install script (Windows)
- GitHub Pages on `eddacraft/anvil` serving install scripts
- Azure DNS: CNAME `install.eddacraft.ai` → `eddacraft.github.io`
- Pulumi resource for the DNS record
- Homebrew tap (`eddacraft/homebrew-tap`) for macOS users
- WinGet manifest (`microsoft/winget-pkgs`) for Windows users
- Optional scoop bucket for Windows developer audience

## Out of Scope

- The Rust CLI itself (see RCLI module)
- Website changes beyond updating install commands (see beta housekeeping)
- Nightly builds or pre-release channels
- Auto-update mechanism
- **crates.io publish** — incompatible with the closed-source IP model
  (see ADR-018 and DIST-008 below)

## Interfaces

**Depends on:**

- `anvil-001` CI — source of compiled binaries
- Azure DNS — `eddacraft.ai` zone (managed by IAC module)
- GitHub — repo creation, Pages, Releases, Actions

**Exposes:**

- `install.eddacraft.ai` — install script endpoint
- `eddacraft/anvil` — GitHub Releases with platform binaries
- `eddacraft/homebrew-tap` — Homebrew formula
- `microsoft/winget-pkgs` — WinGet manifest

## Constraints

- Release workflow must be triggered from `anvil-001` (source of truth) and
  push artefacts to the public repo via a deploy key or PAT
- Install script must detect platform (Linux x86_64/aarch64, macOS
  x86_64/aarch64, Windows x86_64) and download the correct binary
- DNS propagation may take up to 48h for new CNAME records

## Ready Checklist

Change status to **Ready** when:

- [x] Purpose and scope are clear
- [x] Dependencies identified
- [x] Public repo layout agreed (anvil = binaries, eddacraft-tui = library)
- [x] DNS zone is managed by Pulumi (IAC module complete)
- [x] Install URL agreed (`install.eddacraft.ai`)

---

## Phase 1 — Public Repos

### DIST-001: Create eddacraft/anvil public repo

- **Status:** Ready
- **Intent:** Create the public-facing repo that hosts release binaries,
  install docs, and the README that users see when they find Anvil
- **Expected Outcome:** `github.com/eddacraft/anvil` exists with README,
  LICENSE, and placeholder for GitHub Pages
- **Validation:** Repo is publicly accessible; README describes what Anvil is
  and how to install
- **Files:** `eddacraft/anvil/README.md`, `LICENSE`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** None

---

## Phase 2 — Install Script + DNS

> **Note:** `eddacraft/eddacraft-tui` extraction is tracked separately in
> the TUIEXTRACT module (3/7 done).

### DIST-003: Write install.sh (Linux/macOS)

- **Status:** Ready
- **Intent:** Shell script that detects OS and architecture, downloads the
  correct binary from GitHub Releases, and installs it to `~/.eddacraft/bin`
  (or `/usr/local/bin` with sudo)
- **Expected Outcome:** `curl -fsSL https://install.eddacraft.ai | sh`
  installs the latest `anvil` binary and prints a success message with
  PATH instructions
- **Validation:** Script works on Ubuntu x86_64, Ubuntu aarch64, macOS
  x86_64, macOS aarch64 (test in CI matrix)
- **Files:** `eddacraft/anvil/install.sh`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** DIST-001

---

### DIST-004: Write install.ps1 (Windows)

- **Status:** Ready
- **Intent:** PowerShell script that downloads the Windows x86_64 binary
  from GitHub Releases and installs it to `%LOCALAPPDATA%\eddacraft\bin`
- **Expected Outcome:** `irm https://install.eddacraft.ai/windows | iex`
  installs the latest `anvil.exe`
- **Validation:** Script works on Windows x86_64 (test in CI)
- **Files:** `eddacraft/anvil/install.ps1`
- **Confidence:** medium
- **Priority:** Medium
- **Dependencies:** DIST-001

---

### DIST-005: Configure GitHub Pages on eddacraft/anvil

- **Status:** Ready
- **Intent:** Serve install scripts via GitHub Pages so
  `install.eddacraft.ai` resolves to the shell script and
  `install.eddacraft.ai/windows` resolves to the PowerShell script
- **Expected Outcome:** GitHub Pages enabled on `eddacraft/anvil`, serving
  from root or `/docs` directory, with a CNAME file for custom domain
- **Validation:** `curl -fsSL https://eddacraft.github.io/anvil/install.sh`
  returns the install script
- **Files:** `eddacraft/anvil/CNAME`, `eddacraft/anvil/index.html` (redirect
  to install.sh or landing page)
- **Confidence:** high
- **Priority:** High
- **Dependencies:** DIST-001, DIST-003

---

### DIST-006: Azure DNS CNAME for install.eddacraft.ai

- **Status:** Ready
- **Intent:** Add a CNAME record pointing `install.eddacraft.ai` to
  `eddacraft.github.io` so the install script is served from a branded URL
- **Expected Outcome:** `dig install.eddacraft.ai CNAME` returns
  `eddacraft.github.io`
- **Validation:** `curl -fsSL https://install.eddacraft.ai | head -1`
  returns the shebang line of install.sh
- **Files:** `infra/src/dns/eddacraft-ai.ts` (Pulumi resource)
- **Confidence:** high
- **Priority:** High
- **Dependencies:** DIST-005, IAC (DNS zone already managed)

---

## Phase 3 — Release Workflow

### DIST-007: Cross-compile release workflow

- **Status:** Ready
- **Intent:** GitHub Actions workflow in `anvil-001` that builds release
  binaries for all 5 targets (Linux x86_64/aarch64, macOS x86_64/aarch64,
  Windows x86_64) on tag push, then uploads them to `eddacraft/anvil`
  Releases via a deploy key
- **Expected Outcome:** Pushing a `v*` tag to `anvil-001` produces a GitHub
  Release on `eddacraft/anvil` with 5 platform binaries + checksums
- **Validation:** Tag `v0.3.0-beta`, verify all 5 binaries appear on the
  public release; checksums match local builds
- **Files:** `.github/workflows/release.yml` (update existing cargo-dist
  workflow to push to public repo)
- **Confidence:** medium (cross-repo release requires deploy key or PAT)
- **Priority:** High
- **Dependencies:** DIST-001

---

### DIST-008: Publish to crates.io

- **Status:** Deferred — incompatible with the closed-source IP model
  (see [ADR-018](../decisions/018-product-ip-architecture.md))
- **Original intent:** Publish the Anvil crate family to crates.io so
  users on platforms without our install script could run
  `cargo install anvil-cli`.
- **Why deferred:** Publishing to crates.io requires publishing source.
  ADR-018 establishes that the Anvil monorepo is closed-source (free at
  base tier, source proprietary), with a deliberate three-piece OSS
  surface (`eddacraft-tui`, `anvil-plan-spec`, `kindling`) that does
  *not* include the product code. `cargo install` is therefore not a
  viable install path for Anvil. The Windows-user gap this item was
  meant to fill is filled by **DIST-010 (WinGet)** instead, which
  points at the GitHub Release binary and requires zero source
  disclosure.
- **Namespace rename:** The `eddacraft-anvil-*` namespace prefix was
  analysed, approved, and applied to all publishable crates (ADR-017).
  Crates.io publication itself is deferred alongside this item. The
  naming analysis is captured in this section.
- **Re-activation criteria** — this item could come off the shelf if:
  - The IP model changes (e.g. a future ADR opens part of the product
    under a permissive licence), **and**
  - There is a real user request volume for `cargo install` that
    WinGet / scoop / Homebrew / install.sh together cannot satisfy
- **Files:** ADR-018 (IP model)
- **Priority:** Deferred
- **Dependencies:** n/a

---

### DIST-009: Homebrew tap

- **Status:** Ready
- **Intent:** Create `eddacraft/homebrew-tap` repo with a formula for
  `anvil` that downloads the macOS binary from GitHub Releases
- **Expected Outcome:** `brew install eddacraft/tap/anvil` installs the
  latest version
- **Validation:** `brew install eddacraft/tap/anvil && anvil --version`
- **Files:** `eddacraft/homebrew-tap/Formula/anvil.rb`
- **Confidence:** high
- **Priority:** Medium
- **Dependencies:** DIST-007

---

## Phase 4 — Windows package managers

> **Context:** DIST-008 (crates.io publish) was deferred per ADR-018
> because it requires source disclosure. WinGet and scoop fill the
> Windows install gap by pointing at the GitHub Release binary
> produced by DIST-007 — both are manifest-based and require zero
> source disclosure.

### DIST-010: WinGet manifest

- **Status:** Ready
- **Intent:** Submit a WinGet manifest to `microsoft/winget-pkgs` so
  Windows users can `winget install eddacraft.anvil`. WinGet ships
  preinstalled on Windows 11 and is the official Microsoft package
  manager. Used by `gh`, `gcloud`, `vercel`, `pulumi`, `bun`, and
  most other modern closed-source dev tools targeting Windows.
- **Expected Outcome:** `winget install eddacraft.anvil` installs a
  working `anvil.exe` on a clean Windows 11 machine.
- **Validation:** `winget install eddacraft.anvil && anvil --version`
  on a clean Windows VM.
- **Files:**
  - WinGet manifest YAML in
    `microsoft/winget-pkgs/manifests/e/eddacraft/Anvil/<version>/`
  - `anvil-001` release workflow extension to auto-generate and
    submit the manifest on each tagged release (use
    `vedantmgoyal2009/winget-releaser` or
    `microsoft/winget-create`)
- **Confidence:** high — well-trodden path, lots of prior art
- **Priority:** High (replaces DIST-008 for the Windows install gap)
- **Dependencies:** DIST-007 (release workflow producing the
  Windows cargo-dist artifacts `eddacraft-anvil-x86_64-pc-windows-msvc.zip`
  and `eddacraft-anvil-aarch64-pc-windows-msvc.zip`)

---

### DIST-011: Scoop bucket (optional)

- **Status:** Ready
- **Intent:** Create `eddacraft/scoop-bucket` repo with a manifest for
  `anvil` that downloads the Windows binary from GitHub Releases.
  Scoop is the popular community Windows package manager favoured
  by developer audiences (lighter weight than WinGet, single-line
  install).
- **Expected Outcome:** `scoop bucket add eddacraft https://github.com/eddacraft/scoop-bucket && scoop install anvil`
  installs the latest version.
- **Validation:** clean Windows VM with scoop pre-installed,
  `scoop install eddacraft/anvil && anvil --version`.
- **Files:** `eddacraft/scoop-bucket/bucket/anvil.json`
- **Confidence:** high
- **Priority:** Medium (WinGet is the primary Windows path; scoop is
  the developer-audience polish)
- **Dependencies:** DIST-007

---

## Risks

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| Cross-repo release auth (PAT/deploy key) | Medium | High | Use fine-grained PAT scoped to eddacraft/anvil repo |
| DNS propagation delay | Low | Low | Set up CNAME early; 48h buffer before beta |
| GitHub Pages HTTPS cert for custom domain | Low | Low | GitHub auto-provisions Let's Encrypt cert |
| Install script platform detection edge cases | Medium | Low | Test in CI matrix; fall back to manual download |
| WinGet manifest review delay | Medium | Low | Microsoft's review queue can take 1–7 days for new packages — submit early; subsequent updates are auto-merged |

## Stats

| Phase | Items | Status |
| ----- | ----- | ------ |
| 1 — Public Repo | 1 | Ready |
| 2 — Install Script + DNS | 4 | Ready |
| 3 — Release Workflow | 2 (DIST-007, DIST-009; DIST-008 deferred) | Ready |
| 4 — Windows Package Managers | 2 | Ready |
| **Total active** | **9** | **0/9 done** |
| Deferred | 1 (DIST-008) | per ADR-018 |
