# ADR-076: Feature catalogue — register the full surface inventory as multi-axis features

## Status

**Draft** — 2026-06-09. The five open questions were worked through and
resolved the same day (see Resolved Questions); a **feature-dependency facet**
was added. Ready to promote to **Proposed** pending a council pass. Extends —
does not supersede — [ADR-048](048-feature-group-architectural-model.md).

## Date

2026-06-09

## Context

[ADR-048](048-feature-group-architectural-model.md) established **how a new
gating decision is structured**: a Feature Group is a defaults carrier (class +
audiences + lifecycle), flags override per-entry, and the shape is deliberately
chosen to translate to FeatureBoard's `Feature → categoryIds[]` model by
configuration rather than rewrite. FLAGCAT then landed the catalogue itself —
`flags/manifest.json` plus the `groups`/`audiences`/`environments` inventories,
codegen for TS and Rust, and CI-gated drift.

What the catalogue does **not** yet do is record **what the product is made
of**. It holds six boolean *policies* (`api.scope.*`, `cli.licence-gate`,
`docs.access`, and now `tui-dashboard.aps-dashboard` from CIB-046). A CLI
surface only appears implicitly — e.g. licence gating is one flag
(`cli.licence-gate`) carrying a hand-maintained list of 19 command names as
metadata. There is no registry of the ~38 commands (and their subcommands) as
first-class features, so:

- You cannot reason about "which surfaces exist" from the catalogue.
- Auth/gating lists (`CLI_GATED_COMMANDS`, the admin-key gate, the CIB-046 staff
  gate) are hand-maintained in Rust, not derived from a declared posture — the
  exact gap that let `anvil plan dashboard` ship unauthenticated-by-absence
  until CIB-046.
- There is no inventory to drive future product composition (e.g. an edition
  that omits MCP) or per-environment availability.

A read-only audit (2026-06-08) enumerated all 38 commands against the three auth
mechanisms (licence gate, admin-key gate, CIB-046 staff gate) and the
`cli-surface.md` class. It found no second "plan dashboard" leak, but it
surfaced the deeper point: the **class label is orthogonal to auth posture**
("Admin" is used for both gated and ungated commands), and the catalogue has no
notion of a surface at all. The desired direction — articulated by Josh — is to
**catalogue every surface/feature**, where auth is just *one* of several lenses
you may flag a feature by (the others being tiered benefit, safety kill-switch,
and environment availability).

Two facts make this cheaper than it sounds:

