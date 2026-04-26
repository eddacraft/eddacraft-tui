# ATTRIB-core — Marker contract, parameterised generator, starter kit

## Purpose

Cover ATTRIB-001 (marker-splice contract documentation), ATTRIB-002
(parameterise the generator), and ATTRIB-003 (extract the kit to
`tools/starters/acknowledgements/`) as one coherent slice that lifts the v1
attribution pipeline into a portable v3 starter kit.

## Actions

### 1. Inventory the v1 surface

- **Purpose:** Capture every assumption baked into the current generator before refactoring.
- **Produces:** Notes listing hard-coded paths, marker invariants, and `--check` exit semantics.
- **Checkpoint:** v1 surface, markers, and exit semantics catalogued.
- **Validate:** `tools/generate-acknowledgements.sh --check`

### 2. Draft the marker-splice contract README

- **Purpose:** Give downstream consumers a single canonical reference for the kit's invariants (ATTRIB-001).
- **Produces:** README under `tools/starters/acknowledgements/` covering marker syntax, idempotency, atomic-write, empty-output guard, and marker-count rule.
- **Checkpoint:** README covers marker, idempotency, atomic-write, and exit-code rules.
- **Validate:** `pnpm lint:md`

### 3. Introduce `attribution.toml` schema

- **Purpose:** Move per-ecosystem manifests and project metadata out of the script (ATTRIB-002).
- **Produces:** Documented config schema plus an `attribution.toml` at repo root describing the anvil graph.
- **Checkpoint:** Config schema documented; anvil's `attribution.toml` reflects current graph.

### 4. Parameterise the generator against the config

- **Purpose:** Eliminate hard-coded `crates/anvil-cli/Cargo.toml` and `pnpm run licenses:generate` strings from the bash.
- **Produces:** Refactored `tools/generate-acknowledgements.sh` reading every project-specific value from `attribution.toml`.
- **Checkpoint:** Generator carries no project-specific strings; reads config only.
- **Validate:** `tools/generate-acknowledgements.sh --check`

### 5. Move the kit into `tools/starters/acknowledgements/`

- **Purpose:** Vendor the kit at its agreed canonical location (ATTRIB-003).
- **Produces:** Directory holds the parameterised script, `attribution.toml.example`, `about.toml`/`about.hbs` templates, `ACKNOWLEDGEMENTS.md.template`, README, and CI snippet.
- **Checkpoint:** Kit directory is self-contained with no repo-internal imports.
- **Validate:** `tar -czf /tmp/attrib-kit.tgz tools/starters/acknowledgements/ && tar -tzf /tmp/attrib-kit.tgz`

### 6. Wire the repo to consume the vendored kit

> **Decision 2026-04-26:** v1 entry points retired outright (Option A). Solo
> repo, no external contributors — single source of truth wins; no shim drift
> risk. Delete `tools/generate-acknowledgements.sh` and remove
> `licenses:generate` from `package.json` rather than thin-shimming. Quick
> grep before cutover to fix any internal references.

- **Purpose:** Prove the kit works against its first consumer (anvil itself) without bypass paths.
- **Produces:** Top-level `tools/generate-acknowledgements.sh` and `pnpm run licenses:generate` are **retired** (deleted). Anvil consumes the vendored entry point at `tools/starters/acknowledgements/generate-acknowledgements.sh` directly. Any internal references (CI workflows, docs, runbooks) updated in this step.
- **Checkpoint:** v1 entry points absent; anvil regenerates `ACKNOWLEDGEMENTS.md` solely via the kit; no broken references remain.
- **Validate:** `tools/starters/acknowledgements/generate-acknowledgements.sh --check && ! test -f tools/generate-acknowledgements.sh && ! grep -q '"licenses:generate"' package.json`

### 7. Verify clean adoption from a scratch tree

- **Purpose:** Confirm a fresh repo can adopt by copy plus an `attribution.toml` edit only (ATTRIB-003 acceptance criterion).
- **Produces:** Throwaway scratch run notes recording the steps an external repo would follow.
- **Checkpoint:** Scratch adoption succeeds with config edits only, no script edits.

### 8. Stage and integrate the slice

- **Purpose:** Land ATTRIB-001/002/003 together with a green workspace.
- **Produces:** Staged changes covering kit, config, README, and any CI rewiring.
- **Checkpoint:** Workspace builds and tests cleanly with the new kit in place.
- **Validate:** `git add tools/starters/acknowledgements tools/generate-acknowledgements.sh attribution.toml ACKNOWLEDGEMENTS.md && cargo test --workspace --no-fail-fast`
