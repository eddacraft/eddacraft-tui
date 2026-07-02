<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Skill Packaging & Distribution

| ID    | Owner | Priority | Status  |
| ----- | ----- | -------- | ------- |
| SKPKG | —     | Medium   | Blocked |

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

- **Status:** Blocked — parked 2026-07-02 (owner: new work landing in the
  `eddacraft-skills` catalogue repo changes the ground this design stands on;
  see `## Notes`). Design doc drafted and self-reviewed but not yet sent for
  owner sign-off; two review-found defects (broken OQ-3/OQ-4 cross-reference,
  overstated `anvil mcp install` target-overlap claim) are noted but not
  fixed while parked — fix on resume, before requesting review.
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

### SKPKG-002: Ratify the ADR-018 IP-boundary call for distributed skills

- **Status:** Draft
- **Intent:** Get an owner (or Council/ADR) decision on whether customer-
  distributed skills as closed-product artefacts (embedded in the `anvil`
  binary, installed via `anvil skill install`, never via direct
  `eddacraft-skills` catalogue access) needs a formal ADR addendum to
  ADR-018, or whether sign-off on the design spec is sufficient precedent.
- **Expected Outcome:** A recorded decision (ADR addendum or an explicit
  accepted note on the spec) that future customer-facing skill work can cite
  without re-litigating the IP boundary each time.
- **Files:** `plans/decisions/018-product-ip-architecture.md` (possible
  addendum), `plans/specs/2026-07-02-skill-packaging-distribution.md`
- **Dependencies:** SKPKG-001
- **Validation:** Decision recorded and linked from both this module and
  ADR-018
- **Confidence:** low (owner call, not something to resolve unilaterally)

### SKPKG-003: Decide the skill-update cadence trade-off

- **Status:** Draft
- **Intent:** Decide whether bundling skill content with `anvil` binary
  releases (default per the design) is acceptable, or whether skill content
  needs a faster-moving update channel independent of CLI release cadence.
- **Expected Outcome:** An explicit decision, recorded in this module's
  `## Decisions` section, with the trade-off (binary-release cadence vs a
  second trust surface) named either way.
- **Dependencies:** SKPKG-001
- **Validation:** Decision recorded in `## Decisions`
- **Confidence:** low

### SKPKG-004: Reconcile target-harness sets between skill install and MCP install

- **Status:** Draft
- **Intent:** Decide which harness targets a first-cut `anvil skill install`
  supports, given the catalogue declares 4 (`claude`, `opencode`, `openclaw`,
  `codex`) but `anvil mcp install`'s `McpClient` enum only has 2 (`cursor`,
  `claude-code`), and whether the two enums should be unified or stay
  independent.
- **Expected Outcome:** A decided initial target set for `anvil skill
  install`, recorded here, with a call on `McpClient` unification.
- **Files:** `crates/anvil-cli/src/commands/mcp.rs`
- **Dependencies:** SKPKG-001
- **Validation:** Decision recorded in `## Decisions`
- **Confidence:** medium

### SKPKG-005: Land `SourceInfo.type: "anvil-bundled"` in the SKOBS manifest schema

- **Status:** Draft
- **Intent:** Add a fourth `SourceInfo.type` value (naming TBD) to
  `plans/specs/skill-manifest-schema.md` so `/skill-inventory` can
  distinguish skills materialised by `anvil skill install` from
  hand-authored/copied/symlinked ones, coordinated with the SKOBS module
  owner before SKOBS-002 goes Ready.
- **Expected Outcome:** `skill-manifest-schema.md` updated with the new
  `SourceInfo.type` value and SKOBS-002 acknowledges the addition.
- **Files:** `plans/specs/skill-manifest-schema.md`
- **Dependencies:** SKPKG-001, coordinates with SKOBS-002
- **Validation:** Schema doc updated; SKOBS module owner has acknowledged the
  change (comment or commit co-sign)
- **Confidence:** medium

## Risks & Mitigations

| Risk | Mitigation |
|------|-----------|
| Design defaults to "Claude Code only" because that's the only harness in this repo today | Work item explicitly requires showing the model generalises to at least one other harness shape, even if that harness isn't implemented |
| IP-boundary question (open-source primitive vs closed-product artefact) gets skipped or assumed | Expected Outcome names ADR-018 explicitly and requires an explicit call, not silence |
| Scope creep into building a general skill marketplace | Out of Scope explicitly excludes marketplace/registry; SKOBS carries the same exclusion for consistency |

## Decisions

_(none ratified yet — SKPKG-002/003/004 above are the pending decisions the
design spec surfaced; this section fills in as each is resolved)_

## Designs

- [Skill packaging & distribution across agent harnesses](../specs/2026-07-02-skill-packaging-distribution.md)
  (Draft, 2026-07-02) — SKPKG-001. Finds that the packaging manifest
  (`skill.meta.json`), the cross-agent catalogue, and the emission pipeline
  (`code-env`) already exist; the actual gap is a customer-reachable
  distribution channel. Proposes a new `anvil skill install --client <target>`
  CLI subcommand, sibling to the existing `anvil mcp install`, that embeds
  skill content in the closed `anvil` binary (ADR-018-aligned, no catalogue
  access required). Surfaces four follow-on decisions as SKPKG-002..005.

## Notes

### Provenance

Spawned 2026-07-02 from the `anvil-developer-functions` skill upload (PR
[#3064](https://github.com/eddacraft/anvil-001/pull/3064)) once the need to
package it for customers, across agent harnesses, was raised.

### Parked 2026-07-02

Owner call: new work is landing in the `eddacraft-skills` catalogue repo
that the design's "What already exists" section leans on (the manifest
schema, the `code-env` emission pipeline, `install.sh`'s target detection).
Continuing design work now risks grounding decisions in a snapshot of that
repo that's about to change. Parked rather than continued or abandoned —
resume by re-reading `eddacraft-skills` state fresh (don't trust this
design's "What already exists" findings without re-verifying them) before
picking SKPKG-001 back up.
