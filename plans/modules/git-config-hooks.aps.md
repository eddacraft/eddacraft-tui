<!--
APS Module: Git Config Hooks
====================================
Native Git 2.54 config-based hook support for repo workflows and Anvil surfaces.
See: plans/aps-rules.md
-->

# Git Config Hooks

| ID    | Owner | Status      | Progress |
| ----- | ----- | ----------- | -------- |
| GHOOK | —     | In Progress | 3/6 |

**Last reviewed:** 2026-04-26

## Purpose

Adopt Git 2.54 config-based hooks where they improve reliability, while keeping
Anvil compatible with existing file-based hook workflows. This module covers
both the monorepo's developer workflow and the product surfaces that currently
assume `.husky/` or `.git/hooks/*` installs.

**Problem:** Git 2.54 adds native `hook.<name>.event`,
`hook.<name>.command`, and `hook.<name>.enabled` support. The repo still uses
Husky plus `.husky/pre-commit`, and Anvil's CLI, TUI, doctor, status, and docs
mostly model file-based hooks only. That leaves us with avoidable wrapper
complexity in development and no first-class product story for native Git hook
configuration.

## In Scope

- **Repo workflow assessment:** Decide when this repo can rely on Git 2.54+
- **Installer support:** Allow `anvil hooks` to manage config-based hooks
- **Detection and status:** Recognise config-based hooks in doctor, status, and onboarding
- **Compatibility policy:** Define how config hooks coexist with `.git/hooks`, Husky, and other managers
- **Validation coverage:** Add automated tests for config-mode installation and detection
- **Documentation:** Update public docs and internal guidance to describe native Git hook configuration

## Out of Scope

- Removing `lint-staged` or changing what repo hooks execute
- Requiring users to edit global Git config by hand
- Server-side Git hook management
- Raising the minimum supported Git version for all Anvil users without an explicit decision

## Interfaces

**Depends on:**

- `crates/anvil-cli` — `anvil hooks`, `doctor`, `status`
- `crates/anvil-tui` — onboarding, tutorial, status surfaces
- `docs/public/anvil/` — public guidance and tutorials
- Repo developer workflow — current Husky plus `lint-staged` setup
- Git 2.54 hook API — native config-backed hook execution and listing

**Exposes:**

- Native Git hook installation mode for Anvil
- Compatibility policy for config and file hook coexistence
- Updated docs for local development and product onboarding

## Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Git 2.54 adoption is uneven across contributors and users | high | Keep file-hook fallback until baseline is explicit |
| Config hooks and file hooks both run, causing duplicate execution | high | Define precedence and detection before switching defaults |
| `git hook list` output differs by Git version or platform | medium | Feature-detect capability and test fallback paths |
| Product docs overstate support before detection lands | medium | Sequence docs after CLI and status support |

## Ready Checklist

- [x] Purpose and scope are clear
- [x] Product and repo workflows are both represented
- [x] Dependencies identified for CLI, TUI, docs, and repo tooling
- [x] Compatibility and fallback concerns called out
- [x] Work items ordered so repo migration depends on product support decisions

## Estimated Scope

- **Effort:** 1 week

## Tasks

### GHOOK-001: Git 2.54 baseline and rollout policy

- **Status:** Complete
- **Intent:** Decide the minimum Git version and fallback policy needed before repo or product flows depend on config-based hooks.
- **Expected Outcome:** A documented compatibility position covering repo contributors, CI, and Anvil users.
- **Files:**
  - `docs/guides/git-hook-compatibility.md`
  - `docs/public/anvil/operations/git-hooks.md`
  - `docs/public/anvil/tutorials/ci.md`
  - `docs/public/anvil/guides/agent-harness.md`
  - `docs/guides/README.md`
  - `package.json`
- **Dependencies:** —
- **Validation:** Compatibility policy is documented and referenced from hook guidance.
- **Confidence:** high

### GHOOK-002: `anvil hooks` config-mode install and uninstall

- **Status:** Done
- **Intent:** Let Anvil install and remove native config-based hooks without relying on shell files in `.husky/` or `.git/hooks/`.
- **Expected Outcome:** `anvil hooks` can manage config-backed pre-commit and pre-push entries.
- **Files:**
  - `crates/anvil-cli/src/commands/hooks.rs`
