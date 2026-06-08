# Eddacraft-TUI Canonical Source Mirror

| ID        | Owner      | Status     | Progress |
| --------- | ---------- | ---------- | -------- |
| TUIMIRROR | joshuaboys | Superseded | 0/8      |

**Last reviewed:** 2026-06-08 (archived by TUIR-008). Prior: 2026-05-20.

> **Archived 2026-06-08 by TUIR-008. Superseded by:**
> [`tui-reintegration`](../../modules/tui-reintegration.aps.md) (TUIR).
> TUIR carried the same ADR-047 intent at higher resolution — the policy
> questions left implicit here (read-only vs release mirror, sync direction,
> versioning ownership, CI gate split, backport policy) became first-class work
> items in TUIR, which has now delivered them. No work was ever executed
> against TUIMIRROR (0/8); this file is retained as historical planning context
> only. All implementation and history live under TUIR.
>
> **Execution gate:** This module implements ADR-047. Do not promote tasks to
> `Ready` or begin implementation until ADR-047 is accepted. Until then this is
> planning context only.

## Purpose

Move `eddacraft-tui`'s canonical source back into Anvil while preserving the
public `eddacraft/eddacraft-tui` repository as an Apache-2.0 read-only mirror
and crates.io as the supported external distribution channel.

**Why:** Anvil is the load-bearing consumer of `eddacraft-tui`. Keeping the
library canonical in a separate public repository forces a release-and-bump loop
for Anvil-driven widget, theme, and snapshot changes. A canonical in-repo crate
lets Anvil TUI surface changes and shared widget changes land atomically, while
the public mirror keeps the trust and reuse benefits of the open-source surface.

## In Scope

- Import the current public `eddacraft-tui` crate into `crates/eddacraft-tui/`.
- Switch Anvil's workspace dependency from crates.io to the in-workspace crate.
- Preserve `eddacraft-tui` as an independently versioned public crate.
- Add mirror automation from Anvil to `eddacraft/eddacraft-tui:main`.
- Protect release tags and published crate versions from mirror rewrites.
- Define the crates.io publish path from the canonical Anvil source.
- Update public mirror docs so direct contributions are redirected correctly.
- Validate both Anvil's TUI consumers and the standalone crate boundary.

## Out of Scope

- Changing the `eddacraft-tui` public API except where required by relocation.
- Redesigning widgets, themes, or Anvil TUI surfaces.
- Changing `anvil-plan-spec` or `kindling` source-topology policy.
- Accepting direct source PRs into the public mirror after migration.
- Releasing a new Anvil product version solely because this migration lands.

## Interfaces

**Depends on:**

- ADR-047 — canonical-source and public-mirror decision, pending acceptance.
- `eddacraft/eddacraft-tui` — current public source and crates.io package.
- `crates/anvil-tui/` — primary in-repo consumer.
- `Cargo.toml` / `Cargo.lock` — workspace dependency source of truth.
- GitHub Actions — mirror and release automation.
- crates.io — external package publication channel.

**Exposes:**

- `crates/eddacraft-tui/` — canonical shared TUI crate in Anvil.
- Public mirror workflow for `eddacraft/eddacraft-tui`.
- Documented external consumption path via crates.io.
- Documented implementation/runbook path for future crate releases.

## Decisions

**D-TUIMIRROR-001:** Canonical source topology

- **Resolution:** Anvil owns the canonical source under `crates/eddacraft-tui/`.
  The public repository mirrors that subtree and is not independently edited.
- **Status:** Proposed by ADR-047.

**D-TUIMIRROR-002:** External consumption contract

- **Resolution:** External consumers should depend on crates.io releases, not the
  public mirror's moving `main` branch. Release tags are stable and must not be
  rewritten by mirror automation.
- **Status:** Proposed by ADR-047.

**D-TUIMIRROR-003:** Implementation sequencing

- **Resolution:** Source import and Anvil dependency switch happen before mirror
  activation. Mirror activation happens before public-repo documentation declares
  the mirror model active.
- **Status:** Proposed.

## Risks

- **Public git consumers of `main` see rewritten history.** Mitigation: document
  crates.io as the supported external consumption path and protect release tags.
- **Crate release automation accidentally couples to Anvil product releases.**
  Mitigation: preserve independent crate versioning and make publish steps
  explicit rather than automatic on every Anvil release.
- **Mirror docs drift from Anvil docs.** Mitigation: mirror README/CONTRIBUTING
  changes from the canonical source and keep ADR-047 linked from both sides.
- **Private-source canonical home makes contributions awkward.** Mitigation:
  issues remain usable in the public repo; maintainers port accepted changes
  upstream manually.

## Work Items

