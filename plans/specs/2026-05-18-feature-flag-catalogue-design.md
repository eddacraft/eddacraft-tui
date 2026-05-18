<!-- APS: Design spec for FLAGCAT module -->

# Feature Flag Catalogue Design

Date: 2026-05-18
Module: `FLAGCAT`
Status: Ready
Resolves: FLAGCAT-001
Coordinates with:
[`plans/specs/2026-04-09-feature-flagging-design.md`](./2026-04-09-feature-flagging-design.md),
[`plans/specs/2026-04-20-feature-flag-migration-design.md`](./2026-04-20-feature-flag-migration-design.md),
[ADR-041](../decisions/041-flag-snapshot-usage-join-contract.md)

## Goal

Retire the per-surface flag definition modules
(`apps/docs-site/lib/feature-flags.ts`,
`apps/anvil-api/src/lib/feature-flags.ts`, the inline literal in
`crates/anvil-cli/src/feature_flags.rs`) in favour of a single authoritative
flag catalogue. The catalogue is a JSON manifest at the repo root, consumed by
typed loaders on the TypeScript side and by Rust constants emitted from
`build.rs` codegen on the Rust side. Adding a new flag is a single-file edit
plus a regenerate.

Behaviour-preserving: no flag gains or loses a class, variant, default,
targeting rule, or status during the migration. ADR-041's `key`-as-join
contract holds — manifest entries preserve every shipped runtime `key` string
byte-for-byte.

## Context

FLAGS shipped the shared resolver, manifest schema, snapshot loader, and
telemetry. FLAGM migrated every ad-hoc gate onto the shared resolver. The
remaining gap is structural: the five shipped flag definitions live in three
packages and duplicate the `FeatureFlagDefinition` shape in their host
language. FLAGCAT does not change *how* flags evaluate — only *where the
definitions live*.

The five shipped flags as of this note:

- `cli.licence-gate` — `crates/anvil-cli/src/feature_flags.rs`
- `docs.access` — `apps/docs-site/lib/feature-flags.ts`
- `api.scope.beta` / `api.scope.preview` / `api.scope.internal` —
  `apps/anvil-api/src/lib/feature-flags.ts`

The catalogue is OpenFeature-adjacent on purpose. The upstream
`flags.json`/`openfeature generate` toolchain does not yet ship a Rust
generator, so we own the codegen for now; the JSON layout stays compatible so
a future swap to upstream tooling is a config change, not a rewrite.

## Manifest JSON layout

### Location

A single file at the repo root:

```text
flags/manifest.json
```

The `flags/` directory is reserved for catalogue artefacts. The current scope
is just `manifest.json`. Future overlay shapes
(`flags/fixtures/`, `flags/environments/`) are out of scope for FLAGCAT but
the directory leaves room for them without re-litigating location later.

Repo-root placement (rather than `packages/anvil/flags-catalogue/manifest.json`)
matches OpenFeature's upstream convention, signals that the manifest is
cross-cutting product data rather than per-package data, and keeps the Rust
`build.rs` walk a deterministic upward search instead of a sideways
package-graph lookup.

### Schema

The on-disk file is a `FeatureFlagManifest` per
`packages/anvil/contracts/src/schemas/feature-flags.schema.ts`:

```jsonc
{
  "schemaVersion": 1,
  "flags": [
    { /* FeatureFlagDefinition */ }
  ]
}
```

`schemaVersion` mirrors `FEATURE_FLAG_SCHEMA_VERSION`. Bumping the schema is a
contracts-level change with its own ADR; the JSON file follows.

Each `flags[]` entry is exactly a `FeatureFlagDefinition` as that schema
already accepts — `key`, `owner`, `intent`, `class`, `valueType`, `variants`,
`defaultVariant`, `status`, `createdFor`, optional `expiryOrReviewDate`,
optional `description`, optional `targeting`. The schema already enforces
uniqueness of `key`s across `flags[]`; no additional uniqueness check is
needed.

Two governance rules apply to the on-disk layout that aren't strictly
enforced by Zod today (FLAGCAT-006 will lift them into the consistency check):

1. **Sort flags by `key`.** Stable ordering keeps diffs reviewable when a new
   flag is added and lets the consistency check compare structures
   positionally without normalising.
