<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Feature Flag Catalogue

| Scope    | Owner | Priority | Status |
| -------- | ----- | -------- | ------ |
| FLAGCAT  | —     | medium   | Draft  |

## Purpose

Retire per-surface flag definition modules in favour of a single authoritative
flag catalogue, aligned with the OpenFeature manifest pattern so the definitions
are portable across TypeScript and Rust surfaces without hand-syncing.

FLAGS built the shared resolver, schema, snapshot loader, and telemetry.
FLAGM migrated every ad-hoc gate onto those contracts. What FLAGM did **not**
address is that the five resulting flag definitions live in three different
packages:

- `crates/anvil-cli/src/feature_flags.rs` — `cli.licence-gate`
- `apps/docs-site/lib/feature-flags.ts` — `docs.access`
- `apps/anvil-api/src/lib/feature-flags.ts` — `api.scope.beta`,
  `api.scope.preview`, `api.scope.internal`

Each one duplicates the `FeatureFlagDefinition` shape in its host language. A
new flag today needs to be declared in whichever surface "owns" it and then
cross-imported elsewhere, which is exactly what FLAGM was supposed to remove.

**Product direction:** adopt OpenFeature's convention of a single JSON manifest
(`flags.json`) at the repo root, consumed by typed loaders per surface. Keep
Anvil's extended schema (which is already a superset of OpenFeature's vanilla
`FeatureFlagDefinition`) rather than collapsing back to the vanilla one — our
extensions (`class`, `owner`, `intent`, `targeting`, `createdFor`, `status`)
are the governance guarantees we've already invested in via FLAGS.

## In Scope

- Authoritative manifest: a single `flags/manifest.json` at the repo root
  holding every shipped flag (location chosen to match OpenFeature upstream
  convention; signals cross-cutting product data, not per-package data;
  leaves room for `flags/fixtures/` and `flags/environments/` overlays)
- TS loader package: `packages/anvil/flags-catalogue/` that imports the JSON,
  validates it against `FeatureFlagManifestSchema`, and re-exports typed
  accessors (`CLI_LICENCE_GATE`, `DOCS_ACCESS_FLAG`, `API_SCOPE_FLAGS`, …)
- Rust codegen: a `build.rs` in `crates/anvil-kernel-types` (or a new
  `crates/anvil-flags-catalogue`) that reads the same JSON at build time and
  emits matching Rust constants (keys, variant names, default variants) so the
  Rust CLI consumes identical definitions without a manual second copy
- Migration of existing call sites off the per-surface modules onto the
  catalogue package
- CI consistency check: a test that fails if manifest JSON, TS re-exports, and
  Rust codegen drift
- `.openfeature.yaml` config checked in for future adopters of the upstream
  `openfeature generate` CLI (optional — no build step added yet)
- Adoption guide in `docs/guides/feature-flag-inventory.md` explaining how to
  add a new flag (one PR, one file) and removing the "split across surfaces"
  language left over from FLAGM

## Out of Scope

- Swapping Anvil's custom Rust resolver for the upstream `open-feature` crate
  runtime SDK — evaluated separately; keeping the custom resolver for FLAGS's
  governance features (class-based override policy, deterministic reason codes)
  for now
- Adopting the OpenFeature `openfeature generate` CLI as a required build step
  — upstream doesn't yet ship a Rust generator, and adding a Node-based codegen
  step to the Rust crate build pipeline is more cost than value today
- New flag definitions — FLAGCAT is a migration, not a capability module
- Dashboard-side flag registration or runtime admin UI — future work, not
  blocked by this
- Reworking the resolver, snapshot loader, or telemetry — those stay as-is

## Interfaces

**Depends on:**

- `feature-flagging` (FLAGS, Complete) — shared manifest schema, resolver,
  snapshot loader, telemetry contract
- `feature-flag-migration` (FLAGM, Complete) — every current flag already
  resolves through the shared resolver; the catalogue can replace the per-
  surface modules without a behaviour change

**Exposes:**

- `flags/manifest.json` — canonical source of truth, versioned under
  `FEATURE_FLAG_SCHEMA_VERSION`
- `@eddacraft/anvil-flags-catalogue` — TS loader package with typed accessors
- Rust constants emitted into `anvil-kernel-types` (or sibling crate) matching
  the TS accessors by key and variant names
- `.openfeature.yaml` — opt-in config for anyone who wants to run
  `openfeature generate` locally (no CI integration)

## Acceptance Criteria

- [ ] A single `flags/manifest.json` contains every shipped flag definition
      (`cli.licence-gate`, `docs.access`, `api.scope.beta`, `api.scope.preview`,
      `api.scope.internal`) and validates against `FeatureFlagManifestSchema`
- [ ] All TS call sites import their flag definition from
      `@eddacraft/anvil-flags-catalogue`, not from a per-app `feature-flags.ts`
- [ ] `crates/anvil-cli/src/feature_flags.rs` derives the `cli.licence-gate`
      key, variants, and default variant from codegen output; the hand-rolled
      `CliGateFlag` literal is gone or reduced to CLI-host metadata only (the
      `CLI_GATED_COMMANDS` list remains CLI-local)
