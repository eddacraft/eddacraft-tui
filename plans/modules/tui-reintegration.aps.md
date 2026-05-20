<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# TUI Reintegration

| ID   | Owner      | Status   | Progress |
| ---- | ---------- | -------- | -------- |
| TUIR | joshuaboys | Proposed | 0/8      |

**Last reviewed:** 2026-05-20

> **Execution gate:** Implements ADR-047. Tasks may not be promoted from
> `Proposed` to `Ready` until ADR-047 is accepted and the Ready Checklist
> below is satisfied. Until then this is planning context only.
>
> **Supersedes:** [`eddacraft-tui-canonical-source`](./eddacraft-tui-canonical-source.aps.md)
> (TUIMIRROR, 0/8, Proposed). TUIR carries the same intent at a higher
> resolution — the policy questions left implicit in TUIMIRROR (read-only vs
> release mirror, sync direction, versioning ownership, CI gate split,
> backport policy) are first-class work items here.

## Purpose

Bring `eddacraft-tui` back into the Anvil monorepo as the canonical source,
keep `eddacraft/eddacraft-tui` as a read-only public mirror, and continue
publishing the crate to crates.io as the supported external distribution
channel.

**Why:** Anvil is the load-bearing consumer of `eddacraft-tui`. The current
"Anvil consumes crates.io" topology forces a release-bump-and-re-verify loop
for every widget, theme, or snapshot change driven by Anvil's own TUI
surfaces. The acknowledgements starter (ATTRIB-011, shipped 2026-05-18) has
already proven the inverse topology: canonical source in `anvil-001`, public
mirror via a least-privilege workflow, downstream consumers via the mirror.
TUIR applies that pattern to `eddacraft-tui` while preserving the public
trust surface and crates.io contract.

## In Scope

- Decide and record where the crate lives in the monorepo.
- Decide whether `eddacraft/eddacraft-tui` becomes a pure read-only mirror or
  a release-tag mirror, and pin the consequences.
- Define the sync direction, automation, and credentials.
- Pin the crates.io publish source (which tree, which workflow, what
  triggers a publish).
- Assign versioning and changelog ownership for the published crate.
- Define the CI gate split between Anvil and the public mirror.
- Define how Anvil consumes the workspace path crate while external users
  consume the crates.io release.
- Define the backport / mirror conflict policy (drift, drive-by PRs,
  emergency fixes on the mirror side).

## Out of Scope

- Redesigning `eddacraft-tui` widgets, themes, or Anvil TUI surfaces.
- Changing the published public API except where relocation forces it.
- Changing `anvil-plan-spec` or `kindling` source-topology policy.
- Accepting direct source PRs into the public mirror after migration.
- Releasing a new Anvil product version solely because this migration
  lands.
- Re-vendoring `eddacraft-tui` privately or sunsetting the public crate.

## Interfaces

**Depends on:**

- [ADR-047](../decisions/047-eddacraft-tui-canonical-source-mirror.md) —
  canonical-source + public-mirror decision; pending acceptance.
- ATTRIB-011 mirror precedent —
  `.github/workflows/mirror-acknowledgements-starter.yml` and the
  least-privilege PAT pattern (`http.extraheader` Basic auth, not
  URL-embedded `x-access-token:${TOKEN}@github.com` — the embedded form
  is brittle to stray bytes in the secret and fails as
  `CURLE_URL_MALFORMAT`).
- `eddacraft/eddacraft-tui` — current public source repo and crates.io
  package owner.
- `crates/anvil-tui/` — primary in-repo consumer.
- `Cargo.toml` / `Cargo.lock` workspace dependency surface.
- crates.io — external package distribution channel.

**Exposes:**

- `crates/eddacraft-tui/` — canonical workspace crate location.
- `.github/workflows/mirror-eddacraft-tui.yml` — mirror automation
  (mirrors `crates/eddacraft-tui/` subtree to
  `eddacraft/eddacraft-tui:main`).
- Documented external consumption contract (crates.io tag, not git
  `main`).
- Documented crate release runbook
  (`docs/runbooks/eddacraft-tui-release.md`, new).
- Documented backport / mirror conflict policy
  (`docs/policies/eddacraft-tui-mirror.md`, new).

## Decisions

**D-TUIR-001:** Canonical source location

- **Resolution:** Anvil owns the canonical source at
  `crates/eddacraft-tui/`. Sits alongside other first-party Rust crates
  (`crates/anvil-tui/`, `crates/anvil-l4/`, etc.) and is a workspace
  member. The public repository mirrors that subtree and is not
  independently edited.
- **Status:** Proposed by ADR-047.

**D-TUIR-002:** Public repo role — read-only mirror with release tags

