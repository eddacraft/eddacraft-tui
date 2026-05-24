<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Attribution Pipeline v3

| ID     | Owner      | Status      |
| ------ | ---------- | ----------- |
| ATTRIB | joshuaboys | In Progress |

**Last reviewed:** 2026-05-24

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
upgrades from "single Rust block" to "Rust + Node devtools" coverage,
unlocks Go and Python attribution for downstream consumers, and removes
the `accepted`-list duplication once `deny.toml` is part of the same
source of truth.

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

- Generator gains a `[[blocks]]` array in `attribution.toml`. Each block
  names an ecosystem and the manifest to walk; the generator dispatches
  per-block to an ecosystem driver under
  `tools/starters/acknowledgements/drivers/`. Flat `[rust]` top-level
  schema auto-promotes to a single unnamed block for back-compat with
  existing consumers (Anvil today, eddacraft-tui, future little-termi port).
- Initial ecosystem drivers — the four that cover ~80% of downstream
  shipping artefacts:
  - **Rust** (`cargo-about`) — extracted from the current single-driver
    body; no behaviour change.
  - **Node** (`license-checker`) — first new driver; pnpm-workspace
    friendly via the "one block per shipping `package.json`" pattern.
  - **Go** (`go-licenses`) — Google's official tool; binary-import-path
    scoped, same shipping-artefact pattern as Rust.
  - **Python** (`pip-licenses`) — runs against a consumer-supplied
    pre-built venv to avoid the kit shipping `uv` / `poetry` / `pdm`
    opinions.
- Each driver ships its own preflight (tool installed, state populated),
  strict-license check (parity with `cargo about --fail`), and
  deterministic-render contract (sorted output for idempotency under
  `--check`).