- [ ] A consistency check (TS test or standalone CI step) fails if the JSON
      manifest, TS re-exports, and Rust codegen output drift
- [ ] `docs/guides/feature-flag-inventory.md` documents the "add a flag" flow
      as a single-file edit + regenerate (one PR, one source of truth)
- [ ] The resolver's existing OpenFeature-shaped exports (`resolveFlag`,
      `ResolutionDetails`, `FlagOverrides`) are unchanged — FLAGCAT only
      replaces **where definitions live**, not how they're evaluated

## Constraints

- Behaviour-preserving: no flag gains or loses a targeting rule, class,
  variant, or default during migration
- No new runtime dependencies added to the Rust CLI — the codegen step must
  run in `build.rs` with the existing `serde`/`serde_json` toolchain, or use
  a minimal hand-rolled JSON reader if adding `serde_json` as a build-dep is
  rejected
- The manifest must remain valid OpenFeature-adjacent JSON so a future upgrade
  to upstream codegen (if/when Rust support lands) is a configuration change,
  not a rewrite
- Docs-site's Vercel edge bundle must continue to work — the catalogue package
  must be edge-compatible (no Node-only imports in the consumer path)
- The CLI gated-command list stays CLI-local; it is CLI routing metadata, not
  flag-manifest data

## Design Spec

Not yet written. A follow-up work item (FLAGCAT-001) produces a short design
note covering:

- Manifest JSON layout vs. upstream OpenFeature `flags.json` (what we extend,
  what we leave alone so upstream tooling keeps working)
- Rust codegen approach (`build.rs` + `serde_json` vs. minimal parser) and how
  constants are named to match the TS accessors
- How the consistency check runs (Vitest over the JSON + generated files, or
  a standalone `node` script invoked from CI)
- Migration ordering — bootstrap catalogue package → flip TS surfaces → add
  Rust codegen → flip CLI → add consistency check → delete per-surface modules

## Ready Checklist

Change status to **Ready** when:

- [ ] Design note documents the manifest layout, codegen approach, and
      consistency-check strategy (FLAGCAT-001)
- [ ] Rust codegen approach confirmed against the `anvil-kernel-types` build
      profile — prototype a `build.rs` walk from `CARGO_MANIFEST_DIR` to the
      workspace root's `flags/manifest.json` and verify `cargo:rerun-if-changed`
      fires correctly
- [ ] Adoption guide outline agreed with inventory doc owners

## Risks & Mitigations

| Risk                                                         | Mitigation                                                                         |
| ------------------------------------------------------------ | ---------------------------------------------------------------------------------- |
| Rust `build.rs` can't reliably walk from `CARGO_MANIFEST_DIR` to the workspace root to find `flags/manifest.json` | Standard workaround: walk up until a `Cargo.toml` with `[workspace]` is found, emit `cargo:rerun-if-changed` for the resolved path. FLAGCAT-001 prototypes this; fallback is a thin `crates/anvil-flags-catalogue/` crate that owns the JSON and re-exports it to TS via `"files"` — retreat to that only if the root-level path genuinely proves painful |
| OpenFeature CLI lands Rust codegen mid-migration             | Harmless — the JSON layout is compatible; we'd just replace our `build.rs` with the upstream tool without schema changes |
| Consistency check is flaky (timezone, formatter drift)       | Compare parsed JSON + generated constants by structural equality, not stringwise; run through the same formatter as the source |
| Docs-site edge bundle regresses                              | Ship the catalogue as an ESM package with no Node-only imports on the consumer path (same constraint FLAGM-004 already met) |
| Migration lands partially and the duplicate definitions sit on `dev` | Each work item is behaviour-preserving and ships independently; no cutover needs both halves landed in the same PR |

## Tasks

### FLAGCAT-001: Design note — manifest layout, Rust codegen, consistency check — Draft

- **Intent:** Before changing any runtime, agree the manifest layout, the Rust
  codegen mechanism, and the drift-detection strategy.
- **Expected Outcome:** A design note at
  `plans/specs/YYYY-MM-DD-feature-flag-catalogue-design.md` documents the
  manifest JSON location, the TS loader package's public surface, the Rust
  `build.rs` approach (or alternative), how naming maps from JSON keys to
  Rust constants, and how the consistency check runs in CI.
- **Scope:** `plans/specs/`, `docs/guides/feature-flag-inventory.md`
- **Non-scope:** Any code change
- **Validation:** `test -f plans/specs/*-feature-flag-catalogue-design.md`
- **Confidence:** high

### FLAGCAT-002: Bootstrap `@eddacraft/anvil-flags-catalogue` package — Draft

- **Intent:** Stand up the new package, import the five existing flag
  definitions into `flags/manifest.json`, and export typed accessors that
  match the shapes currently exported by the per-surface modules.