- **Resolution:** `eddacraft/eddacraft-tui:main` is mirror-managed and
  force-pushed by automation. Release tags follow the prefixed form
  `eddacraft-tui-vX.Y.Z` everywhere — Anvil canonical source, mirror,
  and crates.io. The prefixed form is mandatory because Anvil ships
  other crates with independent semver from the same monorepo; an
  unprefixed `vX.Y.Z` would collide with Anvil product tags. Release
  tags are protected by branch/tag rules and are NEVER rewritten by
  the mirror job. The repo is therefore a hybrid: read-only on `main`,
  append-only on `eddacraft-tui-v*` tags. Issues remain open; source
  PRs are closed with a redirect template.
- **Status:** Proposed.

**D-TUIR-003:** Sync direction

- **Resolution:** One-way Anvil → public mirror. There is no reverse sync.
  Drive-by PRs against the mirror are closed with a redirect; if the
  change is accepted, a maintainer ports it into `crates/eddacraft-tui/`
  inside Anvil and the next mirror run carries it out.
- **Status:** Proposed.

**D-TUIR-004:** Sync automation

- **Resolution:** A GitHub Actions workflow (`mirror-eddacraft-tui.yml`)
  in `anvil-001` watches `crates/eddacraft-tui/**` on `main` and
  force-pushes the subtree to `eddacraft/eddacraft-tui:main` on every
  change. Auth uses a fine-scoped PAT via `http.extraheader` Basic
  auth, scoped to `eddacraft/eddacraft-tui` only (matching the
  ATTRIB-011 `.github/workflows/mirror-acknowledgements-starter.yml`
  pattern). Manual `workflow_dispatch` is supported for catch-up.
- **Status:** Proposed.

**D-TUIR-005:** crates.io publish source

- **Resolution:** Crates are published from the Anvil canonical source
  (`crates/eddacraft-tui/` on `main`), not from the public mirror. The
  publish workflow tags `eddacraft-tui-vX.Y.Z` on `anvil-001`, runs
  `cargo publish` from the workspace crate, and the mirror job propagates
  the tag to `eddacraft/eddacraft-tui` as an append-only tag (no
  rewrite). The public mirror's `main` will track the same tree but
  publishing never originates from the mirror.
- **Status:** Proposed.

**D-TUIR-006:** Versioning and changelog ownership

- **Resolution:** `eddacraft-tui` keeps independent semver from Anvil
  product releases. `crates/eddacraft-tui/CHANGELOG.md` is the canonical
  changelog; entries are written in Anvil PRs. An Anvil product release
  MUST NOT bump the crate version automatically; a crate release MUST NOT
  imply an Anvil product release. Version bumps land in their own PR
  with `releaseIntent: candidate` on the crate side and `none` on the
  Anvil product side.
- **Status:** Proposed.

**D-TUIR-007:** CI gate split

