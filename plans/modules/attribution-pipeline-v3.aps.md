<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Attribution Pipeline v3

| ID     | Owner      | Status      |
| ------ | ---------- | ----------- |
| ATTRIB | joshuaboys | In Progress |

**Last reviewed:** 2026-04-26

## Purpose

Evolve the third-party-attribution pipeline that currently lives in
`tools/generate-acknowledgements.sh` + `about.toml` + `about.hbs` +
`ACKNOWLEDGEMENTS.md` into a generic, multi-language, low-manual-overhead
artefact that other repositories can lift wholesale.

The v1 pipeline shipped with the Rust CLI release rehearsal (see
`plans/specs/2026-04-23-rustnx-completion-design.md` § Discrepancy Notes
for the actual-vs-design summary). The v2 work added the CI freshness
gate (`acknowledgements-diff`) and the `anvil licenses` subcommand that
embeds `ACKNOWLEDGEMENTS.md` via `include_str!`. **v3** is this module:
make it portable, multi-language, and remove the remaining manual
duplications.

**Problem this solves.** The pipeline as it stands has three rough
edges that have been called out during port-it-into-another-project
exercises:

1. **Hard-coded paths.** The script bakes
   `crates/anvil-cli/Cargo.toml` and `pnpm run licenses:generate`
   strings into the bash. Lifting it into a sibling repo
   (e.g. `little-termi`, `arkahna-*`) means hand-editing the script.
2. **Two sources of truth for the licence allow-list.** `about.toml`'s
   `accepted = [...]` and `deny.toml`'s `[licenses].allow = [...]`
   already exist side-by-side in this repo and will drift. The
   comment in `about.toml` ("keep this list in sync with deny.toml")
   concedes the smell.
3. **Single-language scope.** The pipeline only attributes Rust crates.
   Repos that ship Kotlin/Android, Node, Python, or bundled
   third-party binaries (OpenSSH, Mosh, FFmpeg, ...) need parallel
   pipelines wired together.

**v3 deliverable.** A reusable starter kit — versioned, documented,
and independently testable — that any of the EddaCraft / arkahna
repositories can adopt with one config file (manifest list + project
metadata) instead of editing the bash. As a side-effect, anvil itself
upgrades from "single Rust block" to "Rust + bundled-binaries" coverage
and removes the `accepted`-list duplication once `deny.toml` is part of
the same source of truth.

## In Scope

**v3.1 — Generic starter kit (extraction & portability)**

- Promote `tools/generate-acknowledgements.sh` to a parameterised
  generator that reads from a single config file (TOML or JSON) listing
  per-ecosystem manifests, target attribution paths, and rendering
  options. Eliminate the hard-coded `crates/anvil-cli/Cargo.toml` and
  `pnpm run licenses:generate` strings.
- Extract the kit (`about.toml`, `about.hbs`, the generator, a sample
  `ACKNOWLEDGEMENTS.md`, and a README explaining the BEGIN/END marker
  contract) into a referenceable location — either a new
  `docs/guides/acknowledgements-starter/` directory checked into anvil
  itself, or a new `tools/starters/acknowledgements/` inside this repo
  that downstream consumers `git subtree pull` from.
- Document the contract: marker syntax, idempotency invariants, the
  `--check` exit-code semantics, and what guarantees the generator
  makes (atomic write, empty-output guard, marker-count validation).

**v3.2 — Multi-language support**

- Add a per-ecosystem ingest stage. Each ecosystem emits a normalised
  intermediate (CycloneDX SBOM JSON preferred where the tooling
  supports it; otherwise a plain TOML inventory of
  `{ name, version, licence_spdx, source_url }`).
- A single render stage consumes the merged intermediate and emits the
  marker-spliced markdown block.
- Initial ecosystems anvil cares about today: **Rust** (via
  `cargo about` or `cargo cyclonedx`); **bundled native binaries** (via
  a hand-maintained `tools/bundled-binaries.toml` that lists each
  third-party binary anvil ships, with version + source URL + SPDX
  expression). Future ecosystems opt in by adding an ingest plugin —
  Gradle (`licensee` / `cyclonedx-gradle-plugin`), Node
  (`license-checker` / `cyclonedx-npm`), Python (`pip-licenses` /
  `cyclonedx-python`).
