<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Acknowledgements Starter Releases

| ID     | Owner      | Status   |
| ------ | ---------- | -------- |
| ATTRIB | joshuaboys | Proposed |

**Last reviewed:** 2026-06-08

## Purpose

Give external consumers of the public mirror
[`eddacraft/acknowledgements-starter`](https://github.com/eddacraft/acknowledgements-starter)
a way to **know when the kit changed**, **read what changed**, and **pin to an
immutable version** — by layering a deliberate semver-tag + GitHub-Release
surface on top of the existing rolling-`main` mirror.

ATTRIB-011 made the kit consumable from public repos by force-pushing
`tools/starters/acknowledgements/` to the mirror's `main` on every change. That
is the right "track latest" surface, but it gives consumers no notification, no
changelog, and no stable ref to pin: `git subtree pull … main` always grabs a
HEAD that is rewound on the next sync.

This module adds the second surface, modelled on the proven `eddacraft-tui`
release flow (D-TUIR-002 / -006 / -009 / -021). It **retains the ATTRIB
lineage** (continuing the id space after ATTRIB-016) rather than re-opening the
archived [`attribution-pipeline-v3`](../archive/modules/attribution-pipeline-v3.aps.md)
module, which is genuinely Complete.

Design contract:
[`plans/specs/2026-06-08-acknowledgements-starter-releases.md`](../specs/2026-06-08-acknowledgements-starter-releases.md).
Execution plan:
[`plans/execution/ATTRIB-017.steps.md`](../execution/ATTRIB-017.steps.md).

## Why a new module rather than re-opening v3

`attribution-pipeline-v3` is archived Complete (15/16, archived 2026-05-26).
Re-opening an archived module is a multi-file un-archive cascade and the module
is genuinely closed — this is new scope (a *release* surface, not a kit
capability). Spawning a focused module keeps the ATTRIB lineage and leaves room
for follow-ons (a second starter kit reusing the same release pattern). This
mirrors how `supply-chain-attestation` (SCA) was spawned from the same archived
parent for the deferred ATTRIB-005 direction.

## In Scope

- A deliberate, **append-only** semver-tag surface on the mirror, kept
  independent of the unchanged rolling-`main` force-push.
- An in-tree `VERSION` + `CHANGELOG.md` for the kit as the durable version
  record (travels into the mirror), starting at **1.0.0**.
- A tag-triggered release workflow that subtree-splits, pushes a bare `vX.Y.Z`
  tag to the mirror (no `--force`), and cuts a GitHub Release with notes from
  the changelog.
- An operator runbook + consumer-facing pinning docs.

## Out of Scope

- The rolling-`main` mirror (`mirror-acknowledgements-starter.yml`) — unchanged.
- Per-push / automatic releases — releases are deliberate.
- Publishing to any package registry (npm, crates.io) — the kit is a
  `git subtree` artifact.
- SBOM / attestation — owned by [`supply-chain-attestation`](./supply-chain-attestation.aps.md).

## Interfaces

**Depends on:** ATTRIB-011 mirror + the `MIRROR_PUSH_TOKEN` secret
(`Contents: write` covers both tag push and Release creation — no new secret);
the kit's `expand-licences.sh --check` drift gate; the
`eddacraft-tui` release flow as the pattern source.

**Exposes:** immutable `vX.Y.Z` tags + GitHub Releases on
`eddacraft/acknowledgements-starter`; a `git subtree add … vX.Y.Z` pinning path
for consumers.

## Prerequisites

- The mirror has no **tag protection** rule blocking the PAT from creating `v*`
  tags (pre-flight probe per the tui runbook gotcha #5; add an exception if
  one exists).
- `MIRROR_PUSH_TOKEN` is valid (not near expiry) and confirmed able to create
  Releases.

## Open Questions

- Whether future starter kits should share one release workflow (parametrised
  by kit dir) or each carry their own.
- Whether to also surface a machine-readable "latest version" endpoint beyond
  the GitHub Release `latest` pointer.

## Work Items

### ATTRIB-017: Versioned releases on the acknowledgements-starter mirror

- **Status:** Proposed
- **Intent:** Consumers of the public mirror can be notified of updates, read a
  changelog, and pin to an immutable version.
- **Expected Outcome:** A deliberate `vX.Y.Z` tag + GitHub Release surface on
  `eddacraft/acknowledgements-starter` (starting at v1.0.0), independent of the
  unchanged rolling-`main` mirror, backed by an in-tree `VERSION` + `CHANGELOG`
  and a tag-triggered append-only release workflow.
- **Validation:** After the first cut,
  `gh release view v1.0.0 --repo eddacraft/acknowledgements-starter` shows the
  Release marked latest;
  `gh api repos/eddacraft/acknowledgements-starter/git/refs/tags/v1.0.0`
  resolves; `git subtree add … v1.0.0 --squash` into a scratch repo reproduces
  the kit tree; the kit self-tests (incl. the new version/changelog consistency
  test) and `expand-licences.sh --check` pass in CI. Full execution breakdown
  in [`plans/execution/ATTRIB-017.steps.md`](../execution/ATTRIB-017.steps.md).
- **Files:** `tools/starters/acknowledgements/VERSION`,
  `tools/starters/acknowledgements/CHANGELOG.md`,
  `tools/starters/acknowledgements/tests/version-changelog-consistency.sh`,
  `.github/workflows/release-acknowledgements-starter.yml`,
  `.github/workflows/acknowledgements-kit.yml`,
  `.github/workflows/README.md`,
  `docs/runbooks/acknowledgements-starter-release.md`,
  `tools/starters/acknowledgements/MIRROR-README.md`,
  `tools/starters/acknowledgements/README.md`.
- **Dependencies:** ATTRIB-011 (mirror), ATTRIB-016 (deterministic expander).
- **Confidence:** high — the design copies an in-repo flow already in
  production (`publish-eddacraft-tui.yml` + its runbook).

## Notes

Spawned 2026-06-08 from an operator request to publish the latest kit version to
the mirror as releases "so people know when it is updated". The kit was already
in sync on rolling `main` (mirror run 27118992612, 2026-06-08); the gap is the
*notification + pinning* surface, not freshness. Starting version fixed at
**1.0.0** by operator directive (four ecosystem drivers + bundled-binaries have
shipped; the contract is stable). Kept as a single work item with a 7-action
execution plan, matching the ATTRIB-011 precedent.
