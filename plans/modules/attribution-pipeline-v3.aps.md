<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Attribution Pipeline v3

| ID     | Owner | Status |
| ------ | ----- | ------ |
| ATTRIB | —     | Draft  |

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
   `accepted = [...]` and (when present) `deny.toml`'s
   `[licenses].allow = [...]` will drift. The current comment
   "keep this list in sync with deny.toml" concedes the smell.
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
- Future `deny.toml` (when added under `security.aps.md`) — v3.3
  generates the allow-list block of that file from the same canonical
  source.

**Exposes:**

- A reusable starter kit other EddaCraft / arkahna repos can adopt
  without re-implementing the marker-splice + drift-gate plumbing.
- A multi-language attribution pipeline anvil itself can grow into as
  it adds non-Rust ecosystems (e.g. the Hono API, the docs sites'
  Node deps if their attribution surface ever matters).
- A documented contract for the BEGIN/END marker block format,
  callable from any project regardless of ecosystem.

## Prerequisites

- Confirm the actual-vs-design discrepancy notes in
  `plans/specs/2026-04-23-rustnx-completion-design.md` are still
  accurate (path, CI job name, `anvil licenses` subcommand
  behaviour).
- Decide where the starter kit lives: `docs/guides/...` (read-only
  documentation), `tools/starters/...` (vendored copy other repos
  fetch), or a sibling repo (`acknowledgements-starter`). Each has
  different update-propagation semantics.
- Confirm CycloneDX as the intermediate-of-choice (vs a homegrown
  JSON / TOML schema). Industry standard, supported by every modern
  ecosystem's licence tooling, gives future-vuln-scanning for free
  if/when that lands.
- Owner named.

## Ready Checklist

Change status to **Ready** when:

- [ ] Discrepancy notes confirmed (or updated) in the design spec.
- [ ] Starter-kit location decided and recorded (ADR or design note).
- [ ] Intermediate format chosen (CycloneDX vs custom).
- [ ] First downstream consumer identified beyond anvil itself
      (e.g. `little-termi` is already using a hand-ported copy and is
      a natural validator).
- [ ] Decision recorded on whether `deny.toml` is part of v3 scope
      (vs deferred to a follow-up under `security.aps.md`).
- [ ] Owner named.

## Tasks

Tasks will be defined when this module moves to Ready. Anticipated:

- ATTRIB-001: Document the marker-splice contract and the existing
  generator's invariants in a stable reference (the starter-kit
  README that downstream consumers read).
- ATTRIB-002: Parameterise `tools/generate-acknowledgements.sh` so it
  reads project metadata from a config file instead of baked-in
  paths and command strings.
- ATTRIB-003: Extract the starter kit (script, config templates,
  template files, sample `ACKNOWLEDGEMENTS.md`, README) into the
  agreed location (`docs/guides/acknowledgements-starter/` or
  `tools/starters/...`).
- ATTRIB-004: Add a `bundled-binaries.toml` schema and ingest plugin;
  document it in the starter kit even though anvil itself does not
  bundle binaries today.
- ATTRIB-005: Add CycloneDX as a parallel intermediate alongside the
  cargo-about-direct path; render both through the same template
  layer.
- ATTRIB-006: Single-source-of-truth licence allow-list
  (`licences.toml` or chosen format) with an expander that produces
  the `about.toml.accepted` block (and, when `deny.toml` lands, its
  allow-list block). CI lints for drift.
- ATTRIB-007: Workspace-crate `license` field lint (hard fail rather
  than cargo-about warning).
- ATTRIB-008: Multi-block marker support — `<!-- BEGIN AUTO-GENERATED rust -->`,
  `<!-- BEGIN AUTO-GENERATED binaries -->`, `<!-- BEGIN AUTO-GENERATED gradle -->`
  etc., independently splice-able so partial regeneration is safe.
- ATTRIB-009: Validate the kit by porting it back into
  `little-termi` (replacing the hand-ported copy already there) and
  confirming both repos regenerate identically.
- ATTRIB-010: Update `docs/guides/release-doc-checklist.md` and the
  release runbook to reference the starter-kit location and the
  `--check` invocation downstream consumers should run pre-release.

## Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Scope creeps into supply-chain attestation / SBOM publication | High | v3 stops at the markdown attribution artefact; SBOM publication is explicitly out of scope and queued under `release-management` |
| CycloneDX intermediate is heavier than the project needs today | Medium | Keep `cargo-about` direct path supported; CycloneDX is additive, not forced |
| Hand-ported starter-kit copies drift from canonical | Medium | ATTRIB-009 validates the kit by re-porting it into a known consumer (little-termi); divergence is a CI failure there |
| Starter-kit location is read-only and downstream repos still copy-paste | Medium | If `docs/guides/...` is the chosen location, document the copy-update workflow explicitly; alternatively pick `tools/starters/...` and provide a `git subtree pull` recipe |
| Manual `## Thanks` section still rots over time | Low | CI requires the section exists with at least one entry per ecosystem represented in the auto-generated block; contents stay manual by design |
| `licences.toml` drift gate produces false-positive churn during adds | Low | Drift gate runs `--check` mode; surfaces an actionable diff rather than blocking the commit silently |

## Open Questions

- [ ] CycloneDX vs a homegrown JSON / TOML schema for the
      intermediate? CycloneDX is the standards answer; a homegrown
      schema is lighter for today's needs. Decide before
      ATTRIB-005.
- [ ] Where does the starter kit live: `docs/guides/...` (docs-only),
      `tools/starters/...` (vendorable), or a sibling repo
      (`eddacraft/acknowledgements-starter`)? Each has different
      propagation semantics — pick before ATTRIB-003.
- [ ] Should `deny.toml` integration be in v3, or queued under
      `security.aps.md` as a follow-up? The single-allow-list refactor
      in ATTRIB-006 only pays off once both consumers exist.
- [ ] Does `anvil licenses` (the CLI subcommand that
      `include_str!`-embeds the markdown) need a corresponding
      `anvil licenses --json` mode that returns the structured
      intermediate? If yes, that's a v3 deliverable; if no,
      defer to a future module.
- [ ] What's the canonical SPDX expression for "BSD-style as in
      OpenSSH" — does the bundled-binaries schema accept arbitrary
      SPDX strings, or a closed enum? Affects ATTRIB-004 schema
      rigour.
