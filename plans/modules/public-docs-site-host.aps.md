<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->

# Public Docs-Site Host

| ID    | Owner      | Status      | Progress |
| ----- | ---------- | ----------- | -------- |
| DSITE | @eddacraft | In Progress | 2/3      |

**Last reviewed:** 2026-06-21

## Purpose

The public docs-site (`apps/docs-site`, Docusaurus) is a **single shared host**
for several products in the Edda Craft family: Anvil, Kindling, APS, Edda Stack,
and (planned) `eddacraft-tui`. Each product gets its own section — a navbar
entry, a `apps/docs-site/sidebars/<product>.ts` sidebar, and a
`docs/public/<product>/` content tree — but the host itself lives in **this**
repository (`anvil-001`).

That hosting arrangement was not modelled in APS. The per-product content of the
**Anvil** section is tracked by [DOCSYNC](documentation-sync.aps.md), but the
**host wiring** and the **sibling product sections** (whose canonical content is
authored upstream in their own repositories and mirrored here) had no active
`Files` owner. As a result the APS drift check flags any change under
`apps/docs-site/sidebars/**` or a sibling `docs/public/<product>/` tree as
"changed but no active APS Files field references it" — which is exactly what
surfaced when PR #2825 overhauled the Kindling section to match the
Rust-canonical engine.

This module owns the **shared docs-site host**: the multi-product wiring, and the
registration of each sibling product's section. It does **not** own the editorial
content of a product's docs — that stays with the product (upstream repo for
siblings; DOCSYNC for Anvil).

## In scope

- The Docusaurus host wiring that aggregates per-product sections: the plugin
  instances and navbar in `apps/docs-site/docusaurus.config.ts`, the
  `apps/docs-site/sidebars/` nav files, the `apps/docs-site/vercel.json`
  `ignoreCommand` path-list, and the section catalogue in
  `apps/docs-site/AGENTS.md`.
- Registration of each **sibling product** section hosted here — the sidebar
  plus the mirrored `docs/public/<product>/` content tree (Kindling, APS, Edda
  Stack; `eddacraft-tui` via TUIN-013).
- Keeping the sibling sections in sync with what each product actually ships
  (the trigger for the Kindling overhaul in PR #2825).

## Out of scope

- Editorial content of the **Anvil** section (`docs/public/anvil/`) — owned by
  [DOCSYNC](documentation-sync.aps.md).
- Authoring the canonical source of a sibling's docs — that lives in the
  sibling's own repository (e.g. `eddacraft/kindling`); this module tracks the
  **mirror + wiring** in `anvil-001`, not the upstream source.
- Internal-docs governance, the `docs:check` / `docs:index` tooling, and
  metadata conventions — owned by DOCGOV.
- The `eddacraft-tui` content tree itself — owned by
  [TUIN-013](tui-next.aps.md); DSITE only owns its host wiring.

## Interfaces

**Depends on:**

- `apps/docs-site` — the Docusaurus instance and its per-product plugin
  configuration.
- The sibling product repositories — the canonical source for each mirrored
  section (e.g. `eddacraft/kindling` for `docs/public/kindling/`).
- DOCGOV — `pnpm docs:check` validates section output; metadata convention every
  new public doc carries (`docs/public/**` pages omit the internal governance
  table).
- [DOCSYNC](documentation-sync.aps.md) — owns the Anvil section content; DSITE
  owns the host that section plugs into.
- [TUIN-013](tui-next.aps.md) — the `eddacraft-tui` section; DSITE hosts its
  wiring, TUIN owns its content.

**Exposes:**

- An active `Files` owner for the shared docs-site host wiring and sibling
  sections, so changes under `apps/docs-site/sidebars/**` and mirrored
  `docs/public/<product>/` trees are APS-tracked rather than drift-flagged.

## Work Items

> The shared host and the live sibling sections already ship; this module
> back-fills their APS ownership. DSITE-003 is a **standing** item: each new
> sibling section (or refresh to match upstream) registers here.

### DSITE-001: Shared docs-site host wiring

- **Intent:** The multi-product Docusaurus host wiring has an active APS owner.
- **Expected Outcome:** the host that aggregates the Anvil, Kindling, APS, Edda
  Stack, and `eddacraft-tui` sections — navbar + plugin instances in
  `docusaurus.config.ts`, the `sidebars/` nav files, the `vercel.json`
  `ignoreCommand` path-list, and the `AGENTS.md` section catalogue — is tracked
  here. A change to any sidebar nav file is recognised by the APS drift check.
- **Validation:** `pnpm aps:drift` with the changed sidebar file no longer emits
  `changed-file-without-aps-reference` for `apps/docs-site/sidebars/*.ts`;
  `pnpm aps:active-lint` clean for this module.
- **Status:** Done
- **Files:** `apps/docs-site/sidebars/`, `apps/docs-site/docusaurus.config.ts`,
  `apps/docs-site/vercel.json`, `apps/docs-site/AGENTS.md`
- **Confidence:** high

### DSITE-002: Kindling docs section (mirrored, Rust-canonical)

- **Intent:** The hosted Kindling docs section is APS-owned and matches the
  shipped Rust-canonical engine.
- **Expected Outcome:** the `docs/public/kindling/` content tree and its
  `apps/docs-site/sidebars/kindling.ts` sidebar — mirrored from
  `eddacraft/kindling` — are tracked here. The section describes the real
  `kindling` binary surface (`init`, `log`, `capsule open/close`, `status`,
  `search`, `list`, `pin`/`unpin`, `forget`, `export`/`import`, `serve`, `hook`),
  not the retired fictional CLI, per the PR #2825 overhaul.
- **Validation:** `pnpm docs:check` passes for the Kindling section; a change
  under `docs/public/kindling/` or to `sidebars/kindling.ts` is recognised by
  `pnpm aps:drift`.
- **Status:** Merged 2026-06-20 via PR #2825
- **Files:** `docs/public/kindling/`, `apps/docs-site/sidebars/kindling.ts`
- **Confidence:** high

### DSITE-003: Register remaining sibling sections

- **Intent:** Every sibling product section hosted on the docs-site is
  APS-tracked, and stays in sync as siblings add or refresh docs.
- **Expected Outcome:** the live sibling sections beyond Kindling — APS
  (`docs/public/aps/`) and Edda Stack (`docs/public/edda-stack/`) — are
  registered here, and a standing convention exists for adding the next sibling
  section (sidebar + content tree + host wiring via DSITE-001). The
  `eddacraft-tui` content tree is delegated to TUIN-013.
- **Validation:** `pnpm aps:drift` recognises changes under the registered
  sibling content trees; new sibling sections are added under this item rather
  than left drift-flagged.
- **Status:** Ready
- **Files:** `docs/public/aps/`, `docs/public/edda-stack/`
- **Dependencies:** DSITE-001
- **Confidence:** medium

## Notes

- The drift check matches a changed file against an item's `Files` patterns:
  a trailing-slash entry (e.g. `apps/docs-site/sidebars/`) matches everything
  beneath it, so the host-wiring item covers each sibling sidebar without listing
  them one by one.
- Anvil's own section (`docs/public/anvil/`, `apps/docs-site/sidebars/anvil.ts`)
  is deliberately **not** claimed here — DOCSYNC owns Anvil content. The host
  wiring item (`apps/docs-site/sidebars/`) covers the sidebar nav files as
  infrastructure; the content trees stay with their owning module.
