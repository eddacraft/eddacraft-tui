# ADR-025: Package Manager Distribution Strategy

## Status

Accepted

> **Freshness note (2026-05-16):** This ADR remains the historical package-
> manager strategy decision. Current release cadence and beta support-window
> policy lives in [`docs/policies/release-cadence.md`](../../docs/policies/release-cadence.md),
> and active distribution execution lives in
> [`plans/archive/modules/distribution-and-update.aps.md`](../archive/modules/distribution-and-update.aps.md).

## Date

2026-04-18

## Context

Anvil 0.3.3-beta ships with four distribution channels:

- **curl installer** — `install.eddacraft.ai` (Linux/macOS)
- **Homebrew tap** — `eddacraft/homebrew-tap` (macOS)
- **WinGet** — `winget install eddacraft.anvil` (Windows, accepted Apr 2026)
- **Scoop** — `eddacraft/scoop-bucket` (Windows developer audience, DIST-011)

A question arose: which additional package managers are worth pursuing, and
when? This ADR captures the assessment of all major package managers,
the decision on current scope, and a revisit trigger.

---

## Package Manager Assessment

### ✅ Shipped or in progress

| Manager | Platform | Status | Notes |
| ------- | -------- | ------ | ----- |
| curl installer | Linux/macOS | **Live** | `install.eddacraft.ai` |
| Homebrew | macOS | **Live** | `eddacraft/homebrew-tap` |
| WinGet | Windows | **Live** | Accepted Apr 2026 |
| Scoop | Windows | **DIST-011 (Ready)** | Developer audience; no review queue |

### 🟡 Worth revisiting after beta signal

| Manager | Platform | Effort | Audience fit | Process summary | Revisit trigger |
| ------- | -------- | ------ | ------------ | --------------- | --------------- |
| **apt/deb (personal repo)** | Ubuntu/Debian | Medium | High — Ubuntu is the dominant Linux dev OS | Create GPG-signed apt repo hosted on GitHub Pages or R2. Add `.deb` build to release workflow. Publish `sources.list` one-liner for users. No review process. | When a beta user explicitly requests it, or when Linux install complaints exceed 3 |
| **Nix / nixpkgs** | Linux/macOS | Medium-High | Medium-High — Nix users are senior devs who value reproducibility; strong overlap with Anvil target persona | Submit derivation to `nixpkgs` (takes weeks, community review) or maintain a personal flake overlay (instant, no review). Flake overlay is the right starting point. | When a Nix user files an issue or requests it, or when nixpkgs submission is strategically timed with a stable release |
| **RPM / dnf (COPR)** | Fedora/RHEL | Medium | Medium — real enterprise Linux audience but smaller than Ubuntu at the developer/individual tier | Build `.rpm` in release workflow. Publish to Fedora COPR (free, self-serve, community hosting). No formal review. | When enterprise beta users on RHEL/Fedora request it (likely post-GA, not beta) |
| **apk (Alpine)** | Alpine Linux | Low | Low-Medium — mostly container/server use; Anvil is a dev workstation tool | Add `apk` build target to release workflow. Submit to Alpine `aports` (community review) or ship in release assets. | When container-focused users request it (e.g. running Anvil in CI containers) |
| **Chocolatey** | Windows | Low | Medium — enterprise Windows environments where WinGet is unavailable (older Windows 10, managed machines behind IT policy). Complements WinGet (official) and Scoop (developer). | Create `eddacraft/chocolatey-packages` repo with a `.nuspec` manifest pointing at the GitHub Release zip. Submit to Chocolatey Community Repository (moderated review, typically 1–3 days for new packages, auto-approved for updates). Add `choco` job to release workflow using `chocolatey/actions`. | When enterprise beta users report WinGet unavailable on their managed machines |

### ❌ Not worth pursuing

| Manager | Platform | Reason |
| ------- | -------- | ------ |
| **Snap** | Linux | Sandboxing actively fights CLI tools. Poor developer reputation. Avoid. |
| **Flatpak** | Linux | Designed for GUI desktop apps. Wrong fit for a CLI tool. |
| **AUR (Arch)** | Arch Linux | Community can self-maintain an AUR package. Not our responsibility. |
| **crates.io** | Cross-platform | Incompatible with closed-source IP model (see ADR-018). Deferred indefinitely (DIST-008). |
| **npm** | Cross-platform | Shelling out a Rust binary via npm is a footgun. Adds Node.js as a runtime dependency for no reason. |
| **pip / PyPI** | Cross-platform | Same problem as npm — wrong runtime dependency. |
| **MacPorts** | macOS | Negligible audience overlap. Homebrew covers macOS. |


---

## Decision

**Current scope (beta):** Ship Scoop (DIST-011). No additional package managers
until beta user signal warrants it.

**Rationale:** The existing four channels (curl, Homebrew, WinGet, Scoop) cover
~95% of realistic beta users. Adding more channels before we have users
requesting them is premature optimisation that consumes release engineering
time better spent on product.

**Priority order when signal arrives:**
1. apt/deb personal repo — highest audience fit, self-serve, no review queue
2. Nix flake overlay — high persona fit, low friction to maintain
3. RPM/COPR — enterprise signal post-GA
4. Chocolatey — enterprise managed Windows (no WinGet)
5. Alpine apk — container/CI use case only

---

## Revisit Triggers

Revisit this ADR when **any** of the following occur:

- [ ] A beta user explicitly requests a package manager not currently shipped
- [ ] Linux install complaints (`curl | sh` failures or friction) exceed 3 reports
- [ ] Stable GA release is planned (apt + Nix are worth adding at GA for credibility)
- [ ] Enterprise pilot requires a managed install method (apt/RPM likely)
- [ ] 6 months post-GA with no requests → mark remaining rows as "Deferred indefinitely"

---

## Consequences

- DIST-011 (Scoop) is the only near-term distribution work remaining
- All other package managers are explicitly deferred with documented rationale
- Future requests have a decision record to reference rather than re-litigating from scratch
- Release workflow complexity stays low during beta