### TUIMIRROR-001: Reconcile current public crate state

**Status:** open

- **Intent:** Establish the exact public source, version, tags, release workflow,
  and crates.io state that will be imported into Anvil.
- **Expected Outcome:** A recorded baseline for `eddacraft-tui` source, package
  metadata, tags, and workflows before relocation.
- **Validation:** `cargo test` in the standalone `eddacraft-tui` repo; `cargo
  publish --dry-run --all-features` where credentials are not required.

**changeType:** docs
**releaseIntent:** never
**releaseScope:** none

### TUIMIRROR-002: Import `eddacraft-tui` into Anvil

**Status:** open

- **Intent:** Add the public crate source under `crates/eddacraft-tui/` without
  changing behaviour.
- **Expected Outcome:** Anvil contains the canonical crate source with package
  metadata, docs, tests, examples, and feature flags preserved.
- **Validation:** `cargo test -p eddacraft-tui --all-features`.

**changeType:** internal
**releaseIntent:** hold
**holdCondition:** Hold until Anvil consumes the imported crate and mirror
automation is ready.
**releaseScope:** none

### TUIMIRROR-003: Switch Anvil to the workspace crate

**Status:** open

- **Intent:** Replace the crates.io dependency used by Anvil with the imported
  workspace/path crate.
- **Expected Outcome:** `crates/anvil-tui/` and other Anvil consumers resolve
  `eddacraft-tui` from the workspace.
- **Validation:** `cargo test -p eddacraft-anvil-tui`; `cargo test --workspace`.

**changeType:** internal
**releaseIntent:** candidate
**releaseScope:** patch

### TUIMIRROR-004: Add mirror workflow

**Status:** open

- **Intent:** Mirror `crates/eddacraft-tui/` to `eddacraft/eddacraft-tui:main`
  using the established public-mirror pattern.
- **Expected Outcome:** A least-privilege workflow can publish the subtree to the
  public repository without rewriting release tags.
- **Validation:** Manual workflow dispatch against `main` succeeds; public mirror
  tree matches `crates/eddacraft-tui/`.

**changeType:** internal
**releaseIntent:** never
**releaseScope:** none

### TUIMIRROR-005: Define independent crate release flow

**Status:** open

- **Intent:** Document and wire the process for publishing `eddacraft-tui` from
  Anvil while preserving independent semver from the Anvil product.
- **Expected Outcome:** Maintainers have an executable release path for crates.io
  that does not imply an Anvil product release.
- **Validation:** Release workflow dry-run or documented manual dry-run proves the
  crate package contents and version source.

**changeType:** docs
**releaseIntent:** never
**releaseScope:** none

### TUIMIRROR-006: Update public mirror documentation

**Status:** open

- **Intent:** Make `eddacraft/eddacraft-tui` accurately describe itself as a
  read-only mirror after mirror automation is active.
- **Expected Outcome:** Public README/CONTRIBUTING/SECURITY or equivalent docs
  explain the mirror model, crates.io consumption path, issue policy, and
  contribution path.
- **Validation:** Public mirror docs contain the mirror notice after a successful
  sync.

**changeType:** docs
**releaseIntent:** never
**releaseScope:** none

### TUIMIRROR-007: Remove obsolete external-repo assumptions

**Status:** open

- **Intent:** Sweep Anvil docs and plans for stale claims that `eddacraft-tui` is
  independently canonical outside Anvil.
- **Expected Outcome:** ADRs, architecture docs, and relevant module references
  consistently describe the accepted topology.
- **Validation:** `pnpm docs:check`; `pnpm adr:check`; targeted search for
  `eddacraft-tui` topology claims.

**changeType:** docs
**releaseIntent:** never
**releaseScope:** none

### TUIMIRROR-008: Close migration with end-to-end verification

**Status:** open

- **Intent:** Prove that the in-repo crate, Anvil consumers, mirror, and publish
  path work as one operating model.
- **Expected Outcome:** Final verification evidence is captured; module can advance
  toward Done once PRs merge and mirror/public docs are live.
- **Validation:** `cargo test --workspace`; `pnpm adr:check`; `pnpm docs:check`;
  mirror tree comparison; crate package dry-run.

**changeType:** internal
**releaseIntent:** candidate
**releaseScope:** patch

## Ready Checklist

- [ ] ADR-047 accepted.
- [ ] Current `eddacraft-tui` source/tag/crates.io baseline recorded.
- [ ] Mirror credentials approach chosen and scoped to `eddacraft/eddacraft-tui`.
- [ ] Release ownership for independent crate publishing confirmed.
- [ ] Rollback path documented for Anvil dependency switch.
- [ ] Validation commands confirmed locally.
