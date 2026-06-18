# ADR-055: OSS carve-out for read-only consumers of the public APS format

## Status

Accepted (2026-06-18; legal gate cleared by operator approval)

> The legal gate (explicit sign-off from `legal@eddacraft.ai` for the
> `LicenseRef-Proprietary` → Apache-2.0 carve-out) has been satisfied. The
> carve-out is now in effect. The APSDASH starter kit and APS format tooling
> may proceed through the pre-publication checklist and publication steps.

## Date

2026-06-18

## Context

We want to ship two Anvil-grown assets that operate on the public APS format as
optional, reusable components in the public
[`anvil-plan-spec`](https://github.com/eddacraft/anvil-plan-spec) repository, so
that anyone adopting the APS format gets tooling for it:

1. **The read-only `anvil plan dashboard`** — a terminal view of APS plan state.
   Staged as a starter kit under `tools/starters/aps-dashboard/` (APSDASH); a
   Council review blocked it on a licensing problem.
2. **The APS workflow tooling** — `scripts/aps/drift-check.mjs` (advisory plan
   drift), `index-counts.mjs` (derives `done/total` from work-item statuses),
   `active-lint.mjs` (active-vs-archive lint scope), and the shared
   `lib/modules.mjs` parser, plus the doc-index scripts
   (`scripts/docs/docs-index.mjs`, `check-index-freshness.mjs`). These are
   zero-dependency Node ESM that read the same canonical APS layout and would make
   natural reusable starters. No kit is staged for them yet; this ADR settles the
   policy before one is.

Both run into the same boundary:

- The dashboard source lives in `crates/anvil-cli` (`plan_dashboard.rs`, the
  snapshot builder) and `crates/anvil-tui` (`surfaces/plan_dashboard/*`, the
  render layer); the tooling lives in `scripts/aps/*` and `scripts/docs/*`. All of
  it is in the `anvil-001` monorepo, whose `LICENSE` is "all rights reserved …
  unauthorised copying, modification, distribution … strictly prohibited"
  (`license.workspace = "LicenseRef-Proprietary"`).
- **ADR-018** ("product IP architecture") names `anvil-cli` and `anvil-tui` as
  the *closed product* and states "source never leaves the private repo." It
  scopes the OSS surface to exactly three repos — `eddacraft-tui`,
  `anvil-plan-spec`, `kindling` — and lists "dashboard" among the closed product.
- **ADR-047** authorises a public mirror only for `eddacraft-tui` (an
  already-Apache-2.0 widget library). It does **not** authorise relicensing
  proprietary application code or scripts.

So the dashboard kit, as staged, relicenses proprietary code as Apache-2.0 for a
public repo with no authorising decision — and publishing the tooling would do
the same. A decision is needed on whether read-only consumers of the public APS
format belong in the OSS surface at all, and if so, under what boundary and
process.

The forces in tension:

- **Network effect / charter fit.** `anvil-plan-spec`'s charter is the APS
  format plus its parser and validator — "anyone can adopt APS … without using
  Anvil." A reference *viewer* of the format is a natural extension of that
  charter, and an open viewer makes the format more adoptable (the
  OpenTelemetry-vs-proprietary-APM logic ADR-018 already invokes).
- **Product moat.** ADR-018's closed boundary protects the kernel, policy
  engine, architecture engine, checks, auth, and the **product** dashboards
  (the web dashboard in `apps/website`; the watch dashboard and gate explorer in
  `anvil-tui`, which render the output of the scan/policy engines). None of that
  moat is exposed by a viewer that only renders plan-file state.
- **What "dashboard" means in ADR-018.** ADR-018's "dashboard" refers to the
  product surfaces that expose product internals — the web dashboard and the
  watch/gate TUI surfaces. The **APS plan** viewer is categorically different: it
  parses public APS markdown (`plans/index.aps.md` + `plans/modules/*.aps.md`)
  and renders it. It touches no kernel, policy, checks, graph, intercept, or auth
  code. It is a consumer of the public format, not a window into the product.

## Decision

Carve a **narrow** exception out of ADR-018's closed-product boundary:
**read-only consumers of the public APS format**, built only on the public OSS
surface, may be published under `Apache-2.0` in `anvil-plan-spec` — subject to
the legal gate above. Two asset classes qualify today.

**In scope of the carve-out (publishable):**

*The APS viewer:*

- The APS snapshot builder (`anvil-cli/src/plan_dashboard.rs`): parses
  `plans/` markdown into a plan-status model. Depends only on `std` + `serde` +
  `anyhow`.
- The Ratatui render surface (`anvil-tui/src/surfaces/plan_dashboard/*`):
  depends only on the public `eddacraft-tui` crate + `ratatui`.
- A minimal standalone run loop and CLI entry point (reduced from
  `anvil-cli/src/tui.rs` and `commands/plan.rs`).

*The APS format tooling:*

- `scripts/aps/drift-check.mjs`, `index-counts.mjs`, `active-lint.mjs`, and the
  shared `lib/modules.mjs` parser. Zero third-party dependencies (`node:fs`/
  `node:path` only); they read and validate the canonical APS layout
  (`plans/index.aps.md` + `plans/modules/*.aps.md`).
- The doc-index helpers `scripts/docs/docs-index.mjs` and
  `check-index-freshness.mjs` to the extent they operate on APS/plan files.

**Explicitly NOT in scope (stays closed under ADR-018):**

- The kernel, policy engine, architecture engine, checks, graph, intercept, and
  auth/activation stacks — none of which the viewer touches.
- The **product** dashboards: the web dashboard (`apps/website`) and the
  `anvil-tui` watch dashboard / gate explorer (which render scan and policy
  engine output, i.e. product analysis).
- Any future TUI surface that renders product-analysis output rather than the
  public plan format.

**Carve-out principle (bounds the precedent):** only code that (a) consumes a
*public OSS format or primitive*, (b) builds *solely* on the existing public OSS
surface (the tooling does so trivially — zero dependencies), and (c) exposes *no*
product internals qualifies. This authorises the APS viewer and the APS format
tooling specifically; it does not crack ADR-018 open for product code in general.
Any future candidate must clear the same three tests and get its own decision.

**Source topology — one-way fork, not a mirror.** Unlike ADR-047 (where
`eddacraft-tui`'s canonical source stays in Anvil and the public repo is a
read-only mirror), the published copies' **canonical source becomes
`anvil-plan-spec`**. Anvil keeps its internal `anvil plan dashboard` and its
`scripts/aps/*` unchanged in the closed monorepo — the scripts especially,
because Anvil's CI depends on the Anvil release-lifecycle status dialect
(`Merged` / `Released/Shipped`, `shipped-aps-without-release-record`) that the
public copy must strip. Each public copy is seeded once and then diverges and is
re-developed independently. A one-way fork lets the public copies target
*canonical* APS (and be scrubbed of Anvil internals) without coupling to, or
leaking, the proprietary originals.

**Pre-publication checklist (required before the seed is lifted, gated on
Accepted + legal sign-off):**

1. Apply `Apache-2.0` licence headers to each published source file.
2. Scrub Anvil-internal references from the published copy — internal crate/script
   paths in comments, internal status dialect framing, and test-fixture strings
   (internal branch names, commit SHAs, PR numbers, unreleased version tags,
   non-existent crate names).
3. *Viewer:* replace hardcoded Anvil branding (e.g. the `"Anvil APS Work
   Dashboard"` render title) with neutral defaults.
4. *Tooling:* decouple the Anvil release-lifecycle status dialect — make the
   done-state set the canonical `Done` / `Complete` and the Anvil extensions
   (`Merged` / `Released/Shipped`) and the `shipped-aps-without-release-record`
   check configurable/optional, so the published tooling validates canonical APS
   rather than Anvil's dialect.
5. Confirm each published component builds and tests pass against its public deps
   (the viewer against crates.io `eddacraft-tui`; the tooling has none), with a
   self-test CI gate in its home repo.

## Rationale

A read-only viewer and the drift/lint/index tooling are the same *class* of asset
as the format spec, the parser, and the validator that already live in
`anvil-plan-spec` — the tooling especially, since it *is* validation and
derivation over the format. Both strengthen the format's adoption story (the
ADR-018 network-effect argument) and give away nothing about the product moat,
which lives in the engines and the product dashboards the carve-out explicitly
excludes. The tooling is an even cleaner fit than the viewer: it has zero
third-party dependencies and touches nothing but plan files. The one-way-fork
topology keeps the proprietary and public codebases cleanly separated, so each
public copy can be scrubbed and re-developed without an ongoing leak surface.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| **Chosen: narrow OSS carve-out for read-only APS-format consumers (viewer + tooling), one-way forks into `anvil-plan-spec`, legal-gated** | Both match `anvil-plan-spec`'s format charter; no product moat exposed; clean public/private split via divergent forks; bounded precedent (three-test rule) | Requires a relicensing decision + legal sign-off; each seed copy must be scrubbed before publication; an explicit (small) crack in ADR-018's "no product source leaves" rule |
| Carve out only the viewer, decide the tooling later | Smaller first step | The tooling raises the *same* IP question and is a cleaner fit; deferring just forces a near-identical second ADR |
| Re-implement both from scratch in `anvil-plan-spec` | No relicensing question at all; clean-room; canonical-only by construction | Discards working, verified implementations; duplicate effort; risk of behavioural divergence for no product reason |
| Keep both closed; ship nothing public | Zero IP risk; ADR-018 untouched | Abandons the goal (format tooling for adopters of the open format); weakens the format's adoption story |
| Open-source `anvil-cli`/`anvil-tui` (open-core) | Enables a viewer trivially | Directly contradicts ADR-018; exposes the policy/architecture/checks engines — the actual product moat |

