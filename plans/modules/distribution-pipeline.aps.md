<!--
APS Module: Distribution Pipeline
===================================
Public repo, release binaries, install script, DNS, crates.io.
Replaces RCLI-024 with a full module.

Scopes: DIST (main)
-->

# Distribution Pipeline

| ID   | Owner | Status |
| ---- | ----- | ------ |
| DIST | —     | Ready  |

## Purpose

Ship the `anvil` binary to users via a public repo (`EddaCraft/anvil`),
GitHub Releases, an install script at `install.eddacraft.ai`, and crates.io.
Also extract `eddacraft-tui` to its own public repo for consumption by APS
and Kindling.

**Why:** The Rust CLI is feature-complete for beta but users can't install it.
The binary is built in the private monorepo (`anvil-001`) and needs a public
distribution surface with platform binaries, an install script, and DNS
routing.

## In Scope

- Create `EddaCraft/anvil` public repo (release binaries + install docs)
- GitHub Actions release workflow in `anvil-001` that cross-compiles and
  pushes binaries to `EddaCraft/anvil` Releases
- Shell install script (Linux/macOS) and PowerShell install script (Windows)
- GitHub Pages on `EddaCraft/anvil` serving install scripts
- Azure DNS: CNAME `install.eddacraft.ai` → `eddacraft.github.io`
- Pulumi resource for the DNS record
- crates.io publish for Windows users (`cargo install anvil-cli`)
- Homebrew tap (`eddacraft/homebrew-tap`) for macOS users

## Out of Scope

- The Rust CLI itself (see RCLI module)
- Website changes beyond updating install commands (see beta housekeeping)
- Nightly builds or pre-release channels
- Auto-update mechanism

## Interfaces

**Depends on:**

- `anvil-001` CI — source of compiled binaries
- Azure DNS — `eddacraft.ai` zone (managed by IAC module)
- GitHub — repo creation, Pages, Releases, Actions

**Exposes:**

- `install.eddacraft.ai` — install script endpoint
- `EddaCraft/anvil` — GitHub Releases with platform binaries
- `anvil-cli` on crates.io

## Constraints

- Release workflow must be triggered from `anvil-001` (source of truth) and
  push artefacts to the public repo via a deploy key or PAT
- Install script must detect platform (Linux x86_64/aarch64, macOS
  x86_64/aarch64, Windows x86_64) and download the correct binary
- crates.io publish requires the crate to build standalone — all workspace
  path deps must be vendored or published first
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

### DIST-001: Create EddaCraft/anvil public repo

- **Status:** Ready
- **Intent:** Create the public-facing repo that hosts release binaries,
  install docs, and the README that users see when they find Anvil
- **Expected Outcome:** `github.com/EddaCraft/anvil` exists with README,
  LICENSE, and placeholder for GitHub Pages
- **Validation:** Repo is publicly accessible; README describes what Anvil is
  and how to install
- **Files:** `EddaCraft/anvil/README.md`, `LICENSE`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** None

---

## Phase 2 — Install Script + DNS

> **Note:** `EddaCraft/eddacraft-tui` extraction is tracked separately in
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
- **Files:** `EddaCraft/anvil/install.sh`
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
- **Files:** `EddaCraft/anvil/install.ps1`
- **Confidence:** medium
- **Priority:** Medium
- **Dependencies:** DIST-001

---

### DIST-005: Configure GitHub Pages on EddaCraft/anvil

- **Status:** Ready
- **Intent:** Serve install scripts via GitHub Pages so
  `install.eddacraft.ai` resolves to the shell script and
  `install.eddacraft.ai/windows` resolves to the PowerShell script
- **Expected Outcome:** GitHub Pages enabled on `EddaCraft/anvil`, serving
  from root or `/docs` directory, with a CNAME file for custom domain
- **Validation:** `curl -fsSL https://eddacraft.github.io/anvil/install.sh`
  returns the install script
- **Files:** `EddaCraft/anvil/CNAME`, `EddaCraft/anvil/index.html` (redirect
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
  Windows x86_64) on tag push, then uploads them to `EddaCraft/anvil`
  Releases via a deploy key
- **Expected Outcome:** Pushing a `v*` tag to `anvil-001` produces a GitHub
  Release on `EddaCraft/anvil` with 5 platform binaries + checksums
- **Validation:** Tag `v0.3.0-beta`, verify all 5 binaries appear on the
  public release; checksums match local builds
- **Files:** `.github/workflows/release.yml` (update existing cargo-dist
  workflow to push to public repo)
- **Confidence:** medium (cross-repo release requires deploy key or PAT)
- **Priority:** High
- **Dependencies:** DIST-001

---

### DIST-008: Publish to crates.io

- **Status:** Ready
- **Intent:** Publish `anvil-cli` to crates.io so Windows users can
  `cargo install anvil-cli`. Requires vendoring or publishing workspace
  dependencies first
- **Expected Outcome:** `cargo install anvil-cli` installs a working `anvil`
  binary
- **Validation:** `cargo install anvil-cli && anvil --version` on a clean
  machine
- **Files:** `crates/anvil-cli/Cargo.toml` (metadata for crates.io)
- **Confidence:** medium (workspace deps need to be resolvable from
  crates.io — may need to publish anvil-kernel, anvil-tui, etc. or vendor)
- **Priority:** Medium
- **Dependencies:** DIST-007

---

### DIST-009: Homebrew tap

- **Status:** Ready
- **Intent:** Create `EddaCraft/homebrew-tap` repo with a formula for
  `anvil` that downloads the macOS binary from GitHub Releases
- **Expected Outcome:** `brew install eddacraft/tap/anvil` installs the
  latest version
- **Validation:** `brew install eddacraft/tap/anvil && anvil --version`
- **Files:** `EddaCraft/homebrew-tap/Formula/anvil.rb`
- **Confidence:** high
- **Priority:** Medium
- **Dependencies:** DIST-007

---

## Risks

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| Cross-repo release auth (PAT/deploy key) | Medium | High | Use fine-grained PAT scoped to EddaCraft/anvil repo |
| crates.io workspace dep resolution | Medium | Medium | Vendor deps or publish sub-crates first |
| DNS propagation delay | Low | Low | Set up CNAME early; 48h buffer before beta |
| GitHub Pages HTTPS cert for custom domain | Low | Low | GitHub auto-provisions Let's Encrypt cert |
| Install script platform detection edge cases | Medium | Low | Test in CI matrix; fall back to manual download |

## Stats

| Phase | Items | Status |
| ----- | ----- | ------ |
| 1 — Public Repo | 1 | Ready |
| 2 — Install Script + DNS | 4 | Ready |
| 3 — Release Workflow | 3 | Ready |
| **Total** | **8** | **0/8 done** |