- Each ecosystem renders into its own `<!-- BEGIN AUTO-GENERATED rust -->`
  / `<!-- BEGIN AUTO-GENERATED binaries -->` etc. block so a partial
  failure (e.g. one tool unavailable in a contributor's local env)
  does not clobber unrelated content.

**v3.3 — Remove manual duplications**

- Single-source-of-truth licence allow-list. Define
  `licences.toml` (or equivalent) at repo root with the canonical
  allow-list; expand it into `about.toml`-shaped and
  `deny.toml`-shaped inputs at script-run time. CI lints both for
  drift against the canonical file.
- `bundled-binaries.toml` becomes the single source of truth for
  attribution of any binary anvil ships that isn't a Cargo crate
  — eliminates the "remember to update the markdown when we bump
  OpenSSH" risk for downstream consumers, even though anvil itself
  doesn't bundle native binaries today.
- A small CI lint that requires every workspace crate to declare a
  `license` field (so cargo-about's "no license field" warning becomes
  a hard error rather than easy-to-miss noise).
- Document the hand-curated `## Thanks` section as deliberate
  curation — the remaining manual content is a feature, not a
  duplication. CI only enforces it exists, not its contents.

## Out of Scope

- **Vulnerability scanning.** SBOMs enable it, but `cargo audit` /
  `pnpm audit` already cover this and live elsewhere
  (`security.aps.md`).
- **Licence-policy decisions.** Whether GPLv3 / AGPL / proprietary
  licences are acceptable is a `security.aps.md` / legal concern;
  this module reports licences, it does not adjudicate them.
- **SBOM publication / SLSA attestation.** Producing a customer-facing
  SBOM artefact alongside releases is a follow-up
  (`release-management` or a new `supply-chain-attestation` module).
- **Replacing `cargo-about`.** v3 keeps `cargo-about` as a supported
  ingest path; the SBOM intermediate is additive, not a forced
  migration.
- **Per-OS variant attribution.** Repos that ship per-architecture
  builds with subtly different dep graphs (musl vs glibc, Android
  ABIs) get one merged inventory across targets, not per-target
  files. cargo-about already supports the `targets = [...]` list for
  this; v3 inherits that behaviour.

## Interfaces

**Depends on:**

- Existing `tools/generate-acknowledgements.sh`, `about.toml`,
  `about.hbs`, `ACKNOWLEDGEMENTS.md` — v3 evolves these in place,
  not replaces them.
- `cargo-about` (current ingest), pinned in
  `.github/workflows/rust.yml`. v3 may add `cargo-cyclonedx` as a
  parallel ingest option.
- `crates/anvil-cli` — the `anvil licenses` subcommand consumes the
  output and must continue to work after the file format evolves.
- Existing `deny.toml` at repo root (enforced via the `cargo-deny`
  CI job, owned under `security.aps.md`) — v3.3 generates or derives
  its `[licenses].allow` block from the same canonical source rather
  than introducing `deny.toml` itself.

**Exposes:**

- A reusable starter kit other EddaCraft / arkahna repos can adopt
  without re-implementing the marker-splice + drift-gate plumbing.
- A multi-language attribution pipeline anvil itself can grow into as
  it adds non-Rust ecosystems (e.g. the Hono API, the docs sites'
  Node deps if their attribution surface ever matters).
- A documented contract for the BEGIN/END marker block format,
  callable from any project regardless of ecosystem.

**Known downstream consumers:**

- `little-termi` — already runs a hand-ported v1 copy; primary
  validation target for ATTRIB-009.
- The Anvil VS Code extension (Node ecosystem) — will adopt the kit
  once published.
- Owner's future public projects — drives the
  ATTRIB-011 milestone (extract to a public sibling repo so non-anvil
  consumers don't have to copy out of a private repo).

## Prerequisites

All resolved as of 2026-04-25. See [Decisions](#decisions) below for the
chosen answers and rationale.

## Decisions

Recorded on transition to Ready (2026-04-25).

| Question | Decision | Rationale |
| --- | --- | --- |
| Starter-kit location | `tools/starters/acknowledgements/` inside this repo | Vendor-friendly: downstream repos `git subtree pull` for updates instead of copy-paste rot. `docs/guides/...` would be read-only and force manual reconciliation; a sibling repo is overkill for the current consumer set. **Public-extract is queued as ATTRIB-011** since anvil-001 is private and the VS Code extension + future public projects can't `git subtree` from it. |
| Intermediate format | CycloneDX SBOM JSON | Standards-compliant; every modern licence tool emits it (`cargo-cyclonedx`, `cyclonedx-npm`, `cyclonedx-gradle-plugin`, `cyclonedx-python`); gives future vuln-scanning leverage for free. A homegrown TOML schema is lighter today but pays back the cost the moment a fourth ecosystem joins. |
| `deny.toml` in v3 scope? | Yes — included in v3 | ATTRIB-006 (single-source-of-truth licence allow-list) only earns its keep when both `about.toml` and `deny.toml` consume it. Splitting them across modules means doing the refactor twice. |
| First downstream consumer | `little-termi` (existing hand-port; primary validation target for ATTRIB-009). Plus the Anvil VS Code extension and the owner's future public projects as anticipated consumers — these drive the ATTRIB-011 public-extract milestone. | — |
| Owner | `joshuaboys` | — |
| Discrepancy notes accuracy | Confirmed accurate (verified 2026-04-25) | All claims in `plans/specs/2026-04-23-rustnx-completion-design.md`'s shipped-implementation note still hold: file path (`ACKNOWLEDGEMENTS.md`), generator (`tools/generate-acknowledgements.sh` wrapping `cargo about generate` scoped to `crates/anvil-cli/Cargo.toml`), CLI surface (`anvil licenses` via `include_str!`), CI gate (`acknowledgements-diff`), and release-pipeline publish (`.github/workflows/release.yml` mirrors the file to `eddacraft/anvil`). |

Two open questions remain — neither blocks Ready and both are expected to
resolve naturally during their respective tasks:

- `anvil licenses --json` — whether the CLI subcommand grows a structured
  output mode emitting the merged CycloneDX intermediate. Decide during
  ATTRIB-005; defer to a future module if the use case doesn't surface.
- SPDX expression rigour for bundled binaries — whether
  `tools/bundled-binaries.toml` accepts arbitrary SPDX strings or a
  closed enum. Decide during ATTRIB-004 schema design once we have a
  concrete first binary to attribute.

## Ready Checklist

All items satisfied; see [Decisions](#decisions) above.

- [x] Discrepancy notes confirmed (or updated) in the design spec.
- [x] Starter-kit location decided and recorded (ADR or design note).
- [x] Intermediate format chosen (CycloneDX vs custom).
- [x] First downstream consumer identified beyond anvil itself
      (`little-termi`; plus VS Code extension + future public projects).
- [x] Decision recorded on whether `deny.toml` is part of v3 scope
      (yes — included in v3).
- [x] Owner named (`joshuaboys`).

## Tasks

A v1.5 reference draft (per-language split, env-var-parameterised script,
TS recipe sketch) lives at `~/scratch/anvil-attribution-v1.5-draft/` on
the owner's machine. It predates the v3 design (single config file,
multi-block markers, CycloneDX intermediate) and is reference-only — its
marker-contract documentation may seed ATTRIB-001 verbatim, but the
script and per-language structure are superseded by ATTRIB-002/003/008.

### ATTRIB-001: Document the marker-splice contract

- **Status:** Complete
- **Intent:** Stable reference that downstream consumers read before adopting the kit.
- **Expected Outcome:** README in `tools/starters/acknowledgements/` covers marker syntax, idempotency invariants, `--check` exit-code semantics, atomic-write / empty-output / marker-count guarantees.
- **Validation:** README exists; `markdownlint` clean; cross-references resolve.

### ATTRIB-002: Parameterise the generator via a config file

- **Status:** Complete
- **Intent:** Eliminate hard-coded `crates/anvil-cli/Cargo.toml` and `pnpm run licenses:generate` strings from the bash.
- **Expected Outcome:** Generator reads `attribution.toml` (per-ecosystem manifests + project metadata) instead of baked-in paths. Anvil's existing config lives at repo root.
- **Validation:** `tools/generate-acknowledgements.sh --check` passes against unchanged anvil graph after the refactor; no project-specific strings remain in the script.

### ATTRIB-003: Extract starter kit to `tools/starters/acknowledgements/`

- **Status:** Complete
- **Intent:** Vendor the kit at its agreed canonical location so downstream repos can `git subtree pull` without copy-paste rot.
- **Expected Outcome:** Directory contains the parameterised script, `attribution.toml.example`, template files, `ACKNOWLEDGEMENTS.md.template`, README, and the GitHub Actions snippet. Self-contained: no imports from the rest of the repo.
- **Validation:** `tar -czf` of the directory extracts cleanly; a fresh repo can adopt by copy + `attribution.toml` edit only.

### ATTRIB-004: `bundled-binaries.toml` schema and ingest plugin

- **Status:** Pending
- **Intent:** Allow attribution of third-party binaries that aren't Cargo crates (OpenSSH, Mosh, FFmpeg, ...).
- **Expected Outcome:** Schema documented in starter kit; ingest plugin emits CycloneDX intermediate from a hand-maintained TOML inventory. Anvil ships an empty inventory today.
- **Validation:** Sample `bundled-binaries.toml` with one fixture entry round-trips through the generator and renders into a `binaries` block.
- **Open:** Whether the schema accepts arbitrary SPDX expressions or a closed enum — decide during implementation.

### ATTRIB-005: CycloneDX intermediate alongside cargo-about-direct

- **Status:** Pending
- **Intent:** Add CycloneDX as the canonical inter-ecosystem format; keep cargo-about-direct path as a faster-but-Rust-only fallback.
- **Expected Outcome:** Generator can run in two modes: CycloneDX (multi-ecosystem merge) and direct (today's behaviour). Output identical for the Rust-only case.
- **Validation:** `tools/generate-acknowledgements.sh --check` passes in both modes against the current anvil graph.
- **Open:** Whether `anvil licenses --json` grows to expose the merged intermediate — decide during this task.

### ATTRIB-006: Single-source-of-truth licence allow-list

- **Status:** Pending
- **Intent:** Eliminate the `about.toml.accepted` ↔ `deny.toml.[licenses].allow` drift smell.
- **Expected Outcome:** Canonical `licences.toml` at repo root; expander script produces both `about.toml`-shaped and `deny.toml`-shaped fragments at run time. CI lints for drift.
- **Validation:** Contributor adds a licence to `licences.toml`; both consumers reflect it without further edits. Removing from one without `licences.toml` triggers CI failure.

### ATTRIB-007: Workspace-crate `license` field lint

- **Status:** In Progress
- **Intent:** Make cargo-about's "no license field" warning a hard error so missing fields can't slip past review.
- **Expected Outcome:** CI step fails when a workspace crate lacks a `license` (or `license-file`) field. Existing crates already comply (per RUSTNX-009); this prevents regression.
- **Validation:** Test crate without a `license` field triggers the lint locally and in CI.

### ATTRIB-008: Multi-block marker support

- **Status:** Pending
- **Intent:** Allow `<!-- BEGIN AUTO-GENERATED rust -->`, `<!-- BEGIN AUTO-GENERATED binaries -->`, `<!-- BEGIN AUTO-GENERATED node -->` etc. to coexist in one file, each independently splice-able so a partial failure can't clobber unrelated content.
- **Expected Outcome:** Generator accepts a block name; splices only the named block; preserves all other blocks verbatim. Marker count gate validates per-block (one BEGIN, one END each).
- **Validation:** Two-block fixture round-trips through partial regeneration without touching the other block.

### ATTRIB-009: Port the kit back into `little-termi`

- **Status:** Pending
- **Intent:** Replace the hand-ported v1 copy in `little-termi` with the v3 starter kit; confirm both repos regenerate identically.
- **Expected Outcome:** `little-termi` adopts the kit via `git subtree pull` (or copy if subtree isn't workable) and runs `--check` clean. Divergence is a `little-termi` CI failure.
- **Validation:** Both repos pass their respective `acknowledgements-diff` jobs after the port.

### ATTRIB-010: Update release runbook + doc checklist

- **Status:** Pending
- **Intent:** Reference the starter-kit location and the `--check` invocation downstream consumers should run pre-release.
- **Expected Outcome:** `docs/guides/release-doc-checklist.md` mentions the kit; release runbook calls out the gate.
- **Validation:** Release runbook references resolve; doc lint passes.

### ATTRIB-011: Mirror starter kit to a public sibling repo

- **Status:** Pending
- **Intent:** Make the kit usable from public projects (the Anvil VS Code extension, the owner's future public projects) that can't `git subtree` from the private `anvil-001` repo.
- **Expected Outcome:** New public repo (proposed: `eddacraft/acknowledgements-starter`) mirrors `tools/starters/acknowledgements/` with a one-shot or scheduled mirror job. Public repo carries its own README pointing at this module for design history.
- **Validation:** Public repo exists; mirror job succeeds; one external project (anvil VS Code extension) consumes it.

## Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Scope creeps into supply-chain attestation / SBOM publication | High | v3 stops at the markdown attribution artefact; SBOM publication is explicitly out of scope and queued under `release-management` |
| CycloneDX intermediate is heavier than the project needs today | Medium | Keep `cargo-about` direct path supported; CycloneDX is additive, not forced |
| Hand-ported starter-kit copies drift from canonical | Medium | ATTRIB-009 validates the kit by re-porting it into a known consumer (little-termi); divergence is a CI failure there |
| Public consumers (VS Code extension, future public projects) can't `git subtree` from a private repo | Medium | ATTRIB-011 mirrors the kit to a public sibling repo; design history stays in this private module |
| Manual `## Thanks` section still rots over time | Low | CI requires the section exists with at least one entry per ecosystem represented in the auto-generated block; contents stay manual by design |
| `licences.toml` drift gate produces false-positive churn during adds | Low | Drift gate runs `--check` mode; surfaces an actionable diff rather than blocking the commit silently |

## Open Questions

All Ready-blocking questions resolved in [Decisions](#decisions).
The two tactical questions deferred to their respective tasks
(`anvil licenses --json` mode, SPDX rigour for bundled binaries) live
inline against ATTRIB-005 and ATTRIB-004.