## Consequences

- **Positive:** the open APS format gains a reference viewer *and* reference
  drift/lint/index tooling; aligns with `anvil-plan-spec`'s charter and the
  ADR-018 network-effect thesis; the verified dashboard seed is reused rather than
  rebuilt; one decision settles both asset classes coherently instead of two
  near-identical ADRs; the one-way forks keep the proprietary copies isolated.
- **Negative:** introduces a (bounded) exception to ADR-018's "no product source
  leaves the private repo" rule; requires legal sign-off and a per-copy
  pre-publication scrub; creates diverging public copies (viewer + tooling) to
  reason about alongside the internal originals.
- **Risks:**
  - *Precedent creep* — the carve-out could be cited to justify publishing other
    product surfaces. **Mitigation:** the three-test rule (public-format consumer,
    public-OSS-only deps, no product internals) is narrow and each future
    candidate needs its own decision.
  - *Scrub miss* — an internal reference slips into the public copy.
    **Mitigation:** the pre-publication checklist is a hard gate; Council
    re-reviews the scrubbed copy before the lift.
  - *Legal rejection* — sign-off is declined. **Mitigation:** fall back to the
    re-implement-from-scratch alternative, which needs no relicensing.
- **Mitigations:** keep the starter kit unpublished and clearly marked
  blocked-on-this-ADR until Accepted; require Council sign-off on the scrubbed
  copy.

## References

- Amends/carve-out from: [ADR-018](018-product-ip-architecture.md) (product IP
  architecture — closed product + three-repo OSS surface)
- Related topology: [ADR-047](047-eddacraft-tui-canonical-source-mirror.md)
  (`eddacraft-tui` canonical-source mirror — contrast: this ADR is a one-way
  fork, not a mirror)
- APS modules: APSDASH (`plans/modules/aps-dashboard-starter.aps.md`) — the
  dashboard starter kit this decision gates; original viewer delivered by
  APSCAN-011. No APS-tooling starter is staged yet; when one is filed it inherits
  this decision and its pre-publication checklist.
- Tooling sources: `scripts/aps/{drift-check,index-counts,active-lint}.mjs`,
  `scripts/aps/lib/modules.mjs`, `scripts/docs/{docs-index,check-index-freshness}.mjs`.
  The `scripts/aps/*` tooling derives largely from CIB-021/-022 (append-only CI
  log + derived index counts) and the archived APSCAN module.
- OSS surface: `docs/architecture/oss-surface.md`
- Legal contact: `legal@eddacraft.ai` (per repository `LICENSE`)