2. **Retired keys stay listed.** Per ADR-041 D-2, retired keys are reserved
   forever. They live in the manifest with `status: "retired"` until retention
   policy retires them; reusing a retired key is forbidden. The consistency
   check (FLAGCAT-006) fails if a `status: "retired"` entry has the same
   `key` as any other entry, or if a retired key is reused later.

### Difference from upstream OpenFeature `flags.json`

OpenFeature's vanilla `flags.json` carries a thinner per-flag shape. Anvil's
extended `FeatureFlagDefinition` is a superset — every extra field
(`class`, `owner`, `intent`, `createdFor`, `status`, `targeting`,
`expiryOrReviewDate`) is additive metadata that upstream parsers will ignore
rather than reject. We do not collapse to the vanilla shape: the extra fields
are governance guarantees already invested in via FLAGS, and they are what
makes the catalogue worth owning in the first place.

A `.openfeature.yaml` ships alongside `flags/manifest.json` purely as opt-in
config for anyone who wants to run `openfeature generate` locally on the TS
side. It is documented as advisory; no CI step depends on it.

## TS loader package

### Identity

```text
packages/anvil/flags-catalogue/
  package.json     name: "@eddacraft/anvil-flags-catalogue"
  src/
    index.ts
    manifest.ts    // imports ../../../../flags/manifest.json
    catalogue.ts   // typed accessors
  tests/
    manifest.test.ts
```

The package is registered in `pnpm-workspace.yaml` and `tsconfig.base.json`'s
`paths`, alongside `@eddacraft/anvil-contracts` and
`@eddacraft/anvil-runtime`. The Nx project name is `flags-catalogue` to match
the directory name and the existing `feature-flagging`/`feature-flag-migration`
naming pattern.

Edge-bundle compatibility: the consumer path imports only the JSON file plus
`@eddacraft/anvil-contracts` types. No Node-only globals (`process.env`, `fs`,
`path`) appear in the catalogue's runtime surface; environment derivation
stays where it lives today (per-surface helpers). This preserves the constraint
FLAGM-004 already met for the docs-site Vercel edge bundle.

### Public surface

```ts
// packages/anvil/flags-catalogue/src/index.ts
import type {
  FeatureFlagDefinition,
  FeatureFlagManifest,
} from '@eddacraft/anvil-contracts';

// Raw manifest, validated at module load via FeatureFlagManifestSchema.
// Validation runs once at import time; bad shapes fail loudly there, not
// at first resolver call.
export function featureFlagManifest(): FeatureFlagManifest;

// Lookup helpers. Both throw a typed error if the key is missing —
// callers must reference flags that exist in the manifest.
export function flagByKey(key: string): FeatureFlagDefinition;
export function tryFlagByKey(key: string): FeatureFlagDefinition | undefined;

// Typed accessors for every shipped flag. Names follow §"Naming map".
export const CLI_LICENCE_GATE: FeatureFlagDefinition;
export const DOCS_ACCESS_FLAG: FeatureFlagDefinition;
export const API_SCOPE_FLAGS: Readonly<Record<ApiScopeName, FeatureFlagDefinition>>;
export const API_SCOPE_NAMES: readonly ['beta', 'preview', 'internal'];
export const DEFAULT_APPROVAL_SCOPES: readonly ApiScopeName[];

// Key constants — preserved as named exports so existing `*_KEY` imports
// migrate with a path change, not a rename.
export const CLI_LICENCE_GATE_KEY = 'cli.licence-gate';
export const DOCS_ACCESS_FLAG_KEY = 'docs.access';
export const API_SCOPE_FLAG_PREFIX = 'api.scope.' as const;

export type ApiScopeName = (typeof API_SCOPE_NAMES)[number];
```

The accessor shapes are deliberately byte-compatible with what
`apps/docs-site/lib/feature-flags.ts` and
`apps/anvil-api/src/lib/feature-flags.ts` export today. FLAGCAT-003 then has
two acceptable migration shapes for each surface:

- **Preferred:** delete the per-surface `feature-flags.ts` and update call
  sites to import directly from `@eddacraft/anvil-flags-catalogue`.