- **Expected Outcome:** `flags/manifest.json` exists and validates against
  `FeatureFlagManifestSchema`. `packages/anvil/flags-catalogue/` exports
  `CLI_LICENCE_GATE`, `DOCS_ACCESS_FLAG`, `API_SCOPE_FLAGS`,
  `API_SCOPE_NAMES`, `DEFAULT_APPROVAL_SCOPES`, and a
  `featureFlagManifest()` helper. No existing call site migrated yet.
- **Scope:** `flags/manifest.json`, `packages/anvil/flags-catalogue/`,
  `pnpm-workspace.yaml`, `tsconfig.base.json`
- **Non-scope:** Rust codegen, flipping existing call sites
- **Dependencies:** FLAGCAT-001
- **Validation:** `pnpm nx test flags-catalogue`
- **Confidence:** medium

### FLAGCAT-003: Migrate TS surfaces onto the catalogue package — Draft

- **Intent:** Flip `apps/docs-site/lib/feature-flags.ts` and
  `apps/anvil-api/src/lib/feature-flags.ts` to re-export from the catalogue
  package (or, ideally, to be deleted in favour of direct imports from the
  catalogue at each call site).
- **Expected Outcome:** No flag definition literal (`key`, `variants`,
  `defaultVariant`, `targeting`) exists outside `flags/manifest.json` on the
  TS side. Docs-site middleware and the admin API resolve the same flags they
  resolve today, against the same definitions, byte-for-byte.
- **Scope:** `apps/docs-site/lib/feature-flags.ts`, `apps/docs-site/middleware.ts`,
  `apps/anvil-api/src/lib/feature-flags.ts`, `apps/anvil-api/src/routes/admin.ts`,
  `apps/anvil-api/src/lib/admin-schemas.ts`
- **Non-scope:** Rust side
- **Dependencies:** FLAGCAT-002
- **Validation:** `pnpm nx run-many -t test --projects=docs-site,anvil-api,runtime`
  + successful Vercel Preview deploy for the docs-site
- **Confidence:** medium

### FLAGCAT-004: Rust codegen from `flags/manifest.json` — Draft

- **Intent:** Emit Rust constants (flag key, variant keys, default variant)
  from the JSON manifest at build time so the Rust CLI consumes the same
  source of truth as the TS surfaces.
- **Expected Outcome:** A `build.rs` (in `crates/anvil-kernel-types` or a new
  `crates/anvil-flags-catalogue` crate) reads `flags/manifest.json` at build
  time and emits a generated module exposing Rust constants and variant
  newtypes. Drift between JSON and generated output is detected by the
  consistency check in FLAGCAT-006, not by hand-editing.
- **Scope:** `crates/anvil-kernel-types/` (or new crate), `Cargo.toml`,
  workspace `Cargo.toml`
- **Non-scope:** Flipping the CLI to consume the generated constants (next
  task); replacing the custom resolver with the `open-feature` crate
- **Dependencies:** FLAGCAT-001
- **Validation:** `cargo test -p anvil-kernel-types feature_flags_catalogue`
- **Confidence:** low (build.rs path resolution + workspace layout is the
  riskiest piece of the whole module)

### FLAGCAT-005: Migrate Rust CLI onto generated catalogue constants — Draft

- **Intent:** Replace the hand-rolled `CliGateFlag` literal in
  `crates/anvil-cli/src/feature_flags.rs` with the generated catalogue
  constants, keeping the CLI-local `CLI_GATED_COMMANDS` routing metadata
  as-is (that's CLI-host data, not flag data).
- **Expected Outcome:** The `cli.licence-gate` key, variants, and default
  variant are derived from codegen output, not from a Rust literal. The
  CLI's behaviour, including `ANVIL_DEV=1` local-override, is unchanged.
- **Scope:** `crates/anvil-cli/src/feature_flags.rs`, `crates/anvil-cli/src/main.rs`
- **Non-scope:** Changing which commands are gated; swapping the resolver
- **Dependencies:** FLAGCAT-004
- **Validation:** `cargo test -p eddacraft-anvil --bin anvil feature_flags`
- **Confidence:** medium

### FLAGCAT-006: Consistency check and adoption guide — Draft

- **Intent:** Make drift between the JSON manifest, TS re-exports, and Rust
  codegen loud in CI; update the inventory doc so future contributors know
  "add a flag" is a single-file edit.
- **Expected Outcome:** A Vitest spec (or standalone script invoked from CI)
  parses `flags/manifest.json`, the TS accessors, and the generated Rust
  constants, and asserts structural equality on keys, variant names, and
  default variants. `docs/guides/feature-flag-inventory.md` documents the
  add-a-flag flow and removes any "split across surfaces" language left
  over from FLAGM.
- **Scope:** `packages/anvil/flags-catalogue/`, `docs/guides/feature-flag-inventory.md`,
  optionally `.github/workflows/*.yml`
- **Non-scope:** Dashboard or admin UI for flag management
- **Dependencies:** FLAGCAT-002, FLAGCAT-003, FLAGCAT-004, FLAGCAT-005
- **Validation:** `pnpm nx test flags-catalogue` +
  `grep -q "single source of truth" docs/guides/feature-flag-inventory.md`
- **Confidence:** high