1. **The schema already fits.** `FlagValueType` is `Boolean | String | Number |
   Object` and `FlagValue` carries each, so non-boolean behaviour ("free tier
   gets 2 MCP servers, pro gets 4") is expressible today via a `Number` feature
   with audience-targeted variants. `targeting` already maps audience → variant.
   Group defaults (`defaultClass`/`defaultAudiences`) already let a feature
   inherit posture and override only when it diverges.
2. **No new dimension is needed.** The four reasons to flag map onto what
   exists: tiered benefit and auth → `Entitlement` (+ audience); safety →
   `OpsKillSwitch`; staged availability → the environment axis (ADR-048's
   rollout component). "Edition" collapses into entitlement (a SKU is an
   audience that lacks a feature's entitlement); "auth restriction" collapses
   into entitlement with an auth-bearing audience (exactly what CIB-046 is).

## Decision

Make the **surface/feature the catalogue's primary noun**, register the full
inventory, and drive flagging through the dimensions that already exist.

1. **Feature = the smallest unit you would ever ship or gate independently.**
   Split where flagging diverges (`mcp install` is licence-gated, `mcp serve` is
   an ungated capability → two features), merge where it never would (`drift`
   snapshot/compare/report/list → one feature).

2. **Categories group features** (extends ADR-048's `primaryGroup`; aligns with
   FeatureBoard `Feature → category`). A `foundational` category holds base
   plumbing (`doctor`, `version`, `config`, `auth`, …) that defaults every axis
   on and rarely overrides — replacing any notion of a "non-flaggable" command,
   since environment targeting is universal (any feature can be scoped to
   `local`/`development`/`preview`/`demo`/`production`).

3. **No new flag class or dimension; presence and behaviour are orthogonal
   (Q1).** Reuse `Entitlement` / `OpsKillSwitch` / `Rollout` and the audience +
   environment axes. Edition ≡ entitlement; auth ≡ entitlement + auth-bearing
   audience. Crucially, there is **no "enforcement point" field**: a surface
   resolves **on/off** for a caller — off for *any* reason (not in the audience,
   not enabled in this environment, killed by the safety switch) — and because
   every registry entry is a **surface**, off ⇒ the command is refused/hidden,
   uniformly and derivably. **Behaviour is the orthogonal "what you get when
   on"**: `valueType: "boolean"` is pure on/off (nothing more to read);
   `"number"`/`"object"` carries an in-code value (e.g. 2 vs 4 MCP servers), and
   `targeting` selects *which* value among those who have it. "If they don't
   have it, behaviour doesn't matter."

4. **Keep it lean via category defaults.** A feature inherits its category's
   class/audience/environment and declares only **overrides** (e.g. `mcp.serve`
   overriding the `mcp` gated default to open). This prevents a 6 → 40 flag
   explosion from becoming 19 identical licence flags.

5. **Feature dependencies are explicit (`requires` edges).** A feature may
   declare `requires: [feature-keys]`. A feature is effectively-available iff
   its own presence resolves on **and** every `requires` target is available,
   transitively. Two enforcement moments: **runtime cascade-off** (a dependency
   absent ⇒ the dependent is absent too — never serve a half-broken feature) and
   an **authoring/CI blast-radius report** (turning A off reports "B, C, D also
   go dark"). The graph is a **DAG**, gated for `requires`-target existence and
   **acyclicity** (a cycle makes "is it on?" undecidable). **Hard `requires`
   only** — no `suggests`/`enhances`/`conflicts-with` until a real need appears
   (that softer-relation sprawl is where feature systems balloon).

6. **Auth lists become catalogue-derived.** `CLI_GATED_COMMANDS` and the
   admin/staff gates stop being hand-maintained Rust lists and are derived from
   each surface's presence resolution (audience/environment/kill-switch) — the
   same single-source pattern CIB-046 already uses to read its key from the
   catalogue.

7. **Storage = a dedicated `flags/surfaces.json` registry (Q2).** Surfaces +
   categories + `requires` edges live in their own graph document (with the
   existence + acyclicity gates); `manifest.json` stays the lean policy-flag
   layer that **references** surface keys. The Rust `build.rs` and TS codegen +
   drift gates extend to the second file.

8. **Runtime stays config-resolved, not build-time.** A product edition is a
   profile that resolves features on/off per audience/environment at startup;
   one binary. Build-time omission (cargo `#[cfg(feature)]`) is a **deferred**
   optimization for a concrete edition, not part of this model.

**Sequencing:** (a) this ADR → (b) back-capture the inventory as catalogue
entries defaulting to present (zero behaviour change) → (c) exercise
non-boolean/audience-targeted behaviour where a real tiering exists → (d) derive
the auth lists from the catalogue → (e) *defer* build-time edition omission
until a real SKU needs it.

## Rationale

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| **Surface-as-feature registry (chosen)** | One inventory of what the product is; auth lists derive from it; editions/environments compose by config; translates to FeatureBoard | Catalogue grows 6 → ~40 entries; one-time population effort |
| Stay flag-centric, add flags ad-hoc | No migration | No inventory; every new gate re-litigates posture (the ADR-048 problem, unsolved for surfaces) |
| New `Capability`/`Edition` class + dimension | Explicit "is it in this SKU" | Redundant — edition = entitlement, auth = entitlement+audience; adds schema/codegen/drift surface for nothing (YAGNI) |
| Build-time cargo features as the primary mechanism | Truly omits code (size, deps, attack surface) | Per-edition CI matrix; cargo-feature discipline across the workspace; premature with no concrete edition |

## Consequences

- **Positive:** a single source of truth for the product's surface; auth/gating
  becomes declarative and catalogue-driven (no more unauthenticated-by-absence);
  per-environment availability and tiered benefits are expressible with the
  schema as-is; the inventory translates to FeatureBoard by configuration.
- **Negative:** the catalogue expands from 6 to ~40 entries; a one-time
  back-capture pass is needed; risk of over-cataloguing pure plumbing.
- **Risks:** (1) non-customer audiences (staff/internal) have no signal in the
  CLI evaluation context today — the CIB-046 deferral — so auth restriction to
  internal audiences can't fully resolve at runtime until RBAC lands; (2) the
  dependency graph adds an acyclicity-gate surface and a new failure mode
  (mis-declared `requires` edges) to maintain.
- **Mitigations:** category defaults keep declarations minimal; the dedicated
  `flags/surfaces.json` registry keeps the graph out of `manifest.json`; the
  `foundational` category contains plumbing without pretending it is a SKU
  lever; the granularity rule bounds the entry count to roughly command-count;
  build-time omission is explicitly deferred; staff-axis resolution is folded
  into the eventual RBAC work; `requires` edges are CI-gated for existence +
  acyclicity and limited to hard dependencies.

## Resolved Questions (2026-06-09)

The five questions raised at draft time were worked through with the operator
and resolved as follows; a sixth concern (dependencies) was raised and folded
into the Decision.

1. **Enforcement point → no field.** Dissolved by the audience/behaviour
   orthogonality: presence (audience/environment/kill-switch) gates the surface
   on/off uniformly (derivable because every entry is a surface); behaviour is
   the orthogonal value-when-on, signalled by `valueType` and selected by
   `targeting`. See Decision §3. The mooted `gate: invocation | behaviour` enum
   is rejected — a feature can both gate invocation and carry a tiered value, so
   it is not a single per-feature mode.
2. **Dependencies (raised here) → explicit `requires` edges.** Cascade-off at
   runtime + authoring-time blast-radius report; DAG with existence + acyclicity
   gates; hard requires only. See Decision §5.
3. **Storage shape → dedicated `flags/surfaces.json`.** See Decision §7.
4. **`dashboard.project` granularity → per-view.** Each native view
   (`dashboard.architecture`/`dashboard.drift`/`dashboard.suppressions`) is its
   own feature that `requires` its engine feature, so cascade-off drops a view
   exactly when its data source is dropped; the saved-spec capability
   (`gate-summary` and the `.dashboard.json` picker) is a separate
   `dashboard.saved` feature.
5. **`admin.credential` split → yes.** `admin.credential` (the `admin auth`
   subcommands) is its own `foundational`, open-audience feature distinct from
   `admin.operations` (admin-key audience) — the audience divergence is the
   boundary; you cannot require the key to set the key.
6. **Staff-axis runtime plumbing → deferred, integrate with RBAC.** Declare
   staff audiences now; escape hatches (`ANVIL_DEV`/`ANVIL_ADMIN_KEY`) carry
   runtime resolution until **RBAC** lands, which is the natural home for
   role/staff identity — at which point staff resolution integrates there (a
   dedicated staff signal in the evaluation context, not an overload of the
   customer `user_role` axis). CIB-046 follow-up.

## References

- Related ADRs: [ADR-048](048-feature-group-architectural-model.md) (extends),
  [ADR-041](041-flag-snapshot-usage-join-contract.md) (flag snapshot shape)
- Specs: [`plans/specs/2026-05-19-feature-gating-model.md`](../specs/2026-05-19-feature-gating-model.md)
- APS: CIB-046 (first catalogue back-capture + staff gate),
  `feature-flag-catalogue` (FLAGCAT)
- External: FeatureBoard `Feature → categoryIds[]` model (adoption target per
  ADR-048)

## Appendix: Seed inventory (first-draft granularity pass)

Granularity rule applied across all 38 commands + subcommands. "Now" = current
posture (evidence); "Class" = proposed *why-you-flag*. Most features inherit
their category default; only divergent ones declare an override.

| Category | Feature | Now | Class / axis | Granularity note |
|----------|---------|-----|--------------|------------------|
| `governance` | `check`, `audit`, `gate`, `export` | licence-gated | Entitlement | one each |
| `governance` | `drift` | licence-gated | Entitlement | merge snapshot/compare/report/list |
| `governance` | `architecture` | licence-gated | Entitlement | merge validate/show |
| `governance` | `policy` | licence-gated | Entitlement | one |
| `governance` | `gate-config` | licence-gated (Admin) | Entitlement | split from `gate` (config vs run) |
| `governance` | `baseline` | ungated | env | local state |
| `governance` | `audit-chain`, `l4-validate`, `validate` | ungated | env | CI/local lanes |
| `mcp` | `mcp.install` | licence-gated | Entitlement | — |
| `mcp` | `mcp.serve` | **ungated** | capability + env | edition/omit candidate |
| `mcp` | `mcp.config` | licence-gated | Entitlement | — |
| `dashboard` | `dashboard.aps` (`plan dashboard`) | **staff-gated** (CIB-046, catalogued) | Entitlement/staff | already a feature |
| `dashboard` | `dashboard.architecture` / `dashboard.drift` / `dashboard.suppressions` | ungated | env | per-view; each `requires` its engine feature (Q3) |
| `dashboard` | `dashboard.saved` (`.dashboard.json` picker, incl. `gate-summary`) | ungated | env | saved-spec capability, separate from native views |
| `save-time` | `watch` | licence-gated | Entitlement | daemon routing is a Rollout sub-behaviour |
| `save-time` | `intercept` | ungated | env | merge start/status/unblock |
| `hooks` | `hooks` | ungated | env | merge manage + run |
| `admin` | `admin.operations` | admin-key gated | Entitlement (admin) | — |
| `foundational` | `admin.credential` (`admin auth`) | **ungated** | foundational | split from `admin.operations` — configures the key (Q4) |
| `tools` | `edda`, `capsule`, `insights` | ungated | env / Entitlement (later) | one each, user's own data |
| `setup` | `init`, `start`, `welcome`, `new`, `wizard` | licence-gated | Entitlement | onboarding |
| `foundational` | `auth`, `config`, `migrate`, `update`, `uninstall`, `doctor`, `version`, `licenses`, `tutorial`, `workspace` | ungated | env-only (default on everywhere) | base layer; never a SKU lever |

≈ 43 features across 9 categories (the per-view dashboard split adds a few) —
roughly command-count, the signal that the granularity is honest. `requires`
edges (e.g. each `dashboard.*` view → its engine feature) live in
`flags/surfaces.json`, not shown here.