- **Dependencies:** GHOOK-001
- **Validation:** `cargo test -p eddacraft-anvil -- hooks` passes with config-mode coverage.
- **Confidence:** medium

### GHOOK-003: Status, doctor, and onboarding recognise config hooks

- **Status:** Done
- **Intent:** Treat config-based hooks as first-class in diagnostics and setup flows.
- **Expected Outcome:** Status, doctor, tutorial, and onboarding surfaces report config hooks accurately instead of assuming Husky or direct hook files.
- **Files:**
  - `crates/anvil-cli/src/commands/status.rs`
  - `crates/anvil-cli/src/commands/doctor.rs`
  - `crates/anvil-tui/src/surfaces/onboarding/hooks.rs`
  - `crates/anvil-tui/src/surfaces/tutorial/paths.rs`
- **Dependencies:** GHOOK-001, GHOOK-002
- **Validation:** Rust tests covering hook detection and status surfaces pass.
- **Confidence:** medium

### GHOOK-004: Coexistence and precedence rules

- **Status:** Todo
- **Intent:** Define how Anvil behaves when config-based hooks coexist with file hooks or third-party managers.
- **Expected Outcome:** Clear precedence and non-destructive behaviour for Husky, `.git/hooks`, lefthook, and config-backed hooks.
- **Files:**
  - `crates/anvil-cli/src/commands/hooks.rs`
  - `crates/anvil-tui/src/surfaces/onboarding/hooks.rs`
  - `docs/public/anvil/`
- **Dependencies:** GHOOK-002, GHOOK-003
- **Validation:** Behaviour is documented and covered by command-level tests.
- **Confidence:** medium

### GHOOK-005: Repo developer workflow migration decision

- **Status:** Todo
- **Intent:** Decide whether this repository should replace Husky with native Git config hooks once the baseline permits it.
- **Expected Outcome:** A repo-specific migration decision with explicit keep/replace criteria and any required bootstrap step.
- **Files:**
  - `package.json`
  - `.husky/`
  - `docs/guides/`
- **Dependencies:** GHOOK-001, GHOOK-004
- **Validation:** Migration decision is documented and the chosen workflow is reproducible from a fresh clone.
- **Confidence:** medium

### GHOOK-006: Docs and examples updated for native hooks

- **Status:** Todo
- **Intent:** Update examples so Anvil documentation reflects both native config hooks and legacy file-hook installs where relevant.
- **Expected Outcome:** Public docs, tutorials, and troubleshooting no longer imply that `.husky/` is the only preferred path.
- **Files:**
  - `docs/public/anvil/tutorials/ci.md`
  - `docs/public/anvil/guides/agent-harness.md`
  - `docs/public/anvil/operations/`
  - `docs/public/anvil/releases/`
- **Dependencies:** GHOOK-003, GHOOK-004
- **Validation:** `pnpm lint:md` passes and docs mention config hooks where hook setup is described.
- **Confidence:** high

## Stats

| Phase | Total | Done | In Progress | Todo |
| ----- | ----- | ---- | ----------- | ---- |
| Policy and compatibility | 2 | 1 | 0 | 1 |
| Product support | 2 | 2 | 0 | 0 |
| Repo and docs rollout | 2 | 0 | 0 | 2 |
| **Total** | 6 | 3 | 0 | 3 |

### Item Detail

| ID | Status | Notes |
| -- | ------ | ----- |
| GHOOK-001 | Complete | Compatibility doc + baseline pinned in `package.json` engines.git |
| GHOOK-002 | Done | Native config-hook install and uninstall via `--config`, with Git 2.54 refusal guard |
| GHOOK-003 | Done | Status, doctor, onboarding, and tutorial copy recognise config-mode entries; shared predicate lifted to `anvil_kernel_types::hooks` |
| GHOOK-004 | Todo | Defines safe coexistence with Husky, file hooks, and other managers |
| GHOOK-005 | Todo | Decides whether this repo should migrate off Husky |
| GHOOK-006 | Todo | Updates docs once detection and coexistence rules are settled |