- Each block renders into its own
  `<!-- BEGIN AUTO-GENERATED <name> -->` /
  `<!-- END AUTO-GENERATED <name> -->` pair so a partial failure
  (e.g. one tool unavailable in a contributor's local env, or one
  ecosystem's strict-license check tripping) does not clobber unrelated
  content. Per-block marker-count gate; a single whole-file atomic
  rename at the end of the splice loop preserves the no-partial-clobber
  invariant (driver failure mid-loop leaves the on-disk target
  untouched). Full dispatcher contract in the v3.2 spec.
- `tools/bundled-binaries.toml` (ATTRIB-004) remains a separate ingest
  stream for binaries that aren't a package manager's deps.
- CycloneDX SBOM JSON stays a queued option as an alternative `tool=`
  selection per ecosystem (e.g. `tool = "cyclonedx-npm"` instead of
  `tool = "license-checker"`); not on the v3.2 critical path.

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
- **Java/Kotlin, Ruby, and Swift ecosystem drivers.** `licensee` /
  `license_finder` / `licenseplist` are well-known tools and the
  driver-dispatch shape would accommodate them, but no current
  downstream consumer ships these ecosystems. Reserved for re-decision
  if a real consumer surfaces a need.

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
- `eddacraft-tui` (public, Rust + cargo-about) — first consumer of
  the published mirror under ATTRIB-011.
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
| Starter-kit location | `tools/starters/acknowledgements/` inside this repo | Vendor-friendly: downstream repos `git subtree pull` for updates instead of copy-paste rot. `docs/guides/...` would be read-only and force manual reconciliation; a sibling repo is overkill for the current consumer set. **Public-extract is queued as ATTRIB-011** since anvil-001 is private and `eddacraft-tui` + future public projects can't `git subtree` from it. |
| Intermediate format | CycloneDX SBOM JSON | Standards-compliant; every modern licence tool emits it (`cargo-cyclonedx`, `cyclonedx-npm`, `cyclonedx-gradle-plugin`, `cyclonedx-python`); gives future vuln-scanning leverage for free. A homegrown TOML schema is lighter today but pays back the cost the moment a fourth ecosystem joins. |
| `deny.toml` in v3 scope? | Yes — included in v3 | ATTRIB-006 (single-source-of-truth licence allow-list) only earns its keep when both `about.toml` and `deny.toml` consume it. Splitting them across modules means doing the refactor twice. |
| First downstream consumer | `little-termi` (existing hand-port; primary validation target for ATTRIB-009). Plus `eddacraft-tui` (public Rust CLI; first consumer of the ATTRIB-011 public mirror) and the owner's future public projects as anticipated consumers — these drive the ATTRIB-011 public-extract milestone. | — |
| Owner | `joshuaboys` | — |
| Discrepancy notes accuracy | Confirmed accurate (verified 2026-04-25) | All claims in `plans/specs/2026-04-23-rustnx-completion-design.md`'s shipped-implementation note still hold: file path (`ACKNOWLEDGEMENTS.md`), generator (`tools/generate-acknowledgements.sh` wrapping `cargo about generate` scoped to `crates/anvil-cli/Cargo.toml`), CLI surface (`anvil licenses` via `include_str!`), CI gate (`acknowledgements-diff`), and release-pipeline publish (`.github/workflows/release.yml` mirrors the file to `eddacraft/anvil`). |
| Driver runtime (added 2026-05-22) | Bash, per-ecosystem scripts under `tools/starters/acknowledgements/drivers/` | Keeps the kit drop-in portable for subtree consumers. A Rust rewrite (`anvil licenses generate`) was considered and reserved as a future re-decision if a single driver outgrows shell; the eddacraft-tui / future-public-consumer story depends on copy-or-subtree adoption, which a compiled binary would break. |
| Node tool (added 2026-05-22) | `license-checker` | Mature, deterministic, simple to drive from shell. `cyclonedx-npm` is the design-doc-preferred SBOM path and stays available as a future `tool=` option per block; not on the v3.2 critical path. `pnpm licenses list` rejected as pnpm-only. |
| Go tool (added 2026-05-22) | `go-licenses` (Google) | Strict-license-check parity with cargo-about's `--fail` via `go-licenses check`. Native `go.mod` `replace` directive support means monorepo internal-module handling is free. |
| Python tool (added 2026-05-22) | `pip-licenses` against a consumer-supplied pre-built venv | Python licence tools walk an installed venv rather than a lockfile. Requiring a pre-built venv keeps the kit free of `uv` / `poetry` / `pdm` opinions; consumers wire up their preferred installer in CI. Driver fails with an actionable error if the venv is missing rather than producing an empty block. |
| Deferred ecosystems (added 2026-05-22) | Java/Kotlin, Ruby, Swift | No current downstream consumer ships these. Re-open as a fresh task if a real consumer surfaces a need; the driver-dispatch shape accommodates them without further architecture work. |

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
      (`little-termi`; plus `eddacraft-tui` + future public projects).
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

- **Status:** Done
- **Intent:** Stable reference that downstream consumers read before adopting the kit.
- **Expected Outcome:** README in `tools/starters/acknowledgements/` covers marker syntax, idempotency invariants, `--check` exit-code semantics, atomic-write / empty-output / marker-count guarantees.
- **Validation:** README exists; `markdownlint` clean; cross-references resolve.

### ATTRIB-002: Parameterise the generator via a config file

- **Status:** Done
- **Intent:** Eliminate hard-coded `crates/anvil-cli/Cargo.toml` and `pnpm run licenses:generate` strings from the bash.
- **Expected Outcome:** Generator reads `attribution.toml` (per-ecosystem manifests + project metadata) instead of baked-in paths. Anvil's existing config lives at repo root.
- **Validation:** `tools/generate-acknowledgements.sh --check` passes against unchanged anvil graph after the refactor; no project-specific strings remain in the script.

### ATTRIB-003: Extract starter kit to `tools/starters/acknowledgements/`

- **Status:** Done
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

- **Status:** Merged via PR #1549 (`b68f33d6` · 2026-05-14)
- **Intent:** Eliminate the `about.toml.accepted` ↔ `deny.toml.[licenses].allow` drift smell.
- **Expected Outcome:** Canonical `licences.toml` at repo root; expander script produces both `about.toml`-shaped and `deny.toml`-shaped fragments at run time. CI lints for drift.
- **Validation:** Contributor adds a licence to `licences.toml`; both consumers reflect it without further edits. Removing from one without `licences.toml` triggers CI failure.
- **Implementation:** `licences.toml` is the canonical source — each `[[licences]]` entry tags itself with `about = true|false` and `deny = true|false` so consumer-specific entries (defensive `OpenSSL` for cargo-about's ring workaround, `LicenseRef-Proprietary` for cargo-deny's allowance of internal crates) live alongside the shared majority. `tools/starters/acknowledgements/expand-licences.sh` rebuilds the two consumer arrays between BEGIN/END marker comments and supports `--check` for drift detection. CI runs both the drift check against the real `licences.toml` and a three-scenario fixture self-test (`tests/licences-drift.sh`) that pins the matcher.
- **Shipped:** PR #1549 merged 2026-05-14 at commit `b68f33d6`. Repo state verified 2026-05-23: `licences.toml` present with the documented `[[licences]]` schema; `tools/starters/acknowledgements/expand-licences.sh --check` exits 0 against current consumers; the fixture test (`tests/licences-drift.sh`) covers all three scenarios (clean expand → check passes; new licence in source → drift detected; hand-edit in consumer → drift detected); CI runs both at `.github/workflows/rust.yml:397-402`. Not yet in a published release record (release-narrative status remains `Merged`, not `Released/Shipped`).

### ATTRIB-007: Workspace-crate `license` field lint

- **Status:** Done
- **Intent:** Make cargo-about's "no license field" warning a hard error so missing fields can't slip past review.
- **Expected Outcome:** CI step fails when a workspace crate lacks a `license` (or `license-file`) field. Existing crates already comply (per RUSTNX-009); this prevents regression.
- **Validation:** Test crate without a `license` field triggers the lint locally and in CI.
- **Shipped:** 2026-05-14 via PR #1546 (merged at `139606ec`). `cargo about generate --fail` is now passed by `tools/starters/acknowledgements/generate-acknowledgements.sh`; `tools/starters/acknowledgements/tests/strict-license-field.sh` pins the contract; the Acknowledgements freshness CI job runs the fixture test alongside the existing freshness check; downstream consumers pick up the same coverage via the kit's `ci-freshness.yml.snippet`. Anvil's real workspace `--check` still exits 0 (every crate complies per RUSTNX-009).

### ATTRIB-008: Multi-block dispatcher + driver-per-ecosystem architecture

- **Status:** Merged via PR #1888 (`a2001a9d` · 2026-05-24)
- **Execution plan:** `plans/execution/ATTRIB-008.steps.md` (kicked off 2026-05-24 on `feat/attrib-008-dispatcher-drivers`).
- **Intent:** Refactor the generator from "one cargo-about call, one block" into a dispatcher that reads a `[[blocks]]` array from `attribution.toml` and routes each block to an ecosystem-specific driver script.
- **Expected Outcome:**
  - `attribution.toml` schema gains a `[[blocks]]` array. Each block declares `name`, `ecosystem`, and ecosystem-specific keys (manifest path, template path, tool-specific options).
  - Flat `[rust]` top-level schema auto-promotes to a single unnamed block. Existing consumers (Anvil, eddacraft-tui via mirror) do not need to migrate.
  - Generator main script becomes a dispatcher: parse config → loop blocks → invoke `drivers/<ecosystem>.sh` → splice each block independently.
  - Rust driver extracted from the current generator body into `drivers/rust.sh` with no behaviour change.
  - Markers carry per-block names: `<!-- BEGIN AUTO-GENERATED <name> -->` / `<!-- END AUTO-GENERATED <name> -->`. Per-block marker-count gate; per-block atomic write. A failure in one block leaves all other blocks untouched.
  - README documents the dispatcher contract, the block schema, and the driver-author contract (preflight + render + strict-license + deterministic-output expectations).
- **Validation:** Two-block fixture (Rust + a stub ecosystem) round-trips through partial regeneration without touching the other block. Anvil's existing single-block flow continues to pass `--check` clean post-refactor. Mirror to `eddacraft/acknowledgements-starter` builds cleanly; `eddacraft-tui` consumer regenerates byte-identically.
- **Dependencies:** None (keystone for ATTRIB-012/013/014/015).
- **Shipped:** PR #1888 merged 2026-05-24 at commit `a2001a9d`. Repo state verified 2026-05-24: `tools/starters/acknowledgements/generate-acknowledgements.sh` (547 lines) is the dispatcher; `tools/starters/acknowledgements/drivers/rust.sh` (92 lines) carries the extracted Rust driver; `tools/starters/acknowledgements/README.md` (427 lines) documents the dispatcher contract + block schema + driver-author contract. Fixture tests green: `tests/dispatcher-schema-validation.sh` 5/5 (schema rejection paths — mixed flat-`[rust]`+`[[blocks]]`, missing `name`, missing `ecosystem`, unknown `ecosystem`, duplicate `name`); `tests/dispatcher-two-block.sh` 3/3 (two-block round-trip + idempotency + `--check`, partial regeneration leaves the other block byte-identical, driver failure leaves the on-disk target byte-identical); `tests/strict-license-field.sh` covers the flat-`[rust]` back-compat shim end-to-end through the rust driver. Anvil's real workspace `generate-acknowledgements.sh --check` exits 0 against the live `attribution.toml`. Mirror workflow run [26347531626](https://github.com/eddacraft/anvil-001/actions/runs/26347531626) succeeded on the merge commit; `eddacraft-tui` downstream byte-identicality has not been re-verified post-merge (deferred to ATTRIB-009 round-trip). Not yet in a published release record (release-narrative status remains `Merged`, not `Released/Shipped`).

### ATTRIB-009: Port the kit back into `little-termi`

- **Status:** Pending
- **Intent:** Replace the hand-ported v1 copy in `little-termi` with the v3 starter kit; confirm both repos regenerate identically.
- **Expected Outcome:** `little-termi` adopts the kit via `git subtree pull` (or copy if subtree isn't workable) and runs `--check` clean. Divergence is a `little-termi` CI failure.
- **Validation:** Both repos pass their respective `acknowledgements-diff` jobs after the port.

### ATTRIB-010: Update release runbook + doc checklist

- **Status:** Merged via PR #1550 (`92d128ab` · 2026-05-14)
- **Intent:** Reference the starter-kit location and the `--check` invocation downstream consumers should run pre-release.
- **Expected Outcome:** `docs/guides/release-doc-checklist.md` mentions the kit; release runbook calls out the gate.
- **Validation:** Release runbook references resolve; doc lint passes.
- **Shipped:** PR #1550 merged 2026-05-14 at commit `92d128ab`. Repo state verified 2026-05-23: `docs/guides/release-doc-checklist.md:180` links the `attribution-pipeline-v3` module; lines 185-194 list both pre-tag `--check` invocations (`generate-acknowledgements.sh --check` + `expand-licences.sh --check`) with cross-refs to ATTRIB-006 + ATTRIB-007 and the rationale for running them locally before tagging. Not yet in a published release record (release-narrative status remains `Merged`, not `Released/Shipped`).

### ATTRIB-011: Mirror starter kit to a public sibling repo

- **Status:** Done
- **Intent:** Make the kit usable from public projects (`eddacraft-tui`, the owner's future public projects) that can't `git subtree` from the private `anvil-001` repo.
- **Expected Outcome:** New public repo (proposed: `eddacraft/acknowledgements-starter`) mirrors `tools/starters/acknowledgements/` with a one-shot or scheduled mirror job. Public repo carries its own README pointing at this module for design history.
- **Validation:** Public repo exists; mirror job succeeds; one external project (`eddacraft-tui`) consumes it.
- **Execution plan:** `plans/execution/ATTRIB-011.steps.md` (kicked off 2026-05-17 on `feat/attrib-011-public-mirror`).
- **Shipped:** 2026-05-18. Validation passed end-to-end: public mirror live at <https://github.com/eddacraft/acknowledgements-starter>, force-pushed by `.github/workflows/mirror-acknowledgements-starter.yml` on every change to `tools/starters/acknowledgements/`; latest mirror run succeeded at <https://github.com/eddacraft/anvil-001/actions/runs/26019193982>; `eddacraft-tui` consumes the mirror via subtree at `tools/starters/acknowledgements/` (eddacraft/eddacraft-tui#33, merged 2026-05-18). Final design + doc fixes landed via PRs #1677 (scaffold), #1686 (PAT auth fix for CURLE_URL_MALFORMAT), #1689 (kit README mirror pointer + Action 6 retarget to eddacraft-tui), #1691 (per-kit-prefix doc discipline; wrap design rejected).

### ATTRIB-012: Node ecosystem driver

- **Status:** Merged via PR #1903 (`6f9c1ab5` · 2026-05-24)
- **Execution plan:** `plans/execution/ATTRIB-012.steps.md` (kicked off 2026-05-24 on `feat/attrib-012-node-driver`).
- **Intent:** Add a Node/JS driver so consumers can attribute pnpm / npm dependencies via the same `[[blocks]]` dispatcher.
- **Expected Outcome:**
  - `drivers/node.sh` shells `license-checker` against a single `package.json`, emits deterministic markdown sorted by package name.
  - `[[blocks]]` entry with `ecosystem = "node"` carries `manifest_path` (which `package.json` to walk), `prod_only` (default `true`), optional `exclude` globs for internal `@workspace/*` deps that pnpm hoists into the graph.
  - Preflight verifies `license-checker` is installed and `node_modules` is populated; missing state fails with an actionable error rather than producing an empty block.
  - Strict-license enforcement via `license-checker --onlyAllow` (kickoff decision — see execution plan; the spec's open question on `--failOn` vs `--onlyAllow` resolved against the live `license-checker@25.0.1` CLI surface, semicolon-separated) matched against the canonical `licences.toml` allow-list (extends ATTRIB-006's expander to emit a Node-shaped fragment `licences.node-allow.txt`).
  - README gains a monorepo guidance section with worked pnpm-workspace examples covering the "one block per shipping `package.json`" pattern and the workspace-wide escape hatch.
- **Validation:** Two-package fixture (built dynamically under `mktemp` per the kit's other tests — `file:./packages/*` deps so no network beyond the per-test `npm install` of pinned `license-checker@25.0.1`) round-trips through the driver in `tests/node-driver-render.sh`. `--check` reports drift when a fixture dependency's licence changes. `tests/node-driver-strict.sh` triggers a non-zero exit when a disallowed-licence package is introduced and leaves the on-disk target byte-identical. `tests/node-driver-preflight.sh` covers missing `node_modules`, missing `license-checker`, missing `manifest_path`, and wrong-argv-count error paths.
- **Dependencies:** ATTRIB-008 (dispatcher must exist first — landed via PR #1888); ATTRIB-006 (allow-list expander must emit a Node-shaped fragment — extended in this PR).
- **Shipped:** PR #1903 merged 2026-05-24 at commit `6f9c1ab5`. Repo state verified 2026-05-25: `tools/starters/acknowledgements/drivers/node.sh` (168 lines) is the Node driver; `tools/starters/acknowledgements/licences.node-allow.txt.template` ships the marker scaffolding for new consumers; Anvil's own `licences.node-allow.txt` at project root carries the populated 13 SPDX entries (dormant until ATTRIB-015 declares a node block). Fixture tests green: `tests/node-driver-preflight.sh` 4/4 (wrong argv count + missing `manifest_path` + missing `node_modules` with installer hint + missing `license-checker` via controlled PATH); `tests/node-driver-render.sh` 3/3 (two-package `file:./packages/*` round-trip + idempotent + `--check` exit 0); `tests/node-driver-strict.sh` 2/2 (disallowed-licence rejection naming the offending `fake-gpl` + on-disk target byte-identicality across the strict-gate failure). `expand-licences.sh --check` exit 0 against the live `licences.toml` (Node fragment expander emits silently when `licences.node-allow.txt` is present); real-workspace `generate-acknowledgements.sh --check` exit 0. First production run of the new `acknowledgements-kit.yml` workflow (run [26363350501](https://github.com/eddacraft/anvil-001/actions/runs/26363350501)) succeeded on the merge commit. Not yet in a published release record (release-narrative status remains `Merged`, not `Released/Shipped`).

### ATTRIB-013: Go ecosystem driver

- **Status:** Merged via PR #1929 (`feat/attrib-013-go-driver`)
- **Intent:** Add a Go driver so consumers can attribute Go module dependencies via the same dispatcher.
- **Expected Outcome:**
  - `drivers/go.sh` shells `go-licenses report` against a binary import path, emits deterministic markdown using a Go template under `templates/go-licenses.tmpl`.
  - `[[blocks]]` entry with `ecosystem = "go"` carries `module_path` (binary import path to walk, e.g. `./cmd/anvil`), `template_path`.
  - Preflight verifies `go-licenses` is installed and the module cache is populated (consumer ran `go mod download`); missing state fails with an actionable error.
  - Strict-license enforcement via `go-licenses check` ahead of `report`, matched against the canonical `licences.toml` allow-list (extends ATTRIB-006's expander to emit a Go-shaped fragment).
  - `go.mod` `replace` directives honoured natively — no special handling needed for monorepo internal modules.
- **Validation:** Go fixture binary with one external dep round-trips through the driver. `--check` reports drift when a fixture dependency's licence changes. Strict-license fixture triggers a non-zero exit when a disallowed licence is introduced.
- **Dependencies:** ATTRIB-008; ATTRIB-006 expander emits Go-shaped fragment.
- **Shipped:** PR #1929 (`feat/attrib-013-go-driver`). `drivers/go.sh` shells `go-licenses report` against a `module_path` (package/binary dir), finds the enclosing `go.mod`, runs from the module root, and `--ignore`s the project's own main module (`go list -m`). Strict gate via `go-licenses check --allowed_licenses`; render carries module import path + SPDX licence only (no source URL — go-licenses resolves URLs over the network, which would break `--check` determinism); `templates/go-licenses.tmpl` emits rows and the driver sorts + headers them. `expand-licences.sh` emits a comma-joined Go fragment into `licences.go-allow.txt` (same optional-presence shape as the Node fragment); Anvil ships a dormant populated root `licences.go-allow.txt` (no Go block in `attribution.toml`). Fixture tests (network-free, local-`replace` Go module): `go-driver-preflight.sh` 5/5, `go-driver-render.sh` 3/3, `go-driver-strict.sh` 2/2; `licences-drift.sh` gains a Go-fragment scenario (6 total). `acknowledgements-kit.yml` installs pinned `go-licenses@v1.6.0` and runs the three Go tests. Verified locally: 10/10 kit tests, `expand-licences --check` + `generate-acknowledgements --check` exit 0. Not yet in a published release record (status `Merged`, not `Released/Shipped`).

### ATTRIB-014: Python ecosystem driver

- **Status:** Merged via PR #1932 (`feat/attrib-014-python-driver`)
- **Intent:** Add a Python driver so consumers can attribute Python dependencies via the same dispatcher.
- **Expected Outcome:**
  - `drivers/python.sh` shells `pip-licenses` against a consumer-supplied pre-built virtualenv, emits deterministic markdown sorted by package name.
  - `[[blocks]]` entry with `ecosystem = "python"` carries `venv_path` (required, points at the consumer's `uv sync` / `poetry install` / `pdm sync` output), optional template overrides.
  - Preflight verifies the venv exists and contains `pip-licenses`; missing or empty venv fails with the actionable error "no installed dependencies at `<path>`; run `<consumer's installer>` first" rather than producing an empty block.
  - Strict-license enforcement via `pip-licenses --fail-on` matched against the canonical `licences.toml` allow-list (extends ATTRIB-006's expander to emit a Python-shaped fragment).
  - Kit ships no `uv` / `poetry` / `pdm` opinions — consumer wires their preferred installer in CI per their existing Python toolchain.
- **Validation:** Python fixture with a pre-built venv and one external dep round-trips through the driver. Missing-venv case produces the actionable error rather than an empty block. Strict-license fixture triggers a non-zero exit when a disallowed licence is introduced.
- **Dependencies:** ATTRIB-008; ATTRIB-006 expander emits Python-shaped fragment.
- **Shipped:** PR #1932 (`feat/attrib-014-python-driver`). `drivers/python.sh` runs the consumer-supplied venv's own `pip-licenses` (`venv_path` + `python_allow_path`); kit ships no installer opinions. Strict gate `pip-licenses --allow-only` (semicolon-joined) chosen over the spec's `--fail-on` (allow-list semantics match the about=true set); render `--format markdown --order name`. pip-licenses self-excludes its own tool chain, so the block lists only the consumer's deps; an empty venv (only the tool) produces the actionable "no installed dependencies" error. `expand-licences.sh` emits a semicolon-joined `licences.python-allow.txt` fragment (same optional-presence shape as Node/Go); Anvil ships a dormant populated root `licences.python-allow.txt`. README documents the licence-name caveat (pip-licenses reports classifier-derived names, not always exact SPDX). Also hardened `drivers/go.sh`/`node.sh` allow-line extraction (`|| true`) against a `set -e`/`pipefail` silent-abort on an unexpanded allow-list. Tests: `python-driver-preflight.sh` 5/5, `python-driver-render.sh` 3/3, `python-driver-strict.sh` 2/2 (local fixture package + pip-licenses); `licences-drift.sh` now 7 scenarios. `acknowledgements-kit.yml` provisions Python 3.12 + pinned pip-licenses 5.5.5. Verified locally: 13/13 kit tests, `expand-licences --check` + `generate-acknowledgements --check` exit 0. Not yet in a published release record (status `Merged`, not `Released/Shipped`).

### ATTRIB-015: Anvil adopts a Node devtools attribution block

- **Status:** Merged via PR #1911 (`101ee6fd` · 2026-05-24)
- **Execution plan:** `plans/execution/ATTRIB-015.steps.md` (kicked off 2026-05-25 on `feat/attrib-015-node-devtools`).
- **Intent:** Exercise the Node driver in Anvil's own `ACKNOWLEDGEMENTS.md` to attribute the JS/TS dev tooling the repo continues to depend on (linters, formatters, Nx, kindling integration, build scripts).
- **Expected Outcome:**
  - `attribution.toml` grows a `[[blocks]] node-devtools` entry pointed at a dev-tooling `package.json` (root `package.json` or a curated devtools manifest — decide during implementation based on what gives the cleanest attribution).
  - `ACKNOWLEDGEMENTS.md` gains a `<!-- BEGIN AUTO-GENERATED node-devtools -->` block with the rendered dev-tool attributions, sitting alongside the existing Rust block.
  - CI freshness check enforces drift across both blocks; the `acknowledgements-diff` job's command surface stays the same.
  - Release runbook (ATTRIB-010) reflects the second block in its pre-release `--check` callout.
- **Validation:** `tools/starters/acknowledgements/generate-acknowledgements.sh --check` exits 0 against both blocks in a clean working tree. Introducing a new dev dep triggers drift; removing one triggers drift; both resolve cleanly via a single re-run of the generator.
- **Dependencies:** ATTRIB-012 (Node driver must exist).
- **Shipped:** PR #1911 merged 2026-05-24 at commit `101ee6fd`. Repo state verified 2026-05-25: `attribution.toml` migrated from the flat `[rust]` shim to `[[blocks]]` with a `node-devtools` entry (`ecosystem = "node"`, `manifest_path = "tools/dev/package.json"`, `prod_only = false`); `tools/dev/package.json` + `tools/dev/package-lock.json` ship the curated 9-dep devtools manifest (8 build/test/lint tools + license-checker), excluded from `pnpm-workspace.yaml` via `!tools/dev` to dodge the pnpm-hoisting/`glob` incompat with `license-checker@25.0.1`; `ACKNOWLEDGEMENTS.md` carries the populated `node-devtools` block (282 attributed packages). Scope decision (spec open question line 358-363): **curated minimal** — root `prod_only=false` produced 2034 transitive packages including proprietary Nx Powerpack + Copilot CLI; curated gives 282 with no proprietary noise. Four permissive licences added to `licences.toml` (`BlueOak-1.0.0`, `0BSD`, `Python-2.0`, `CC-BY-3.0`) and propagated through `about.toml` / `deny.toml` / `licences.node-allow.txt`. `acknowledgements-diff` job extended with `(cd tools/dev && npm ci --ignore-scripts)` + PATH prepend; Node version bumped 20 → 22 to match `engines.node`. First production run of the new `acknowledgements-kit.yml` workflow on the merge commit (run [26369276266](https://github.com/eddacraft/anvil-001/actions/runs/26369276266)) succeeded. Not yet in a published release record (release-narrative status remains `Merged`, not `Released/Shipped`).

### ATTRIB-016: Deterministic comment-wrapping in `expand-licences.sh`

- **Status:** Merged via PR #1925 (`fix/attrib-016-deterministic-wrap`)
- **Intent:** Make the expander's `licences.toml` `note` wrapping produce byte-identical output regardless of which `coreutils` implementation provides `fold`, so the acknowledgements freshness gate can't fail spuriously when a contributor's local `fold` disagrees with CI's.
- **Problem:** `expand-licences.sh` wraps long `note` fields with `fold -s -w 75`. `fold` wraps on **byte** count, not display columns; a `note` containing multi-byte UTF-8 (em dashes are 3 bytes) wraps at a different word boundary under GNU coreutils vs uutils coreutils vs different uutils versions. The locally-generated `about.toml` / `deny.toml` / `licences.node-allow.txt` then differ byte-for-byte from what CI regenerates, and `expand-licences.sh --check` reports drift the author can't see locally. Bit PR #1911 (ATTRIB-015): the BlueOak-1.0.0 note's em dash wrapped one word later under local uutils 0.8.0 than under CI's coreutils, requiring a regenerate-with-matching-`fold` fix commit (`898554a6`).
- **Expected Outcome:**
  - Replace `fold -s -w 75` in `expand-licences.sh`'s `render_fragment` with a deterministic, implementation-independent wrap (awk with explicit display-width counting, or a small pure-bash word-wrap loop). No dependency on `fold`'s byte-vs-column behaviour.
  - Wrapping is identical across GNU coreutils, uutils coreutils (all versions), and BusyBox.
  - Existing `about.toml` / `deny.toml` / `licences.node-allow.txt` regenerate byte-identically OR the one-time reflow is committed as part of this work item with a note that it's a wrapping-normalisation, not a content change.
- **Validation:** A fixture `note` containing em dashes and other multi-byte UTF-8 wraps to the same bytes under at least two `fold` implementations (and under the new fold-free path). `tests/licences-drift.sh` gains a scenario that pins the wrap output so a regression that reintroduces `fold` (or an implementation-sensitive wrap) is caught. `expand-licences.sh --check` exits 0 on a tree generated by either coreutils.
- **Dependencies:** None (ATTRIB-006 expander already shipped; this hardens it). Independent of ATTRIB-013/014 but de-risks them — every future driver's allow-list `note` fields hit the same wrap path.
- **Shipped:** PR #1925 (`fix/attrib-016-deterministic-wrap`). `fold -s -w 75` in `render_fragment` replaced by two pure-bash helpers: `cp_len` (code-point length in byte mode — `LC_ALL=C` + stripping UTF-8 continuation bytes `0x80-0xBF`) and `wrap_note` (greedy whitespace wrap at ≤75 code points). No dependency on any `fold` on PATH. `about.toml` / `deny.toml` regenerated once — pure wrapping-normalisation (trailing whitespace dropped, tighter packing near the em-dash notes), no licence content change; `licences.node-allow.txt` is single-line and unchanged. `tests/licences-drift.sh` scenario 5 pins the contract: ≤75 code points/line, words intact, idempotent under `--check`, and byte-identical output with a poisoned `fold` first on PATH. Verified locally: all 7 kit self-tests green, `expand-licences.sh --check` + `generate-acknowledgements.sh --check` exit 0 (ACKNOWLEDGEMENTS.md byte-stable), determinism confirmed via poisoned-`fold` regen. Not yet in a published release record (status `Merged`, not `Released/Shipped`).

## Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Scope creeps into supply-chain attestation / SBOM publication | High | v3 stops at the markdown attribution artefact; SBOM publication is explicitly out of scope and queued under `release-management` |
| CycloneDX intermediate is heavier than the project needs today | Medium | Keep `cargo-about` direct path supported; CycloneDX is additive, not forced |
| Hand-ported starter-kit copies drift from canonical | Medium | ATTRIB-009 validates the kit by re-porting it into a known consumer (little-termi); divergence is a CI failure there |
| Public consumers (`eddacraft-tui`, future public projects) can't `git subtree` from a private repo | Medium | ATTRIB-011 mirrors the kit to a public sibling repo; design history stays in this private module |
| Manual `## Thanks` section still rots over time | Low | CI requires the section exists with at least one entry per ecosystem represented in the auto-generated block; contents stay manual by design |
| `licences.toml` drift gate produces false-positive churn during adds | Low | Drift gate runs `--check` mode; surfaces an actionable diff rather than blocking the commit silently |
| Bash multi-driver framework outgrows shell (added 2026-05-22) | Medium | Each driver is a self-contained shell script with isolated preflight, render, and strict-license logic; tested via per-driver fixtures. If a single driver gets gnarly enough that bash becomes the bottleneck, promote the generator to a Rust binary (`anvil licenses generate`) as a separate decision — the dispatcher contract stays the same. |
| Driver preflight requirements diverge across CI environments (added 2026-05-22) | Medium | Each driver fails fast with an actionable error naming the missing tool or state (`license-checker` not installed, `node_modules` not populated, venv path empty, module cache missing); `ci-freshness.yml.snippet` documents per-eco setup steps and is updated alongside each new driver. |
| ATTRIB-006 allow-list expander has to grow Node/Go/Python output shapes (added 2026-05-22) | Low | The expander already emits two consumer shapes (`about.toml.accepted`, `deny.toml.[licenses].allow`) via marker-spliced fragments; adding three more is mechanical. Per-driver fixtures pin the expected fragment format so drift fails closed. |

## Open Questions

All Ready-blocking questions resolved in [Decisions](#decisions).
The two tactical questions deferred to their respective tasks
(`anvil licenses --json` mode, SPDX rigour for bundled binaries) live
inline against ATTRIB-005 and ATTRIB-004.
