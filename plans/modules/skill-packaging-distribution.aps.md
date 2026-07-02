<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Skill Packaging & Distribution

| ID    | Owner | Priority | Status |
| ----- | ----- | -------- | ------ |
| SKPKG | —     | Medium   | Draft  |

## Purpose

Design how an Anvil-authored agent skill — starting with
`anvil-developer-functions` (`.claude/skills/anvil-developer-functions/`,
added via PR [#3064](https://github.com/eddacraft/anvil-001/pull/3064)) —
becomes a packageable, versioned artefact that customers can install and run
across more than one agent harness, not just Claude Code.

Today skills live as loose files under `.claude/skills/<name>/` with no
packaging manifest, no version pin, no install/update path outside this repo,
and no defined behaviour for harnesses whose skill/tool-definition format
differs from Claude Code's (`SKILL.md` + frontmatter). This module produces
the design before any implementation work is scoped.

## In Scope

- Packaging model for a single skill: what ships (source `SKILL.md` +
  references + metadata vs a compiled/bundled artefact), and where the
  package boundary sits relative to `.claude/skills/anvil-developer-functions/`
- Cross-agent portability: what in a Claude Code skill is
  harness-independent (prose instructions, reference docs) vs
  Claude-Code-specific (frontmatter schema, `SKILL.md` conventions, tool
  invocation shape), and what translation or adapter layer is needed for
  other agent harnesses (e.g. Codex, OpenCode-style config, MCP-only clients)
- Versioning and update model for a distributed skill package (semver? content
  hash pin? update channel?), consistent with existing distribution precedent
  (DISTRIB, ATTRIB-017 starter-repo release pattern)
- IP/licensing boundary per ADR-018 (`plans/decisions/018-product-ip-architecture.md`):
  is a customer-distributed skill part of the closed product, or does it need
  to sit on the OSS/primitive side of the boundary — and if closed, what
  distribution channel (bundled with the `anvil` binary vs a separate
  artefact) fits the free-tier-not-open-source model
- Coordination points with SKOBS (skill-discovery-observability): a
  customer-installed skill is inventory the same way as any other, so the
  packaging manifest and SKOBS's manifest schema (SKOBS-002) should not
  diverge

## Out of Scope

- Implementation of any packaging tooling (this module produces a design;
  implementation is follow-on work items once the design is reviewed)
- A general-purpose skill marketplace or registry (explicitly out of scope
  for SKOBS too; revisit only if this design calls for it)
- Runtime governance/enforcement of installed skills (AGOV territory)
- Packaging skills other than `anvil-developer-functions` (this module scopes
  the *model*, generalising it is a validation step, not new scope)

## Interfaces

**Depends on:**

- ADR-018 (product/IP architecture) — governs whether a distributed skill is
  closed-product or OSS-surface
- SKOBS-002 (skill manifest schema) — packaging manifest should align rather
  than fork a second schema

**Exposes:**

- A design doc under `designs/` once SKPKG-001 completes, to be linked back
  into this module's `## Designs` section

## Work Items

### SKPKG-001: Cross-agent skill packaging design

- **Intent:** Produce a design document that answers how
  `anvil-developer-functions` (and future customer-facing skills) get
  packaged, versioned, and distributed so they work across multiple agent
  harnesses, not just Claude Code.
- **Expected Outcome:** A design doc at
  `designs/YYYY-MM-DD-skill-packaging-distribution.design.md` that covers, at
  minimum: (1) the packaging artefact shape and manifest fields, (2) which
  parts of a skill are harness-portable vs Claude-Code-specific and how the
  gap is bridged for at least one other harness shape, (3) a versioning/update
  model, (4) an explicit call on the ADR-018 IP boundary question for
  distributed skills, and (5) how the packaging manifest relates to SKOBS's
  inventory manifest. Open questions the design can't resolve are logged as
  Draft follow-on work items in this module, not silently decided.
- **Non-scope:** Building any packaging tooling; deciding on a specific
  second harness to support (the design should show the model generalises,
  not commit to one)
- **Files:**
  - `designs/YYYY-MM-DD-skill-packaging-distribution.design.md`
  - This module (`## Designs` link, Draft follow-on items)
- **Dependencies:** —
- **Validation:** Design doc reviewed and accepted (owner sign-off, or
  Council/ADR review if the IP-boundary call warrants it); this module's
  `## Designs` section links it and lists the resulting follow-on work items
- **Confidence:** medium

## Risks & Mitigations

| Risk | Mitigation |
|------|-----------|
| Design defaults to "Claude Code only" because that's the only harness in this repo today | Work item explicitly requires showing the model generalises to at least one other harness shape, even if that harness isn't implemented |
| IP-boundary question (open-source primitive vs closed-product artefact) gets skipped or assumed | Expected Outcome names ADR-018 explicitly and requires an explicit call, not silence |
| Scope creep into building a general skill marketplace | Out of Scope explicitly excludes marketplace/registry; SKOBS carries the same exclusion for consistency |

## Decisions

_(none yet — this module exists to produce the first one via SKPKG-001)_

## Designs

_(link the design doc here once SKPKG-001 completes)_

## Notes

### Provenance

Spawned 2026-07-02 from the `anvil-developer-functions` skill upload (PR
[#3064](https://github.com/eddacraft/anvil-001/pull/3064)) once the need to
package it for customers, across agent harnesses, was raised.