- **Acceptable:** keep `feature-flags.ts` as a thin re-export layer if the
  surface owns helper functions (e.g. `evaluateDocsAccess`,
  `resolveApiScope`) that should remain co-located with their callers.
  The literal flag definition still moves to the catalogue; the surface
  module is reduced to "re-export from catalogue + surface-specific
  evaluation helpers".

### Resolver helpers stay where they live

`resolveFlag`, `ResolutionDetails`, `FlagOverrides`, `resolveFlagOrThrow`, the
snapshot loader, and the OpenFeature provider all stay in
`@eddacraft/anvil-runtime/feature-flags`. FLAGCAT does not move them.
Acceptance criterion 7 in the module spec ("the resolver's existing
OpenFeature-shaped exports are unchanged") is satisfied by the catalogue
package depending on `@eddacraft/anvil-contracts` types only, with no
resolver re-exports.

## Rust codegen

### Crate placement

Rust constants are emitted into `eddacraft-anvil-kernel-types` via a new
`build.rs`. The kernel-types crate is the right home because:

- It already exports `FeatureFlagDefinition`, `FlagClass`, `FlagStatus`,
  `FlagVariant`, `FlagValueType`, and `EnvironmentName` — the catalogue's
  generated code uses these types directly.
- Every consumer of the Rust flag model already depends on it
  (`anvil_kernel::feature_flags`, `anvil-cli`).
- Adding a sibling crate purely to hold codegen output would force every
  consumer to add a second dependency for no behavioural gain.

If the `build.rs` workspace-root walk proves painful (see §"Risks"), we fall
back to a new `crates/anvil-flags-catalogue/` crate that owns the JSON file
and re-exports it to TS via `"files"` in its `package.json`. This is the
fallback noted in the module's Risks table.

### Workspace-root resolution

`build.rs` walks upward from `CARGO_MANIFEST_DIR` until it finds a
`Cargo.toml` containing `[workspace]`, then resolves
`<root>/flags/manifest.json`:

```rust
// crates/anvil-kernel-types/build.rs
fn workspace_root() -> PathBuf {
    let mut dir: PathBuf = env::var_os("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR")
        .into();
    loop {
        let candidate = dir.join("Cargo.toml");
        if candidate.is_file() {
            let text = fs::read_to_string(&candidate).expect("read root Cargo.toml");
            if text.contains("[workspace]") {
                return dir;
            }
        }
        if !dir.pop() {
            panic!("workspace root not found above {CARGO_MANIFEST_DIR:?}");
        }
    }
}

fn main() {
    let root = workspace_root();
    let manifest_path = root.join("flags").join("manifest.json");
    println!("cargo:rerun-if-changed={}", manifest_path.display());
    // …emit generated.rs into OUT_DIR…
}
```

`cargo:rerun-if-changed` is emitted with the resolved absolute path. Stale
caches when the manifest changes are then a Cargo invariant, not something
the consistency check has to police.

### JSON reader

The kernel-types crate already has `serde` as a runtime dep and `serde_json`
as a `dev-dependency`. The codegen path needs `serde_json` at *build* time,
not runtime. We add it as a `[build-dependencies]` entry on the kernel-types
crate, which keeps it out of the consumer dependency graph:

```toml
# crates/anvil-kernel-types/Cargo.toml
[build-dependencies]
serde_json = { workspace = true }
```

This satisfies the "no new runtime dependencies on the Rust CLI" constraint
in the module spec — `[build-dependencies]` are linker-isolated from the
consumer crate.

### Generated output

The `build.rs` writes one file:

```text
$OUT_DIR/feature_flags_generated.rs
```

It is included from a hand-written sibling at
`crates/anvil-kernel-types/src/feature_flags_generated.rs` via:

```rust
include!(concat!(env!("OUT_DIR"), "/feature_flags_generated.rs"));
```

The generated module's surface is intentionally narrow — just constants and
identity-matching helpers, no runtime evaluation logic. Example for
`cli.licence-gate`:

```rust
// generated — do not hand-edit
pub mod cli_licence_gate {
    pub const KEY: &str = "cli.licence-gate";
    pub const DEFAULT_VARIANT: &str = "enabled";
    pub mod variants {
        pub const ENABLED: &str = "enabled";
        pub const DISABLED: &str = "disabled";
    }
    /// Builds the catalogue-sourced `FeatureFlagDefinition`. Identical to
    /// the inline literal that `crates/anvil-cli/src/feature_flags.rs`
    /// produces today; FLAGCAT-005 cuts the CLI literal over to this.
    pub fn definition() -> crate::FeatureFlagDefinition { /* … */ }
}

pub mod all {
    pub const KEYS: &[&str] = &[
        "api.scope.beta",
        "api.scope.internal",
        "api.scope.preview",
        "cli.licence-gate",
        "docs.access",
    ];
    pub fn definitions() -> Vec<crate::FeatureFlagDefinition> { /* … */ }
}
```

`all::KEYS` is sorted to match the manifest ordering rule. `all::definitions`
returns owned values rather than borrows so callers do not pay a static
lifetime cost for runtime use.

## Naming map — JSON `key` to Rust constants

Manifest `key`s use dotted lowercase (`cli.licence-gate`, `docs.access`,
`api.scope.beta`). The Rust namespace is derived deterministically so the
consistency check can re-derive it without a side table:

| JSON `key`         | Rust module path           | Notes                                  |
| ------------------ | -------------------------- | -------------------------------------- |
| `cli.licence-gate` | `cli_licence_gate`         | `.` → `_`; `-` → `_`                   |
| `docs.access`      | `docs_access`              |                                        |
| `api.scope.beta`   | `api_scope_beta`           |                                        |
| `api.scope.preview`| `api_scope_preview`        |                                        |
| `api.scope.internal`| `api_scope_internal`      |                                        |

Variant names follow the same transformation:
`enabled` → `variants::ENABLED`, `disabled` → `variants::DISABLED`. If a
future variant key contains characters the transformation cannot map
unambiguously, the consistency check fails and the codegen refuses to emit;
the manifest schema's `FlagVariantSchema` already constrains `key` to a
slug-like shape, so this is theoretical for the current five flags but worth
pinning before adding the sixth.

The TS accessor names re-use the existing per-surface module names so call
sites migrate with a path change only:

| JSON `key`              | TS accessor              |
| ----------------------- | ------------------------ |
| `cli.licence-gate`      | `CLI_LICENCE_GATE`       |
| `docs.access`           | `DOCS_ACCESS_FLAG`       |
| `api.scope.{beta,…}`    | `API_SCOPE_FLAGS[name]`  |

`CliGateFlag` (the CLI-local wrapper carrying `CLI_GATED_COMMANDS`) stays in
`crates/anvil-cli/src/feature_flags.rs` — that struct is CLI host metadata,
not catalogue data. FLAGCAT-005 replaces the *literal definition* inside
`cli_licence_gate_flag()` with `cli_licence_gate::definition()`; the
`CliGateMetadata` struct, `CLI_GATED_COMMANDS`, and the CLI's evaluation
helpers stay where they live.

## Consistency check

A single TS test file owns the check:

```text
packages/anvil/flags-catalogue/tests/manifest.test.ts
```

It runs in the standard `pnpm nx test flags-catalogue` lane. The check
exercises three layers:

1. **JSON validates.** Parse `flags/manifest.json`, run
   `FeatureFlagManifestSchema.safeParse`, fail on schema errors. Catches
   hand-edits that break the on-disk shape.
2. **TS accessors match.** For each named accessor, assert its `key`,
   `defaultVariant`, and sorted `variants[].key` array equal the
   corresponding manifest entry. Structural equality on parsed objects, never
   stringwise — formatter drift cannot break the check.
3. **Rust codegen matches.** A small script (`scripts/dump-flags.rs` or the
   sibling crate's own test binary) emits the generated keys, variants, and
   default-variants as JSON. The TS test reads that JSON and asserts
   structural equality against the manifest. The Rust dump runs as a
   `pretest` step via `pnpm nx test flags-catalogue` so a single command
   covers all three layers locally.

The Rust dump path is unusual but cheap — generating a JSON description of
the codegen output keeps the consistency check language-neutral and side-steps
the awkwardness of trying to read Rust source from a TS test. CI calls the
same npm/Nx target, so no separate GitHub Actions wiring is needed.

Additional assertions the check enforces (turned on in FLAGCAT-006):

- **Sort order.** `manifest.flags[].key` must be sorted ascending; the test
  fails on first out-of-order entry.
- **Retired-key reservation.** If a flag has `status: "retired"`, the same
  `key` must not appear elsewhere in `flags[]`. Reuse is an error.
- **Variant slug safety.** Each variant `key` must round-trip through the
  naming map without ambiguity (`.`/`-` → `_`, no collisions with another
  variant's mapped name).

The check is deliberately scoped to the catalogue. It does not assert
anything about how flags are *resolved* — that is FLAGS' contract and is
already covered by `packages/anvil/runtime/src/feature-flags/*.test.ts`.

## Migration ordering

FLAGCAT items ship in this order; each item is behaviour-preserving on its
own and can land in a separate PR:

1. **FLAGCAT-002** — Bootstrap the catalogue package. `flags/manifest.json`
   and `packages/anvil/flags-catalogue/` exist; the package re-exports the
   shipped flags with TS accessors. No existing call site migrated yet, so
   FLAGS-008 / FLAGM-005 behaviour is unchanged.
2. **FLAGCAT-003** — Flip the TS surfaces (docs-site, anvil-api) onto the
   catalogue package. The existing per-surface `feature-flags.ts` files
   either become thin re-exports or disappear; call sites point at
   `@eddacraft/anvil-flags-catalogue` for *definitions* and at
   `@eddacraft/anvil-runtime/feature-flags` for *evaluation*.
3. **FLAGCAT-004** — Add `build.rs` codegen on `eddacraft-anvil-kernel-types`.
   Generated module is present but not yet consumed. CLI continues to use
   the hand-written literal in `feature_flags.rs`.
4. **FLAGCAT-005** — Flip the Rust CLI to consume
   `cli_licence_gate::definition()`. `CliGateMetadata` and
   `CLI_GATED_COMMANDS` stay where they live.
5. **FLAGCAT-006** — Land the consistency check in CI, update
   `docs/guides/feature-flag-inventory.md` to document the single-file
   add-a-flag flow, delete any remaining per-surface duplicates.

If FLAGCAT-004 hits a `build.rs` issue we cannot resolve, FLAGCAT-005 stalls
behind it (the CLI still works — the inline literal is preserved). The TS
half remains shipped via FLAGCAT-002/003 either way, so the migration cannot
strand the codebase between two sources of truth on the TS side.

## Clawpatch advisory inventory

`clawpatch map` was not run during this design pass. The shipped flag set is
small (five entries, all already documented in
`docs/guides/feature-flag-inventory.md`) and a clawpatch sweep would add
discovery noise rather than insight at this scope.

If a future contributor runs `clawpatch map` during a larger catalogue
expansion (e.g. when adding the `dashboard.*` or `tutorial.*` flag families
from the adopt list), they should:

- Record findings in a new section of this design note or in a dated
  follow-up under `plans/specs/`.
- Treat `{kind, source, entrypoints, tags, trustBoundaries}` records as
  *advisory* — they seed conversations about which surfaces a flag should
  touch, but `flags/manifest.json` entries (`key`, `owner`, `class`,
  `defaultVariant`, `targeting`, `status`, `createdFor`) are chosen
  explicitly under feature-flag governance.
- Not introduce any runtime or CI consumer of `.clawpatch/features/*.json`.
  The catalogue is human-curated and APS-governed; Clawpatch output is local
  review state.

Recording the deliberate non-run here closes the FLAGCAT-001 ready-checklist
item on Clawpatch without forcing a sweep we don't need.

## Acceptance for FLAGCAT-001

This note resolves FLAGCAT-001 by documenting:

- the manifest JSON location (`flags/manifest.json`), layout, and how it
  relates to upstream OpenFeature's vanilla `flags.json` (§"Manifest JSON
  layout");
- the TS loader package's public surface and migration shape
  (§"TS loader package");
- the Rust `build.rs` approach, including workspace-root walk and
  `build-dependencies` strategy (§"Rust codegen");
- the JSON-key-to-Rust-constant naming map (§"Naming map");
- how the consistency check runs, what layers it covers, and which extra
  invariants it enforces (§"Consistency check");
- the migration ordering (§"Migration ordering");
- a written record that Clawpatch was deliberately not run during design,
  with instructions for future sweeps (§"Clawpatch advisory inventory").

The follow-up tasks (FLAGCAT-002 through FLAGCAT-006) inherit these
decisions; nothing in them re-litigates layout, location, or codegen
strategy.
