<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready, except explicitly authorised decision-only work items recorded inline. -->

# Feature Flag Catalogue

| ID   | Owner | Priority | Status | Progress |
| ------- | ----- | -------- | ------ | -------- |
| FLAGCAT | —     | medium   | Ready  | 2/8      |

**Last reviewed:** 2026-06-01 — Reframed FLAGCAT-008 to its
beta-intentional disposition: the `cli.licence-gate` membership (including
`welcome`) is deliberate beta access control, deferred to GA — not a
planless-first defect. Planless-first is Anvil's zero-config *product
posture* (ADR-001), not a per-feature gate, and was retired as an
evaluation lens in PR #2192. No status or count change — FLAGCAT-008 stays
Draft (GH #1795).

**Earlier — 2026-05-28** — Promoted Draft → **Ready**. The
`v0.7.0-beta` release-freeze deferral on FLAGCAT-002..-006 (recorded
2026-05-19) is now spent: `v0.7.0-beta`, `v0.7.1-beta`, and `v0.7.2-beta`
have all cut, so the contracts-level migration is clear of the freeze
window. FLAGCAT-002..-006 promoted to Ready (FLAGCAT-004 stays
`Confidence: low` — the `build.rs` workspace-root walk is the riskiest
slice; its approach and a sibling-crate fallback are pinned in the design
note §"Rust codegen", so it is execution-authorised at low confidence, not
blocked). FLAGCAT-008 stays Draft pending the planless-membership triage
(GH #1795).

**Earlier — 2026-05-19** — Feature gating model landed at
[`plans/specs/2026-05-19-feature-gating-model.md`](../specs/2026-05-19-feature-gating-model.md)
with architectural pin [ADR-048](../decisions/048-feature-group-architectural-model.md):
Feature Groups are defaults carriers (class + audiences + lifecycle) under a
hybrid `primaryGroup` (surface) + `tags` (capability) taxonomy, with universal
kill-switch via the existing emergency-override channel. Spec adds canonical
audiences (9) and environments (5, `prod` → `production` and `dev` →
`development`) inventories plus a
seven-group primary taxonomy and the day-1 gating policy. **2026-05-18:**
FLAGCAT-001 design note landed at
[`plans/specs/2026-05-18-feature-flag-catalogue-design.md`](../specs/2026-05-18-feature-flag-catalogue-design.md);
manifest layout, TS loader surface, Rust `build.rs` codegen approach, naming
map, consistency-check strategy, and migration ordering are pinned. **2026-05-11:**
FLAGCAT-007 resolved by
[ADR-041](../decisions/041-flag-snapshot-usage-join-contract.md): USAGE stores
the resolved flag context inline, manifest `key` is the stable join key, and
ADR-019 remains gate-affecting-only for standalone Kindling flag facts. FLAGS
and FLAGM remain archived as Complete; the five flag definitions and
per-surface modules referenced below are still the current state on `main`.

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
- Optional advisory seed from `clawpatch map` output: use
  `.clawpatch/features/*.json` during design/discovery to inventory candidate
  surfaces, entrypoints, tags, and trust boundaries before manually curating the
  shipped flag definitions into `flags/manifest.json`

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
- Treating `.clawpatch/features/*.json` as source of truth, runtime input, or a
  CI gate — Clawpatch output is advisory discovery data only; FLAGCAT remains
  human-curated and APS-governed

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
- A one-off inventory note or section in the FLAGCAT design note that records
  any useful `clawpatch map` findings and how they were accepted, declined, or
  deferred during manual manifest curation

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
- [ ] If `clawpatch map` is used during discovery, the design note records that
      it was advisory only and lists which mapped surfaces informed manifest
      categorisation; no Clawpatch state is consumed by runtime code or CI
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
- Clawpatch feature IDs are unstable local review artefacts. They may seed an
  inventory conversation, but manifest `key` values, owners, classes, defaults,
  targeting, status, and `createdFor` must be chosen explicitly under feature
  flag governance.

## Design Spec

Two design artefacts now cover FLAGCAT:

1. **FLAGCAT-001 design note** —
   [`plans/specs/2026-05-18-feature-flag-catalogue-design.md`](../specs/2026-05-18-feature-flag-catalogue-design.md)
   pins manifest layout, TS loader surface, Rust `build.rs` codegen, naming
   map, consistency-check strategy, and migration ordering.
2. **Feature gating model spec + ADR-048** —
   [`plans/specs/2026-05-19-feature-gating-model.md`](../specs/2026-05-19-feature-gating-model.md)
   and
   [`plans/decisions/048-feature-group-architectural-model.md`](../decisions/048-feature-group-architectural-model.md)
   extend FLAGCAT scope to include `groups.json`, `audiences.json`, and
   `environments.json` alongside `manifest.json`; adds `primaryGroup` and
   optional `tags` fields to `FeatureFlagDefinition`; renames
   `EnvironmentNameSchema` `prod` → `production`; and pins the day-1 gating
   policy plus the FeatureBoard translation table.

The FLAGCAT-001 note covers:

- Manifest JSON layout vs. upstream OpenFeature `flags.json` (what we extend,
  what we leave alone so upstream tooling keeps working)
- Whether to run `clawpatch map` as a one-off advisory input, and if so how its
  `{kind, source, entrypoints, tags, trustBoundaries}` records are mapped into a
  human-reviewed feature inventory without becoming canonical state
- Rust codegen approach (`build.rs` + `serde_json` vs. minimal parser) and how
  constants are named to match the TS accessors
- How the consistency check runs (Vitest over the JSON + generated files, or
  a standalone `node` script invoked from CI)
- Migration ordering — bootstrap catalogue package → flip TS surfaces → add
  Rust codegen → flip CLI → add consistency check → delete per-surface modules

## Ready Checklist

Status promoted Draft → **Ready** 2026-05-28.

- [x] Design note documents the manifest layout, codegen approach, and
      consistency-check strategy (FLAGCAT-001) —
      [`2026-05-18-feature-flag-catalogue-design.md`](../specs/2026-05-18-feature-flag-catalogue-design.md)
- [x] Rust codegen approach pinned against the `eddacraft-anvil-kernel-types`
      build profile — the design note §"Rust codegen" gives the full
      `build.rs` workspace-root walk (upward search for `[workspace]` +
      `cargo:rerun-if-changed` on the resolved absolute path), the
      `[build-dependencies] serde_json` placement, and a sibling
      `crates/anvil-flags-catalogue/` fallback. The walk is **specified, not
      yet prototyped against a live build**; FLAGCAT-004 carries that residual
      risk explicitly (`Confidence: low`) and runs the verification as its
      first action.
- [x] Release-freeze deferral cleared — `v0.7.0-beta` has shipped (current
      tag `v0.7.2-beta`), so the `EnvironmentName` contract changes in
      FLAGCAT-002 are no longer inside a freeze window.
- [x] Adoption guide owner — `docs/guides/feature-flag-inventory.md` already
      exists and is owned by this module; FLAGCAT-006 updates it in place.

## Risks & Mitigations

| Risk                                                         | Mitigation                                                                         |
| ------------------------------------------------------------ | ---------------------------------------------------------------------------------- |
| Rust `build.rs` can't reliably walk from `CARGO_MANIFEST_DIR` to the workspace root to find `flags/manifest.json` | Standard workaround: walk up until a `Cargo.toml` with `[workspace]` is found, emit `cargo:rerun-if-changed` for the resolved path. FLAGCAT-001 prototypes this; fallback is a thin `crates/anvil-flags-catalogue/` crate that owns the JSON and re-exports it to TS via `"files"` — retreat to that only if the root-level path genuinely proves painful |
| OpenFeature CLI lands Rust codegen mid-migration             | Harmless — the JSON layout is compatible; we'd just replace our `build.rs` with the upstream tool without schema changes |
| Consistency check is flaky (timezone, formatter drift)       | Compare parsed JSON + generated constants by structural equality, not stringwise; run through the same formatter as the source |
| Docs-site edge bundle regresses                              | Ship the catalogue as an ESM package with no Node-only imports on the consumer path (same constraint FLAGM-004 already met) |
| Migration lands partially and the duplicate definitions sit on `dev` | Each work item is behaviour-preserving and ships independently; no cutover needs both halves landed in the same PR |
| Clawpatch feature mapping is mistaken for product truth      | Keep it in FLAGCAT-001 discovery only: copy any useful observations into the design note, then manually curate manifest entries under APS and feature-flag governance |

## Work Items

### FLAGCAT-001: Design note — manifest layout, Rust codegen, consistency check — Complete

- **Intent:** Before changing any runtime, agree the manifest layout, the Rust
  codegen mechanism, and the drift-detection strategy.
- **Expected Outcome:** A design note at
  `plans/specs/YYYY-MM-DD-feature-flag-catalogue-design.md` documents the
  manifest JSON location, the TS loader package's public surface, the Rust
  `build.rs` approach (or alternative), how naming maps from JSON keys to
  Rust constants, and how the consistency check runs in CI. If `clawpatch map`
  is used, the note includes an advisory inventory section that categorises the
  mapped surfaces and records which observations informed manifest curation.
- **Scope:** `plans/specs/`, `docs/guides/feature-flag-inventory.md`
- **Non-scope:** Any code change
- **Validation:** `test -f plans/specs/*-feature-flag-catalogue-design.md` — validation passed (the design note exists at `plans/specs/2026-05-18-feature-flag-catalogue-design.md`).
- **Confidence:** high
- **Status:** Done
- **Resolution (2026-05-18):**
  [`plans/specs/2026-05-18-feature-flag-catalogue-design.md`](../specs/2026-05-18-feature-flag-catalogue-design.md)
  pins the manifest at `flags/manifest.json` (repo root, OpenFeature-adjacent),
  the TS loader at `packages/anvil/flags-catalogue/` with accessors named for
  byte-compatible call-site migration, Rust codegen via a `build.rs` walk to
  the workspace root with `serde_json` as a `[build-dependencies]` entry on
  `eddacraft-anvil-kernel-types`, a deterministic JSON-key-to-Rust naming map
  (`.`/`-` → `_`), a single Vitest-driven consistency check that compares
  parsed JSON, TS accessors, and a JSON dump of the Rust codegen, and a
  five-step migration order (FLAGCAT-002 → -006). Clawpatch was deliberately
  not run for this design pass; instructions for future sweeps are recorded
  in the note's "Clawpatch advisory inventory" section.

### FLAGCAT-002: Bootstrap `@eddacraft/anvil-flags-catalogue` package — Ready

> **Sequencing note (resolved 2026-05-28):** The 2026-05-19 operator decision
> to defer FLAGCAT-002..-006 until after the `v0.7.0-beta` tag cut is now
> spent — `v0.7.0-beta`, `v0.7.1-beta`, and `v0.7.2-beta` have all shipped.
> The migration of the five shipped flags plus the `EnvironmentName` enum
> changes (`Prod` → `Production`, `Dev` → `Development`, add `Demo`, drop
> `Staging`) touch runtime construction sites (CLI eval context at
> `crates/anvil-cli/src/feature_flags.rs`, kernel resolver at
> `crates/anvil-kernel/src/feature_flags/resolver.rs`, and the Rust unit
> tests in `crates/anvil-kernel-types/src/feature_flags.rs`); doing the
> contracts-level change outside a freeze window was the only reason to wait,
> and that window is closed. The `EnvironmentName` enum still ships the
> pre-rename `Local/Preview/Dev/Staging/Prod` set on `main` (Rust:
> `crates/anvil-kernel-types/src/feature_flags.rs:57`; TS:
> `packages/anvil/contracts/src/schemas/feature-flags.schema.ts:43`), so the
> rename remains part of this item's scope.
>
> **Environment-list re-validation (2026-05-19):** Spec env inventory
> reduced from seven to five (`local`, `development`, `preview`, `demo`,
> `production`) after operator review. Renames `prod` → `production` and
> `dev` → `development` to match `NODE_ENV`/`VERCEL_ENV` native values and
> drop a translation hop in the per-surface auto-detection code. `test` and
> `staging` dropped — `NODE_ENV=test` stays aliased to `development` (test
> is a transient runtime state, not a deployment); `staging` has no real
> target today and gets added back via manifest amendment if we stand one
> up later. `demo` is retained as a near-term real target (operator
> confirmed demo-specific behaviour is planned). FLAGCAT-002 renames
> `EnvironmentName::Prod` → `Production` and `Dev` → `Development`, adds
> the new `Demo` variant, drops `Staging`, and updates the construction
> sites in the same change.

- **Intent:** Stand up the new package, import the five existing flag
  definitions into `flags/manifest.json`, and export typed accessors that
  match the shapes currently exported by the per-surface modules.
- **Expected Outcome:** `flags/manifest.json` exists and validates against
  `FeatureFlagManifestSchema`. `packages/anvil/flags-catalogue/` exports
  `CLI_LICENCE_GATE`, `DOCS_ACCESS_FLAG`, `API_SCOPE_FLAGS`,
  `API_SCOPE_NAMES`, `DEFAULT_APPROVAL_SCOPES`, and a
  `featureFlagManifest()` helper. The manifest preserves the shipped runtime
  `key` strings as ADR-041 stable join keys and has room to represent retired
  keys or key-migration notes for historical queries. No existing call site
  migrated yet.
- **Scope:** `flags/manifest.json` (new — does not exist yet),
  `packages/anvil/flags-catalogue/` (new package — does not exist yet),
  `pnpm-workspace.yaml`, `tsconfig.base.json`, plus the `EnvironmentName`
  rename in `crates/anvil-kernel-types/src/feature_flags.rs`,
  `packages/anvil/contracts/src/schemas/feature-flags.schema.ts`, and their
  construction sites (`crates/anvil-cli/src/feature_flags.rs`,
  `crates/anvil-kernel/src/feature_flags/resolver.rs`)
- **Non-scope:** Rust codegen, flipping existing call sites
- **Dependencies:** FLAGCAT-001 (Done)
- **Validation:** `pnpm nx test flags-catalogue` (new project; basename
  convention matches existing `runtime`/`contracts`/`policy` projects) plus
  `cargo test -p eddacraft-anvil-kernel-types environment_name` to prove the
  enum rename round-trips
- **Confidence:** medium
- **Status:** Ready

### FLAGCAT-003: Migrate TS surfaces onto the catalogue package — Ready

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
  `apps/anvil-api/src/routes/admin-schemas.ts`
- **Non-scope:** Rust side
- **Dependencies:** FLAGCAT-002
- **Validation:** `pnpm nx run-many -t test --projects=docs-site,@eddacraft/anvil-api,runtime`
  + successful Vercel Preview deploy for the docs-site
- **Confidence:** medium
- **Status:** Ready

### FLAGCAT-004: Rust codegen from `flags/manifest.json` — Ready

- **Intent:** Emit Rust constants (flag key, variant keys, default variant)
  from the JSON manifest at build time so the Rust CLI consumes the same
  source of truth as the TS surfaces.
- **Expected Outcome:** A `build.rs` (in `crates/anvil-kernel-types` or a new
  `crates/anvil-flags-catalogue` crate) reads `flags/manifest.json` at build
  time and emits a generated module exposing Rust constants and variant
  newtypes. Drift between JSON and generated output is detected by the
  consistency check in FLAGCAT-006, not by hand-editing.
- **Scope:** `crates/anvil-kernel-types/build.rs` (new — does not exist yet),
  `crates/anvil-kernel-types/src/feature_flags_generated.rs` (new include
  shim — does not exist yet), `crates/anvil-kernel-types/Cargo.toml`
  (`[build-dependencies] serde_json`); fallback path is a new
  `crates/anvil-flags-catalogue/` crate + workspace `Cargo.toml` if the
  in-crate `build.rs` walk proves unworkable (design note §"Rust codegen")
- **Non-scope:** Flipping the CLI to consume the generated constants (next
  task); replacing the custom resolver with the `open-feature` crate
- **Dependencies:** FLAGCAT-002 (the manifest must exist to read at build
  time)
- **Validation:** `cargo test -p eddacraft-anvil-kernel-types feature_flags_catalogue`
  passes, and a touch of `flags/manifest.json` triggers a rebuild (proves
  `cargo:rerun-if-changed` fires on the resolved path)
- **Confidence:** low (build.rs path resolution + workspace layout is the
  riskiest piece of the whole module; design + sibling-crate fallback are
  pinned in the design note, so it is Ready at low confidence, not blocked)
- **Status:** Ready

### FLAGCAT-005: Migrate Rust CLI onto generated catalogue constants — Ready

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
  (binary crate `eddacraft-anvil`, bin `anvil`); the existing
  `cli_dev_bypass_active_*` tests at
  `crates/anvil-cli/src/feature_flags.rs:352` must still pass to prove the
  `ANVIL_DEV=1` override is behaviour-preserving
- **Confidence:** medium
- **Status:** Ready

### FLAGCAT-006: Consistency check and adoption guide — Ready

- **Intent:** Make drift between the JSON manifest, TS re-exports, and Rust
  codegen loud in CI; update the inventory doc so future contributors know
  "add a flag" is a single-file edit.
- **Expected Outcome:** A Vitest spec (or standalone script invoked from CI)
  parses `flags/manifest.json`, the TS accessors, and the generated Rust
  constants, and asserts structural equality on keys, variant names, and
  default variants. The check also fails if an active/retired key is reused or
  if a key migration lacks the historical-query note required by ADR-041.
  `docs/guides/feature-flag-inventory.md` documents the add-a-flag flow and
  removes any "split across surfaces" language left over from FLAGM.
- **Scope:** `packages/anvil/flags-catalogue/`, `docs/guides/feature-flag-inventory.md`,
  optionally `.github/workflows/*.yml`
- **Non-scope:** Dashboard or admin UI for flag management
- **Dependencies:** FLAGCAT-002, FLAGCAT-003, FLAGCAT-004, FLAGCAT-005
- **Validation:** `pnpm nx test flags-catalogue` +
  `grep -q "single source of truth" docs/guides/feature-flag-inventory.md`
  (the phrase is already present; this item also removes the residual
  "split across surfaces" framing and adds the structural-equality
  consistency spec to the new `flags-catalogue` project)
- **Confidence:** high
- **Status:** Ready

<a id="flagcat-007"></a>

### FLAGCAT-007: Catalogue → Kindling snapshot shape and stable join key for USAGE-002 — Complete

> **Authorisation note (2026-05-11):** The operator explicitly requested
> `FLAGCAT-007` alongside TRACE-004 as urgent cross-module decision work while
> the FLAGCAT implementation module remains Draft. Per `plans/aps-rules.md`,
> this item is executable as a documentation/ADR contract only; the remaining
> catalogue implementation tasks stay Draft.

- **Intent:** Answer the three sub-questions USAGE-002 raised under
  OQ5 so the usage module can reference the resolved flag context per
  invocation without duplicating flag-evaluation facts. The output of
  this task is either a thin ADR or a section in the FLAGCAT design
  note pinning the contract.
- **Concrete failure mode (today):** USAGE-002 cannot decide the
  shape of its `flag_set` field without knowing whether the catalogue
  publishes a queryable snapshot or only per-evaluation rows, what
  the stable join key is, and whether non-gate-affecting flags
  (informational rollouts, off-state kill switches) ever land in
  Kindling under ADR-019's existing rule. Without an answer, USAGE-002
  either duplicates flag state on every usage row (fragile) or only
  joins gate-affecting flags (incomplete).
- **Expected Outcome:**
  - Sub-question (a) — resolved-snapshot shape: written answer on
    whether the catalogue (today, post-FLAGCAT-002..-005) persists
    a per-invocation resolved snapshot of all active flag values
    to Kindling, or only individual evaluation rows for
    gate-affecting outcomes. If only the latter, the ADR records
    whether USAGE-002 should be the trigger to add a snapshot
    publisher, or whether USAGE-002 stores the snapshot inline on
    each row.
  - Sub-question (b) — stable join key: the canonical identifier
    USAGE rows use to reference a flag definition. Likely candidates:
    the manifest entry's `key` (string, today's de facto identifier),
    a generated UUID at first publish, or the manifest's existing
    `createdFor` task ID. Decision pinned, with a rule on what
    happens at rename / retirement so historical USAGE rows stay
    queryable.
  - Sub-question (c) — ADR-019 scope: a written answer on whether
    USAGE rows that touch a non-gate-affecting flag have anything
    to join to. If the answer is "no", either ADR-019's
    gate-affecting-only rule is widened (with a separate Council
    round), or USAGE-002 explicitly scopes to gate-affecting flags
    and documents the gap.
  - The answer is captured in either a new ADR (numbered after the
    current latest) or a dedicated section in `flags/manifest.json`'s
    design note, whichever the founder prefers when the task is
    picked up.
- **Resolution:** [ADR-041](../decisions/041-flag-snapshot-usage-join-contract.md)
  records the durable contract. The catalogue/FLAGS do not publish a
  separate per-invocation snapshot row; USAGE-002 publishes the resolved
  flag context inline on the usage observation. The manifest entry `key`
  is the stable join key; key changes create new logical flags, retired
  keys are reserved, and `createdFor` stays provenance only. ADR-019 is
  not widened: non-gate-affecting flags have inline usage context only,
  not separate Kindling join rows.
- **Coordinates with:** USAGE-002 — this task resolves USAGE-002's OQ5
  precondition. USAGE-002 still depends on USAGE-001 row-shape work and
  OQ1's observation-kind decision before promotion to Ready.
- **Coordinates with:** ADR-019 (flags observability alignment) — no
  widening was needed; ADR-019 now cross-links ADR-041 as a
  clarification.
- **Coordinates with:** ADR-035 (three-pipe rule) — the snapshot,
  wherever it lives, must obey the matrix (governance facts on
  Kindling, not on tracing).
- **Scope:** `flags/manifest.json` schema (if changes are needed),
  `crates/anvil-kernel-types` (if Rust codegen needs to expose the
  join key), `plans/decisions/` (new ADR if needed),
  `plans/decisions/019-flags-observability-alignment.md` (cross-link
  or amendment).
- **Non-scope:** Implementing the snapshot publisher itself if one
  is needed — that's a follow-up task whose scope falls out of this
  task's answers; FLAGCAT-007 only pins the contract.
- **Dependencies:** None for the documentation contract. FLAGCAT-002
  consumes the contract later by preserving manifest `key` as the stable
  identifier; it is not required to decide the contract.
- **Validation:** `rg 'ADR-041|OQ5|FLAGCAT-007|flag_set|manifest key|manifest \`key\`' plans/decisions plans/modules plans/index.aps.md`
  plus review that ADR-041 answers snapshot shape, join key, and ADR-019 scope.
- **Confidence:** high — the contract is documented; implementation
  remains intentionally out of scope.
- **Status:** Done

### FLAGCAT-008: Revisit `cli.licence-gate` membership at GA (welcome / status / check)

- **Status:** Draft — deferred to GA. No beta-window action (see Disposition).
- **Tracking:** GH issue [#1795](https://github.com/eddacraft/anvil-001/issues/1795)
- **Disposition (2026-06-01):** The auth wall — including `welcome` — is
  **intended for the beta**. Gating the CLI behind `cli.licence-gate`
  while Anvil is in invite/edict-only beta is a deliberate
  controlled-cohort access-control choice, not a defect. The earlier
  "contradicts planless-first" framing is **withdrawn**: planless-first
  is Anvil's zero-config *product posture* (ADR-001), not a per-feature
  gate, and was retired as an evaluation lens in PR #2192. This item does
  not act during beta; it is the placeholder for the GA decision about
  which commands come off the gate once beta access control is lifted.
- **Intent:** At GA. `CLI_GATED_COMMANDS` at
  `crates/anvil-cli/src/feature_flags.rs:38` lists nineteen commands,
  including `welcome`, `status`, `check`, `init`, and `start`. When the
  beta access wall is removed, decide which of these become
  unauthenticated entry points — `welcome` (the welcome screen) is the
  obvious first candidate, with `status` (read-only) and `check`
  (`--help`: "planless mode") as further candidates, possibly via a
  `planless` / `full` sub-mode split.
- **Expected Outcome:** At GA, a decided gated-command set, plus a
  consistent exit-code contract across gated commands: today `welcome`
  returns 0 while `init` / `start` return 3 for the same auth condition.
  (The exit-code inconsistency is a latent nit worth tracking regardless
  of the GA membership decision.)
- **Identified From:** [2026-05-21 new-user journey audit](../audits/2026-05-21-new-user-journey-audit.md)
  finding #1 (raised as a planless-first concern; reframed beta-intentional 2026-06-01).
- **Evidence pointers:**
  - `crates/anvil-cli/src/feature_flags.rs:38` (gated-command list).
  - `crates/anvil-cli/src/main.rs:278` (`requires_auth`).
  - `crates/anvil-cli/src/main.rs:320, 336` (the two
    "Authentication required." emit sites and their exit codes).
- **Coordinates with:** MLP2-072 (MCP gate-shape) — if the CLI's gate
  membership changes at GA, the MCP server's auth-required response may
  want to mirror the new contract so editor agents see a coherent story
  across CLI and MCP surfaces.
- **Validation:** When actioned at GA, the existing `requires_auth_*`
  tests at `crates/anvil-cli/src/main.rs:884-961` are updated to match the
  new membership. A CLI integration test runs `anvil welcome --no-tui` on
  a machine with no credentials and asserts the welcome screen prints
  (not the auth-required message), exit 0.
- **Confidence:** high — the gating list is a single edit in
  `CLI_GATED_COMMANDS`; the open question is purely which subset is the
  right GA entry set. Out-of-scope: redesigning the licence model, and any
  beta-window change to the gate.
