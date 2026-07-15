<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Skill Packaging & Distribution

| ID    | Owner | Priority | Status  |
| ----- | ----- | -------- | ------- |
| SKPKG | —     | High     | In Progress |

## Purpose

Design how an Anvil-authored agent skill — starting with
`anvil-developer-functions` (`.claude/skills/anvil-developer-functions/`,
added via PR [#3064](https://github.com/eddacraft/anvil-001/pull/3064)) —
becomes a packageable, versioned artefact that customers can install and run
across more than one agent harness, not just Claude Code.

The approved beta embeds pinned, proprietary-but-customer-readable skill
snapshots in the Anvil binary and installs them through a managed, cross-agent
CLI flow. SKPKG-001..008 completed the first single-skill implementation via PR
#3328; SKPKG-009 reopens the module to validate the model with a second named
skill.

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

- A general-purpose skill marketplace or registry (explicitly out of scope
  for SKOBS too; revisit only if this design calls for it)
- Runtime governance/enforcement of installed skills (AGOV territory)
- Authoring the domain content of skills other than
  `anvil-developer-functions`; validating the packaging model with a second
  skill remains in scope, while its domain owner supplies the reviewed content

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

- **Status:** Done 2026-07-14 — live catalogue/code-env state re-verified and
  owner approved the revised beta design.
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

- **Status:** Done 2026-07-14 — ADR-018 now records the owner-approved beta
  boundary and ADR-106 defines the managed distribution contract.
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

- **Status:** Done 2026-07-14 — beta skill updates follow the Anvil binary
  release cadence; an independent channel is deferred until the skills become
  OSS or evidence justifies a second trust surface.
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

- **Status:** Done 2026-07-14 — one typed agent registry owns detection and
  capability metadata; skill discovery and MCP configuration remain separate
  capabilities because their verified client sets differ.
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

- **Status:** Done 2026-07-14 — the shared SKOBS schema defines
  `anvil-bundled`, including the provenance and drift semantics approved for
  the managed installer.
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

### SKPKG-006: Decide install scope — home directory vs project

- **Status:** Done 2026-07-14 — interactive setup offers both scopes and
  defaults to user-global for the beta; scriptable callers can select
  project-local installation explicitly.
- **Intent:** Decide whether `anvil skill install` defaults to the user's
  home directory (mirroring `anvil mcp install`'s default) or defaults to
  project-scoped, given a skill file is more naturally something a team
  commits and shares than per-user MCP client config.
- **Expected Outcome:** A decided default install scope, recorded in `##
  Decisions`, with the effect on SKOBS's machine/user/project scope model
  named.
- **Files:** `crates/anvil-cli/src/commands/mcp.rs`,
  `crates/anvil-cli/src/commands/mcp_config.rs`
- **Dependencies:** SKPKG-001
- **Validation:** Decision recorded in `## Decisions`
- **Confidence:** medium

### SKPKG-007: Decide the build-time content-embedding mechanism

- **Status:** Done 2026-07-14 — vendor the pinned portable snapshot under
  `crates/anvil-cli/assets/skills/` and embed it with `include_str!`; builds
  never fetch the private catalogue or depend on code-env.
- **Intent:** Decide whether `anvil skill install`'s embedded content comes
  from a new build-time step that pulls `code-env`'s already-emitted
  per-harness output into the `anvil` build, or embeds the canonical
  `SKILL.md` directly and defers per-harness adaptation to install time.
- **Expected Outcome:** A decided embedding mechanism, recorded in `##
  Decisions`, that the eventual implementation work item can build against.
- **Dependencies:** SKPKG-001
- **Validation:** Decision recorded in `## Decisions`
- **Confidence:** low

### SKPKG-008: Ship the managed beta skill installer

- **Status:** Done 2026-07-14 — the CLI embeds the pinned bundle and provides
  detected/scripted client selection, global/project scope, verification,
  provenance, idempotence, and managed-drift protection.
- **Intent:** Make `anvil-developer-functions` customer-installable from the
  shipped Anvil binary across verified Agent Skills harnesses.
- **Expected Outcome:** `anvil skill install` detects installed harnesses,
  supports interactive multi-select and scriptable client selection, offers
  global/project scope with global default, installs the embedded snapshot
  transactionally, records content provenance, no-ops when current, and
  refuses unmanaged or user-modified drift.
- **Files:** `crates/anvil-cli/assets/skills/`,
  `crates/anvil-cli/src/commands/skill.rs`, shared agent registry and CLI tests
- **Dependencies:** SKPKG-001..007, coordinates with MCPX-001..006
- **Validation:** targeted skill-install tests, repeated-install/drift fixtures,
  `cargo test -p eddacraft-anvil`, `pnpm docs:check`
- **Confidence:** medium

### SKPKG-009: Extend the managed bundle to multiple named skills

- **Status:** Proposed
- **Intent:** Validate the approved packaging model with
  `authoring-anvil-policy` without moving policy content ownership into SKPKG or
  creating a second client registry.
- **Expected Outcome:** A typed content/provenance registry selects among the
  existing `anvil-developer-functions` bundle and the OPAE-owned
  `authoring-anvil-policy` snapshot. Non-interactive installs require an
  explicit client and preserve the legacy no-name default; detection,
  destinations, and capabilities continue to come only from ADR-106's typed
  agent-client registry. Managed-drift, symlink, transactional install,
  verification, and provenance guarantees remain unchanged.
- **Files:** `crates/anvil-cli/src/commands/skill_catalogue.rs`,
  `crates/anvil-cli/src/commands/skill.rs`, embedded skill assets and installer
  tests.
- **Dependencies:** SKPKG-008, OPAE-017 content review
- **Validation:** `cargo test -p eddacraft-anvil --test skill_install` plus
  legacy/no-name, named-selection, explicit-client, provenance, drift, and
  symlink fixtures
- **Confidence:** medium

## Risks & Mitigations

| Risk | Mitigation |
|------|-----------|
| Design defaults to "Claude Code only" because that's the only harness in this repo today | Work item explicitly requires showing the model generalises to at least one other harness shape, even if that harness isn't implemented |
| IP-boundary question (open-source primitive vs closed-product artefact) gets skipped or assumed | Expected Outcome names ADR-018 explicitly and requires an explicit call, not silence |
| Scope creep into building a general skill marketplace | Out of Scope explicitly excludes marketplace/registry; SKOBS carries the same exclusion for consistency |

## Decisions

1. **Beta distribution:** skill content ships inside the proprietary Anvil
   binary as a pinned, customer-readable snapshot; eventual OSS publication is
   a later transition, not a beta prerequisite.
2. **Update cadence:** the beta snapshot updates with Anvil releases.
3. **Install scope:** interactive setup offers both and defaults to user-global.
4. **Client model:** one typed agent registry records independent skill and MCP
   capabilities; surfaces must not infer one from the other.
5. **Safety:** managed provenance and content hashes gate updates; unmanaged or
   user-modified destinations are never silently overwritten.
6. **Embedding:** `include_str!` over vendored assets; no build-time network or
   private-repository dependency.

## Designs

- [Skill packaging & distribution across agent harnesses](../specs/2026-07-02-skill-packaging-distribution.md)
  (Accepted, 2026-07-14) — SKPKG-001. Finds that the packaging manifest
  (`skill.meta.json`), the cross-agent catalogue, and the emission pipeline
  (`code-env`) already exist; the actual gap is a customer-reachable
  distribution channel. Proposes a new `anvil skill install --client <target>`
  CLI subcommand, sibling to the existing `anvil mcp install`, that embeds
  skill content in the closed `anvil` binary (ADR-018-aligned, no catalogue
  access required). Surfaces six follow-on decisions as SKPKG-002..007.

## Notes

### Provenance

Spawned 2026-07-02 from the `anvil-developer-functions` skill upload (PR
[#3064](https://github.com/eddacraft/anvil-001/pull/3064)) once the need to
package it for customers, across agent harnesses, was raised.

### Resolved 2026-07-14

The owner approved shipping the beta skill inside Anvil. The catalogue and
`code-env` were re-verified as design inputs, but neither is a build-time or
customer-install dependency.
