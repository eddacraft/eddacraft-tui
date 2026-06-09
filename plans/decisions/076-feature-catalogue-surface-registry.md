# ADR-076: Feature catalogue — register the full surface inventory as multi-axis features

## Status

**Proposed** — 2026-06-09. The five open questions were resolved and a
**feature-dependency facet** added (see Resolved Questions); a five-reviewer
council pass then ran (feasibility, scope, coherence, adversarial,
architecture) and its findings are folded in below — most consequentially:
the dependency facet is **declared-edges + static checks only** (runtime
cascade-off deferred to its own ADR), §6 is reframed from catalogue-*derived
lists* to catalogue-*declared posture* (preserving host ownership), and the
"off ⇒ refuse uniformly" invariant gains explicit exceptions for
system-invoked and recovery-critical surfaces. Extends — does not supersede —
[ADR-048](048-feature-group-architectural-model.md).

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

Part of this is cheaper than it sounds; part is genuinely net-new. Being
precise about which (council feasibility + architecture review):

1. **Value-tiering and posture are free** — no new flag class or dimension.
   `FlagValueType` is `Boolean | String | Number | Object` and `FlagValue`
   carries each, so non-boolean behaviour ("free tier gets 2 MCP servers, pro
   gets 4") is expressible today via a `Number` feature with audience-targeted
   variants (caveat: the Rust `build.rs` codegen currently emits
   boolean/number/string and **panics on `Object`** — `object` variants need a
   codegen extension first). `targeting` maps audience → variant; group defaults
   let a feature inherit posture and override only when it diverges. The four
   reasons to flag map onto what exists: tiered benefit and auth → `Entitlement`
   (+ audience); safety → `OpsKillSwitch`; staged availability → the environment
   axis. "Edition" collapses into entitlement; "auth restriction" into
   entitlement with an auth-bearing audience (CIB-046).
2. **The surface registry, cross-file references, `requires` edges, and a
   uniform presence projection are net-new** — schema, codegen, and validation
   the catalogue does not have today. The resolver (`resolve_flag`) is strictly
   per-flag and returns a variant *string*, not a present/absent boolean; the
   `flags/*.json` inventories are loaded independently with no cross-file
   referential check. This ADR does not pretend these "map onto the existing
   schema"; they are a real build (sized in Sequencing and Consequences).

## Decision

Make the **surface/feature the catalogue's primary noun**, register the full
inventory, and drive flagging through the dimensions that already exist.

1. **Feature = the smallest unit you would ever ship or gate independently.**
   Split where flagging diverges (`mcp install` is licence-gated, `mcp serve` is
   an ungated capability → two features), merge where it never would (`drift`
   snapshot/compare/report/list → one feature).

2. **Categories partition the `cli` surface by capability.** Every entry here
   lives under ADR-048's `cli` surface, so `category` is the **capability axis**
   (closer to ADR-048's `tags` than to its surface-primary `primaryGroup`) — a
   distinction to make explicit so this does not read as rotating ADR-048's
   primary axis from surface to capability. A `foundational` category holds base
   plumbing (`doctor`, `version`, `config`, `auth`, …) that defaults every axis
   on. These are **inventoried but rarely flagged**; an open question (below) is
   whether they carry a `catalogued: false` marker so they are recorded without
   adding drift-gated maintenance for commands that have no gate, tier, or
   environment override today. Environment targeting is in principle universal
   (any feature can be scoped to `local`/`development`/`preview`/`demo`/
   `production`) — but see the runtime-gap risk: the CLI host does not yet supply
   an environment signal.

3. **No new flag class or dimension; presence and behaviour are orthogonal
   (Q1).** Reuse `Entitlement` / `OpsKillSwitch` / `Rollout` and the audience +
   environment axes. There is **no `gate: invocation | behaviour` enum field**
   (the rejected design): a surface resolves **on/off** for a caller — off for
   *any* reason (not in the audience, not enabled in this environment, killed by
   the safety switch). The mooted enum is wrong because a feature can *both* gate
   invocation and carry a tiered value, so it is not a single per-feature mode.
   **Behaviour is the orthogonal "what you get when on"**: `valueType: "boolean"`
   is pure on/off; `"number"`/`"object"` carries an in-code value (e.g. 2 vs 4
   MCP servers), and `targeting` selects *which* value among those who have it.

   Two correctness constraints the council surfaced, binding before auth-list
   work:
   - **Axis precedence is AND.** A surface is on only if audience **and**
     environment **and** kill-switch all admit it; off for any reason wins. The
     existing resolver's `targeting` is first-match-wins (OR-ish), so this AND
     projection must be built deliberately and tested — it is not free from the
     current resolver.
   - **"off ⇒ refuse" is *not* uniform** for two surface kinds: **system-invoked**
     surfaces (`hook`, run by git — a refusal is a non-zero exit that breaks
     every commit, not a UX message) and **recovery-critical** surfaces
     (`auth login`/`refresh`, `admin.credential` — you cannot refuse the thing
     that restores access; a kill-switch here is an irrecoverable lockout). The
     model therefore needs a small set of surface markers — a `system`
     invocation context and a kill-switch exemption for recovery-critical
     surfaces — and these are categorical, not optional.

4. **Keep it lean via category defaults.** A feature inherits its category's
   class/audience/environment and declares only **overrides** (e.g. `mcp.serve`
   overriding the `mcp` gated default to open). This prevents the 6 → ~43
   expansion from becoming 19 identical licence flags.

5. **Feature dependencies are explicit (`requires` edges) — declared data +
   static checks now; runtime cascade deferred.** A feature may declare
   `requires: [feature-keys]`. **In scope for this ADR:** the edges as declared
   data, the **authoring/CI blast-radius report** (turning A off reports "B, C, D
   also go dark" — the visibility that catches "disable X, break Y" *before*
   shipping), and the static **existence + acyclicity gates** (a cycle makes "is
   it on?" undecidable). This is cheap and static — no resolver change.
   **Explicitly deferred to its own ADR** (per the council's over-build finding):
   **runtime cascade-off**, which would require a *new multi-flag resolution
   layer* above the strictly-per-flag `resolve_flag` — the very config-as-DAG
   complexity ADR-048 avoided when it rejected `killable` as a field. The
   authoring-time report is what addresses the operator's actual pain; the
   runtime engine waits for a concrete consumer. The seed surfaces ship with a
   runtime **cycle guard that fails closed** if cascade is later turned on, so
   `surfaces.json` is never trusted as an unbounded graph. **Hard `requires`
   only** — no `suggests`/`enhances`/`conflicts-with`.

6. **Auth *posture* becomes catalogue-declared; each host derives its own
   enforcement.** Reframed from the draft's "auth lists become catalogue-derived"
   — the CLI crate deliberately documents that its gated-command list "is a
   property of the host, not the shared flag contract." So each surface
   **declares** its posture (class + audience) in the catalogue — which is what
   closes the unauthenticated-by-absence gap — and the CLI host **reads posture
   per surface to build its own** `requires_auth` decision, rather than importing
   a host-specific command list *from* the catalogue. Two guards the council
   requires: (a) a **`MUST_ALWAYS_BE_OPEN` floor** — a compile-time constant of
   recovery/auth/system surfaces that a `surfaces.json` edit can never gate,
   cross-checked at startup; (b) derivation must go through a **key →
   canonical-command crosswalk**, since one feature can map to several
   auth-distinct canonical names (`mcp serve` vs `mcp install` via
   `auth_gate_name`; `auth` fans to four) and some gated names (`status`,
   `whoami`) are not 1:1 with surface keys. Without the crosswalk, the
   hand-maintained mapping is merely relocated.

7. **Storage = a dedicated `flags/surfaces.json` registry (Q2).** Surfaces +
   categories + `requires` edges live in their own graph document (with the
   existence + acyclicity gates); `manifest.json` stays the lean policy-flag
   layer that **references** surface keys. The Rust `build.rs` and TS codegen +
   drift gates extend to the second file.

8. **Runtime stays config-resolved, not build-time — but editions are a future
   *consequence*, not a goal of this ADR.** A product edition would be a profile
   that resolves features on/off per audience/environment at startup, one binary.
   This is **not yet reachable**: `cli_evaluation_context` hard-codes
   `EnvironmentName::Production`, so per-environment resolution needs a host
   change to plumb an environment signal first (a runtime gap of the same shape
   as the staff-axis one — see Risks). No concrete SKU exists, so the edition
   framing is forward-looking only; the `mcp.serve`/`mcp.install` split stands on
   its *current* divergence (one is licence-gated, one is not), not on a
   hypothetical edition. When refusals are surfaced, the
   `ResolutionDetails.reason` **must** distinguish `NotEntitled` /
   `NotAvailableInEnvironment` / `KillSwitchOff` so support, telemetry, and
   security can tell them apart (they are operationally and security-distinct,
   even though all three resolve to "off"). Build-time omission (cargo
   `#[cfg(feature)]`) stays **deferred**.

**Sequencing:** (a) this ADR → (b) stand up `flags/surfaces.json` + back-capture
the inventory defaulting to present (zero behaviour change), with the
`MUST_ALWAYS_BE_OPEN` floor and `bypass_auth` regression pins for the CI-lane /
recovery surfaces in place → (c) exercise non-boolean/audience-targeted
behaviour where a real tiering exists → (d) declare posture and have the host
derive its own enforcement via the crosswalk → (e+) *defer* runtime cascade-off,
per-environment plumbing, staff-axis/RBAC, and build-time edition omission, each
to its own trigger. `surfaces.json` codegen + drift is its own multi-item slice
(comparable to FLAGCAT-002..-006), not a free extension.

## Rationale

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| **Surface-as-feature registry (chosen)** | One inventory of what the product is; auth posture is declared (hosts derive their own enforcement); tiered benefits + declared dependencies expressible; translates to FeatureBoard | Catalogue grows 6 → ~43 entries; `surfaces.json` codegen/drift is a real slice; population effort |
| Stay flag-centric, add flags ad-hoc | No migration | No inventory; every new gate re-litigates posture (the ADR-048 problem, unsolved for surfaces) |
| New `Capability`/`Edition` class + dimension | Explicit "is it in this SKU" | Redundant — edition = entitlement, auth = entitlement+audience; adds schema/codegen/drift surface for nothing (YAGNI) |
| Build-time cargo features as the primary mechanism | Truly omits code (size, deps, attack surface) | Per-edition CI matrix; cargo-feature discipline across the workspace; premature with no concrete edition |

## Consequences

- **Positive:** a single source of truth for the product's surface; auth posture
  becomes declarative (no more unauthenticated-by-absence); tiered benefits are
  expressible with the schema as-is; declared dependencies give a pre-ship
  blast-radius report; the inventory translates to FeatureBoard by configuration.
- **Negative:** the catalogue expands from 6 to ~43 entries; `surfaces.json`
  codegen + drift is a real multi-item slice, not a free extension; risk of
  over-cataloguing pure plumbing (the `catalogued: false` open question).
- **Risks:** (1) **two runtime gaps of the same shape** — the CLI evaluation
  context has no staff signal (CIB-046; → RBAC) *and* hard-codes
  `EnvironmentName::Production`, so neither staff-audience nor per-environment
  resolution works until the host is changed; any claim resting on them is
  inert today. (2) **catalogue-as-auth-source is a footgun** if mishandled — a
  semantically-valid `surfaces.json` edit could move the auth boundary silently;
  mitigated by the `MUST_ALWAYS_BE_OPEN` floor + posture-not-lists framing + the
  crosswalk. (3) **`off ⇒ refuse` has categorical exceptions** (system-invoked,
  recovery-critical) that, if missed, brick git workflows or lock users out. (4)
  axis-precedence AND-projection and `object`-value codegen are net-new work the
  prose must not present as free.
- **Mitigations:** category defaults keep declarations minimal; the dedicated
  `flags/surfaces.json` registry isolates the graph; the `MUST_ALWAYS_BE_OPEN`
  floor + `bypass_auth` regression pins protect recovery/CI surfaces; the
  granularity rule bounds entry count to ~command-count; **runtime cascade-off,
  per-environment plumbing, staff/RBAC, and build-time omission are each
  explicitly deferred to their own triggers**; `requires` is CI-gated for
  existence + acyclicity and limited to hard dependencies, declared-only for now.

## Resolved Questions (2026-06-09)

The five questions raised at draft time were worked through with the operator
and resolved as follows; a sixth concern (dependencies) was raised and folded
into the Decision.

1. **Enforcement point → no field.** Dissolved by the audience/behaviour
   orthogonality: presence (audience/environment/kill-switch) gates the surface
   on/off, and behaviour is the orthogonal value-when-on, signalled by
   `valueType` and selected by `targeting`. See Decision §3. The mooted
   `gate: invocation | behaviour` enum is rejected — a feature can both gate
   invocation and carry a tiered value, so it is not a single per-feature mode.
   (Post-council: "off ⇒ refuse" holds for the general case but is **not
   uniform** — system-invoked and recovery-critical surfaces are categorical
   exceptions; see Decision §3 and the `MUST_ALWAYS_BE_OPEN` floor in §6.)
2. **Dependencies (raised here) → explicit `requires` edges, declared + static
   only.** In scope: edges as data, the authoring-time blast-radius report, and
   existence + acyclicity gates; **runtime cascade-off deferred** to its own ADR
   (post-council scoping). Hard requires only. See Decision §5.
3. **Storage shape → dedicated `flags/surfaces.json`.** See Decision §7.
4. **Dashboard granularity → per-view.** Each native view
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

## Council Review (2026-06-09)

Five reviewers (feasibility, scope, coherence, adversarial, architecture). Core
verdict: the declarative-posture core is sound and well-aligned with ADR-048;
**accept with changes**, all folded in above. Material changes the council
drove:

- **Dependency facet narrowed** to declared edges + static blast-radius +
  acyclicity gate; runtime cascade-off deferred to its own ADR (it is a new
  multi-flag resolution layer, the config-DAG complexity ADR-048 avoided).
- **§6 reframed** from catalogue-*derived lists* to catalogue-*declared
  posture*; host keeps enforcement ownership; added the `MUST_ALWAYS_BE_OPEN`
  floor and the key→canonical crosswalk.
- **`off ⇒ refuse` exceptions** made categorical for system-invoked (`hook`) and
  recovery-critical (`auth`, `admin.credential`) surfaces.
- **Honest schema-fit re-label** (value-tiering free; registry/refs/`requires`/
  presence-projection net-new); **environment-axis runtime gap** added to Risks
  alongside the staff-axis one; **AND axis-precedence** specified;
  **`ResolutionDetails.reason`** propagation required; `object`-codegen and
  `bypass_auth` regression-pin gaps noted.

Open for the eventual council ratification to Accepted: the `catalogued: false`
treatment of `foundational` plumbing, and confirming the AND-projection against
the live resolver.

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
| `mcp` | `mcp.serve` | **ungated** | capability + env | split stands on current divergence (ungated vs gated `install`), not a hypothetical edition |
| `mcp` | `mcp.config` | licence-gated | Entitlement | — |
| `dashboard` | `dashboard.aps` (`plan dashboard`) | **staff-gated** (CIB-046, catalogued) | Entitlement/staff | already a feature |
| `dashboard` | `dashboard.architecture` / `dashboard.drift` / `dashboard.suppressions` | ungated | env | per-view; each `requires` its engine feature (RQ4) |
| `dashboard` | `dashboard.saved` (`.dashboard.json` picker, incl. `gate-summary`) | ungated | env | saved-spec capability, separate from native views |
| `save-time` | `watch` | licence-gated | Entitlement | daemon routing is a Rollout sub-behaviour |
| `save-time` | `intercept` | ungated | env | merge start/status/unblock |
| `hooks` | `hook` (git-invoked runners) | ungated | env | **system-invoked** — exempt from kill-switch refusal (refusal = broken commits) |
| `hooks` | `hooks` (manage: install/uninstall/status) | ungated | env | merge subcommands |
| `admin` | `admin.operations` | admin-key gated | Entitlement (admin) | — |
| `foundational` | `admin.credential` (`admin auth`) | **ungated** | foundational | **recovery-critical** — `MUST_ALWAYS_BE_OPEN`; configures the key (RQ5) |
| `tools` | `edda`, `capsule`, `insights` | ungated | env / Entitlement (later) | one each, user's own data |
| `setup` | `init`, `start`, `welcome`, `new`, `wizard` | licence-gated | Entitlement | onboarding |
| `foundational` | `auth` (login/logout/whoami/refresh) | ungated | env | **recovery-critical** — `MUST_ALWAYS_BE_OPEN` (can't refuse the thing that re-authenticates) |
| `foundational` | `config`, `migrate`, `update`, `uninstall`, `doctor`, `version`, `licenses`, `tutorial`, `workspace` | ungated | env-only | base layer; `catalogued: false` candidate (open) |

≈ 43 features across 9 categories (the per-view dashboard split adds a few) —
roughly command-count, the signal that the granularity is honest. `requires`
edges (e.g. each `dashboard.*` view → its engine feature) are **declared** in
`flags/surfaces.json` and drive the static blast-radius/acyclicity check;
runtime cascade-off is deferred. `MUST_ALWAYS_BE_OPEN` surfaces (`auth`,
`admin.credential`) and `system`-invoked surfaces (`hook`) are categorical
exceptions to "off ⇒ refuse".