- **Resolution:**
  - **Anvil side (`anvil-001`):** `cargo test -p eddacraft-tui
    --all-features` and `cargo test --workspace` are the load-bearing
    gates. Workspace clippy must run as `cargo clippy --workspace
    --all-targets -- -D warnings` (per-crate `-p` invocations miss
    doc-markdown errors in sibling crates) and `cargo fmt --all
    --check` must run alongside it (workspace clippy with `-D
    warnings` does NOT run rustfmt, so tests + clippy can be green
    while CI's Format check fails).
  - **Public mirror side:** retain only `cargo test` and `cargo publish
    --dry-run --all-features` as a smoke gate against a fresh checkout
    of the mirrored tree. The mirror does not re-run the full Anvil
    matrix; that would be theatre.
- **Status:** Proposed.

**D-TUIR-008:** Consumption contract

- **Resolution:**
  - **Anvil (internal):** `crates/anvil-tui/Cargo.toml` and other
    consumers resolve `eddacraft-tui` via `path =
    "../eddacraft-tui"` workspace inheritance, NOT crates.io. Local
    development gets atomic widget + consumer changes for free.
  - **External users:** depend on the crates.io release
    (`eddacraft-tui = "X.Y"`), NOT the public git `main`. Git `main`
    is explicitly documented as mirror-managed and rewritable.
- **Status:** Proposed.

**D-TUIR-009:** Backport / mirror conflict policy

- **Resolution:**
  - **Drive-by PR on the mirror:** auto-closed with a template pointing
    at the canonical source path and the contribution guide. Maintainer
    discretion to port the change.
  - **Mirror force-push collision (someone pushed to the mirror
    directly):** the next scheduled mirror run wins; lost work is the
    pusher's responsibility, documented up front. Public README warns
    against direct pushes.
  - **Emergency security fix:** lands in `anvil-001` first, mirror
    propagates within one workflow run, crates.io publish follows the
    standard publish workflow. No "fix on mirror first" path is
    sanctioned.
  - **Tag conflict (someone tagged on the mirror):** mirror automation
    refuses to overwrite existing tags; the operator resolves manually
    and the conflicting tag is either renamed or yanked from crates.io.
- **Status:** Proposed.

## Risks

- **Public git consumers of `main` see rewritten history.** Mitigation:
  document crates.io as the supported external consumption path; protect
  release tags so they are never rewritten; add a public README banner
  naming `main` as mirror-managed.
- **Crate release accidentally couples to Anvil product release.**
  Mitigation: D-TUIR-006 keeps independent versioning; publish workflow
  is gated on an explicit `eddacraft-tui-vX.Y.Z` tag, not on Anvil
  release tags.
- **Mirror PAT scope creep.** Mitigation: PAT is fine-scoped to
  `eddacraft/eddacraft-tui` only and stored in repo secrets; rotation
  cadence documented alongside ATTRIB-011's PAT.
- **Drift between Anvil docs and mirror docs.** Mitigation: README,
  CONTRIBUTING, and SECURITY for `eddacraft-tui` live under
  `crates/eddacraft-tui/` and are mirrored verbatim; no parallel mirror
  edits.
- **Atomicity loss if the mirror job fails mid-release.** Mitigation:
  publish workflow runs `cargo publish` only after the mirror push
  succeeds AND the tag propagation step succeeds; failure leaves a clean
  rollback point on Anvil side.
- **Issue traffic on the mirror is ignored.** Mitigation: issues remain
  open on the mirror; a triage rotation forwards relevant items to
  `anvil-001` issues. PRs are auto-closed with redirect.

## Work Items

### TUIR-001: Lock the import baseline

**Status:** open

**Intent:** Record the exact public source, version, tags, release
workflow, and crates.io state that will be imported into Anvil so the
relocation has a recorded provenance.

**Outcome:** A baseline document at
`plans/specs/2026-05-20-tui-reintegration-baseline.md` capturing current
`eddacraft-tui` source SHA, latest crates.io version, tag list, public
CI surface, and identified deltas to fold into the in-repo crate.

**Validation:** `cargo test` against the standalone repo at the recorded
SHA; `cargo publish --dry-run --all-features` succeeds against the same
tree.

**changeType:** docs
**releaseIntent:** never
**releaseScope:** none

### TUIR-002: Import `eddacraft-tui` into `crates/eddacraft-tui/`

**Status:** open

**Intent:** Move the canonical source into the workspace without
behaviour or API change.

**Outcome:** `crates/eddacraft-tui/` contains the imported crate with
package metadata, docs, tests, examples, feature flags, and CHANGELOG
preserved. Workspace `Cargo.toml` lists it as a member.

**Validation:** `cargo test -p eddacraft-tui --all-features`; `cargo
test --workspace`; `cargo fmt --all --check`; `cargo clippy --workspace
--all-targets -- -D warnings`.

**changeType:** internal
**releaseIntent:** hold
**holdCondition:** Hold publishing until TUIR-003 (Anvil consumes the
workspace crate) and TUIR-004 (mirror automation) are merged.
**releaseScope:** none

### TUIR-003: Switch Anvil consumers to the workspace path crate

**Status:** open

**Intent:** Replace the crates.io dependency on `eddacraft-tui` with the
in-workspace path crate inside Anvil.

**Outcome:** `crates/anvil-tui/` and any other Anvil crates that consume
`eddacraft-tui` resolve it via workspace `path =` inheritance. The
crates.io `eddacraft-tui` entry no longer appears in the workspace
`Cargo.lock` as an external dependency for first-party crates.

**Validation:** `cargo tree -p eddacraft-anvil-tui -i eddacraft-tui`
shows the path crate; `cargo test --workspace`.

**changeType:** internal
**releaseIntent:** candidate
**releaseScope:** patch

### TUIR-004: Mirror `crates/eddacraft-tui/` to the public repo

**Status:** open

**Intent:** Ship `.github/workflows/mirror-eddacraft-tui.yml` modelled
on `mirror-acknowledgements-starter.yml`, mirroring the subtree to
`eddacraft/eddacraft-tui:main` on every change, with tag protection
honoured.

**Outcome:** A workflow_dispatch + path-filtered push trigger that
publishes the subtree, uses fine-scoped PAT via `http.extraheader`
Basic auth, refuses to overwrite existing tags, and emits a run summary
linking the mirrored SHA.

**Validation:** Manual `workflow_dispatch` against `main` succeeds;
public mirror tree byte-matches `crates/eddacraft-tui/`; existing
release tag remains intact across the run.

**changeType:** internal
**releaseIntent:** never
**releaseScope:** none

### TUIR-005: Wire the crates.io publish workflow from canonical source

**Status:** open

**Intent:** Add a publish workflow that releases `eddacraft-tui` to
crates.io from `anvil-001` canonical source, independent of Anvil
product releases.

**Outcome:** `.github/workflows/publish-eddacraft-tui.yml` triggers on
`eddacraft-tui-v*` tags pushed to `anvil-001`, runs the standard test
matrix, runs `cargo publish` with the crates.io token, then waits for
the mirror job to propagate the tag. `docs/runbooks/eddacraft-tui-release.md`
documents the cut.

**Validation:** Dry-run publish against a release candidate tag; runbook
walkthrough captured on PR.

**changeType:** internal
**releaseIntent:** never
**releaseScope:** none

### TUIR-006: Split CI gates between Anvil and mirror

**Status:** open

**Intent:** Pin which gates run where so the mirror does not duplicate
Anvil's full matrix and Anvil retains authoritative validation.

**Outcome:** Anvil side: `cargo test -p eddacraft-tui --all-features`,
`cargo test --workspace`, workspace clippy with `-D warnings`, and
`cargo fmt --all --check` are required on PRs touching
`crates/eddacraft-tui/**`. Mirror side: a minimal `cargo test` + `cargo
publish --dry-run` smoke workflow runs on every mirror push. Both gate
contracts are documented in `docs/policies/eddacraft-tui-mirror.md`.

**Validation:** `pnpm test:ci-workflow-contracts` (or successor) lists
the new workflows; doc check passes; intentional failures on each side
surface only the gate that should run there.

**changeType:** internal
**releaseIntent:** never
**releaseScope:** none

### TUIR-007: Document mirror policy and update public surfaces

**Status:** open

**Intent:** Make the public repo accurately describe itself as a
read-only mirror, redirect contributions, and document the backport /
conflict policy.

**Outcome:** Public `README.md`, `CONTRIBUTING.md`, and `SECURITY.md` on
`eddacraft/eddacraft-tui` (mirrored from `crates/eddacraft-tui/`)
explain the mirror model, crates.io consumption path, issue policy, and
where source contributions actually land. A PR-redirect template
auto-closes drive-by source PRs against the mirror.
`docs/policies/eddacraft-tui-mirror.md` (in Anvil) is the canonical
copy of the backport / conflict policy.

**Validation:** Public mirror docs contain the mirror notice after a
successful sync; `pnpm docs:check`; `pnpm adr:check`; targeted search
for stale "eddacraft-tui is independently canonical" claims returns
zero hits.

**changeType:** docs
**releaseIntent:** never
**releaseScope:** none

### TUIR-008: End-to-end verification and TUIMIRROR retirement

**Status:** open

**Intent:** Prove the in-repo crate, Anvil consumers, mirror, publish
path, and policy docs work as one operating model, and archive the
superseded TUIMIRROR module.

**Outcome:** A full dry-run cuts a candidate `eddacraft-tui-vX.Y.Z` tag
in Anvil, the mirror propagates, `cargo publish --dry-run` succeeds, no
Anvil product release is implied, and the public mirror reflects the
canonical tree. TUIMIRROR is `git mv`'d to `plans/archive/modules/`
with a redirect note pointing at TUIR.

**Validation:** `cargo test --workspace`; `pnpm adr:check`; `pnpm
docs:check`; mirror tree byte-comparison; crate `cargo publish
--dry-run --all-features`; index.aps.md reflects archive.

**changeType:** internal
**releaseIntent:** candidate
**releaseScope:** patch

## Ready Checklist

- [ ] ADR-047 accepted (moves from Proposed → Accepted in
      `DECISION-LOG.md`).
- [ ] TUIMIRROR superseded note added to
      `eddacraft-tui-canonical-source.aps.md` and to the index row.
- [ ] Current `eddacraft-tui` source SHA, latest crates.io version, and
      tag list recorded in the baseline spec (TUIR-001 deliverable).
- [ ] Mirror PAT scope agreed and stored in `anvil-001` repo secrets
      under a named secret (e.g. `EDDACRAFT_TUI_MIRROR_PAT`).
- [ ] crates.io token ownership confirmed for the new publish workflow.
- [ ] Rollback path documented for the Anvil dependency switch (how to
      revert `crates/anvil-tui/Cargo.toml` to the crates.io dep without
      losing in-flight Anvil work).
- [ ] `docs/policies/eddacraft-tui-mirror.md` draft reviewed before
      TUIR-007 implementation begins.
