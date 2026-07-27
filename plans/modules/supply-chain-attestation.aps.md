<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Supply-Chain Attestation

| ID  | Owner      | Status   |
| --- | ---------- | -------- |
| SCA | joshuaboys | Proposed |

**Last reviewed:** 2026-07-28 — enrolled in
[Graph Trust Surfaces](../specs/2026-07-28-graph-trust-surfaces.md) Wave 0
(SCA-001 one-ecosystem design). Module remains **Proposed** until design
accept and graph-ingest feasibility are confirmed.

## Purpose

Capture (not yet build) the initiative that the
[`attribution-pipeline-v3`](../archive/modules/attribution-pipeline-v3.aps.md) module
deliberately scoped out: turning Anvil's dependency footprint into a
machine-readable, graph-queryable, policy-gated artefact rather than
only human-readable attribution markdown.

The attribution pipeline answers *"what licences do our dependencies
ship under?"* and renders that as `ACKNOWLEDGEMENTS.md`. This module
asks the structured questions that markdown throws away: *what is the
full dependency graph, where did each edge come from, did this change
introduce a new transitive dependency / licence / known vulnerability,
and can we attest to it at release?*

This proposal exists so the ambition is recorded and plannable. It is
**not Ready** — it is gated on Anvil's graph/witness infrastructure
being able to consume a dependency graph (`ATTRIB-005` was deferred here
on 2026-05-25 for exactly this reason).

## Why this is a strong fit for Anvil's primitives

- **CycloneDX SBOM** carries components **and** dependency edges (purls,
  versions, hashes) — the structured source markdown discards. It is the
  natural intermediate for dependency mapping.
- **New-edges-only** (a core Anvil architecture principle): baseline the
  dependency graph, warn on *new* edges — i.e. flag a PR that pulls in a
  new transitive dep, a licence change, or a known-vulnerable version.
  This is the same shape as the witness/protection-claim model, applied
  to the dependency graph instead of the filesystem.
- **L4 policy** can adjudicate the dep graph (disallowed licence / vuln /
  unexpected edge) with the same warnings-over-blocks, exit-0-by-default
  posture.

## In Scope (proposed)

- A per-ecosystem **SBOM generation** stage using the *proper* generators
  (`cargo-cyclonedx`, `cyclonedx-npm`, `cyclonedx-gomod`,
  `cyclonedx-py`) — distinct from the attribution kit's licence scanners,
  which do not emit CycloneDX.
- A **merged dependency graph** fed into Anvil's graph/witness layer for
  mapping, provenance, and reverse-dependency queries.
- **New-edges-only** dependency-graph diffing + **L4 policy gating**
  (licence / vuln / new-edge) consistent with Anvil's warnings-over-blocks
  posture.
- **Release-time attestation**: customer-facing SBOM + SLSA provenance.
- A possible per-block `tool = "cyclonedx-*"` variant in the
  attribution kit, if a consumer wants SBOM output from that pipeline.

## Out of Scope

- **Licence attribution markdown** — owned by `attribution-pipeline-v3`
  (this module consumes/extends it, does not replace it).
- **Vulnerability database hosting** — correlate against OSV/advisory
  feeds; do not host them. `cargo audit` / `pnpm audit` remain the
  point tools (`security.aps.md`).
- **Bundled binaries** that are not in any lockfile (OpenSSH, FFmpeg) —
  attributed via `attribution-pipeline-v3` ATTRIB-004's hand-maintained
  inventory; an SBOM tool cannot discover them.

## Interfaces

**Depends on:** Anvil graph/witness infrastructure (must be able to
ingest a dependency graph); the `attribution-pipeline-v3` kit (shares
ecosystem coverage + the `licences.toml` allow-list); per-ecosystem
CycloneDX generators.

**Exposes:** a merged CycloneDX SBOM per release; a dependency-graph feed
for mapping/policy; SLSA provenance attestation.

## Prerequisites

- Anvil's graph layer can consume a dependency graph (the gating
  dependency — this is why the module is Proposed, not Ready).
- A decision on where SBOM generation runs (release pipeline vs a
  dedicated CI stage) and how it is witnessed.

## Open Questions

- Graph schema: reuse the existing witness/protection graph, or a
  dedicated dependency-graph store?
- Per-release vs per-commit SBOM cadence.
- Whether new-edge policy is advisory-only (warnings-over-blocks) or can
  gate release.
- Whether the attribution kit grows a `tool = "cyclonedx-*"` per-block
  variant, or SBOM generation stays entirely in this module's stage.

## Work Items

This module is **Proposed**; tasks are sketched, not Ready. They become
Ready once the graph-ingestion prerequisite lands.

### SCA-001: Design the SBOM generation + merge stage

- **Status:** Proposed
- **Intent:** Decide the per-ecosystem CycloneDX generators, the merge
  step, and where it runs (release pipeline vs CI stage).
- **Expected Outcome:** A design doc + APS sub-tasks; no code until the
  graph-ingestion prerequisite is confirmed.
- **Validation:** Design doc reviewed; it names the per-ecosystem
  CycloneDX generators, the merge step, and where the stage runs, and the
  follow-on APS sub-tasks exist (manual review).

### SCA-002: Dependency-graph ingestion + mapping

- **Status:** Proposed
- **Intent:** Feed the merged SBOM's components + edges into Anvil's
  graph/witness layer for mapping and provenance queries.
- **Expected Outcome:** Design + sub-tasks; gated on the graph layer.
- **Validation:** Design reviewed; it shows the merged SBOM's components
  and edges loading into the graph/witness layer with provenance queries
  resolving (manual review, pending graph-layer prerequisite).

### SCA-003: New-edges-only dependency policy (L4)

- **Status:** Proposed
- **Intent:** Baseline the dependency graph; warn on new edges / licence
  changes / known-vuln versions via L4 policy (warnings-over-blocks).
- **Expected Outcome:** Design + sub-tasks; gated on SCA-002.
- **Validation:** Design reviewed; it shows a baselined dependency graph
  emitting warnings on new edges / licence changes / known-vuln versions
  under L4 policy at exit 0 (manual review, gated on SCA-002).

## Notes

Recorded 2026-05-25 as the home for the deferred `ATTRIB-005` CycloneDX
direction, after confirming the shipped multi-block attribution
architecture (per-block markdown drivers) made a canonical-CycloneDX
*intermediate inside the attribution pipeline* both unnecessary (the
dispatcher already merges by splicing) and ill-fitting (the chosen
licence scanners do not emit CycloneDX). The strategic value of
CycloneDX is dependency mapping + supply-chain policy, which belongs
here and is gated on Anvil's graph infrastructure.
