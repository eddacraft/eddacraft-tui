<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if work items exist and status is Ready. -->

# Documentation Definition Layer

| ID     | Owner | Priority | Status | Progress |
| ------ | ----- | -------- | ------ | -------- |
| DOCDEF | —     | high     | Ready  | 0/6      |

**Last reviewed:** 2026-08-19 against the operator-approved
[definition-layer design](../specs/2026-08-19-anvil-docs-definition-layer.md),
live sidebar `apps/anvil-docs-private/sidebars/anvil.ts`, ADR-123, ADR-108,
and DOCRB public-IA ownership.

> **Exclusive module.** DOCDEF owns public Anvil *definition content* and the
> public-reference generator. Live information architecture and the live
> sidebar are **DOCRB** (DOCRB-011). Feature PRs update only their own item
> status and evidence; stored progress counts are reconciled separately under
> ADR-053.

## Purpose

Beta testers cannot find a product manual. After DOCSYNC-028 the live sidebar
is journey-only, existing reference pages are off-nav, and no public page
states the planless `anvil check` subset or the check-versus-scan-versus-gate
model.

This module adds the definition layer: evaluation model, generated check
catalogue, source-cited config catalogue, full CLI reference, and the
user-visible policy / boundary / baseline pages. It does not replace
journeys and it is not a release claim.

## In Scope

- User-facing evaluation-model and short capability index
- Generated check catalogue from `CHECK_DEFINITIONS`
- Hand-curated, source-cited `.anvil` field catalogue
- Generator extension for CLI subcommands and flags
- User-visible policy pack lifecycle, architecture boundaries, and baseline
- Journey-to-definition link-through on existing public pages

## Out of Scope

- Live sidebar / live-nav check (DOCRB-011)
- Public Draw.io diagrams (DOCRB-008)
- Substantive journey rewrites owned by DOCSYNC
- ADR-108 Rego / agent authoring corpus on the public site
- Kindling, APS, or edda-stack documentation
- Dashboard as generally available
- `anvil admin` how-to

## Interfaces

**Depends on:**

- [docs-rebaseline](./docs-rebaseline.aps.md) (DOCRB-011) — live IA/nav
- [documentation-sync](./documentation-sync.aps.md) — existing journey pages
- [cli-command-truth](./cli-command-truth.aps.md) — CLI command truth
- [unified-config-format](./unified-config-format.aps.md) — config writers/readers

**Exposes:**

- Public definition pages under `docs/public/anvil/concepts/` and
  `docs/public/anvil/reference/`
- Extended `scripts/docs/generate-anvil-public-reference.mjs` outputs

**Does not own:** live sidebar (`apps/anvil-docs-private/sidebars/anvil.ts`).
Adding a new page id to that file after DOCRB-011 is a coordinated one-line
edit; it does not move ownership.

## Work Items

### DOCDEF-001: Publish the evaluation model and short capability index

- **Status:** Merged 2026-08-19 via PR #4028
- **Intent:** Give testers a precise public model of check, scan, finding,
  gate, audit, watch, and the planless `anvil check` subset.
- **Expected Outcome:** `concepts/evaluation-model.md` quotes the approved
  check-versus-scan sentences and states that `anvil check` runs only
  `secret-detection` and `antipattern-scan`; a 12-row `reference/what-anvil-can-do`
  index exists; glossary adds scan, rule-versus-check, and audit-versus-check;
  `concepts/gates.md` and first-gate link to the model.
- **Files:** `docs/public/anvil/concepts/evaluation-model.md`,
  `docs/public/anvil/reference/what-anvil-can-do.md`,
  `docs/public/anvil/concepts/glossary.md`,
  `docs/public/anvil/concepts/gates.md`,
  `docs/public/anvil/first-gate.md`,
  `docs/public/anvil/overview.md`,
  `apps/anvil-docs-private/sidebars/anvil.ts`,
  `apps/docs-site/sidebars/anvil.ts`,
  `scripts/docs/check-public-docs.mjs`
- **Scope:** One explanation page, one short index, glossary adds, inbound
  links from existing concept/tutorial pages
- **Non-scope:** Check catalogue generation, config catalogue, CLI flags,
  policy authoring, live-nav check (DOCRB-011)
- **Dependencies:** DOCRB-011
- **Confidence:** high
- **Validation:** `pnpm docs:public:check && pnpm docs:check && pnpm --filter @eddacraft/anvil-docs-private build`

### DOCDEF-002: Generate the public check catalogue

- **Status:** Merged 2026-08-19 via PR #4030
- **Intent:** Publish every shipped check from `CHECK_DEFINITIONS` so testers
  can look up what runs, what `anvil check` ignores, and what is
  flag-selected.
- **Expected Outcome:** `docs/public/anvil/reference/checks.md` is generated
  from `CHECK_DEFINITIONS` fields; surface-check flag ids come from an
  explicit table or flag-registry symbol, not comments; `rule-reference`
  is labelled as the body of `antipattern-scan` only.
- **Files:** `scripts/docs/generate-anvil-public-reference.mjs`,
  `docs/public/anvil/reference/checks.md`,
  `docs/public/anvil/reference/rules.md`,
  `crates/anvil-cli/src/commands/check_catalog.rs`,
  `apps/anvil-docs-private/sidebars/anvil.ts`,
  `apps/docs-site/sidebars/anvil.ts`,
  `scripts/docs/check-public-docs.mjs`
- **Dependencies:** DOCDEF-001
- **Confidence:** high
- **Validation:** `pnpm docs:public:check && pnpm docs:check`

### DOCDEF-003: Publish the source-cited config field catalogue

- **Status:** In Progress
- **Intent:** Give testers one place to look up every `.anvil` key we can
  honestly extract from writers, readers, and the file `anvil init` writes.
- **Expected Outcome:** `docs/public/anvil/reference/config.md` exists with
  front-matter `id: config` and sidebar slug `reference/config`; operations
  page is renamed away from "Configuration reference"; `config show --json`
  is documented as a file-label contract, not a census; `antipattern.exclude`
  and `config set` rule-mode-only behaviour are included.
- **Files:** `docs/public/anvil/reference/config.md`,
  `docs/public/anvil/operations/config.md`,
  `scripts/docs/fixtures/anvil-init.yaml`,
  `scripts/docs/check-public-docs.mjs`,
  `.lintstagedrc.cjs`,
  `.prettierignore`,
  `apps/anvil-docs-private/sidebars/anvil.ts`,
  `apps/docs-site/sidebars/anvil.ts`,
  `crates/anvil-cli/src/commands/init.rs`,
  `crates/anvil-config/src/gate_section.rs`
- **Dependencies:** DOCRB-011
- **Confidence:** high
- **Validation:** `pnpm docs:public:check && pnpm docs:check`

### DOCDEF-004: Generate CLI subcommands and flags

- **Status:** Merged 2026-08-20 via PR #4037
- **Intent:** Replace the top-level command table with source-derived
  subcommands and flags so testers do not have to run `--help` for the
  definition layer.
- **Expected Outcome:** The existing public-reference generator emits flags
  and subcommands for the scoped command list; `GlobalArgs` appear once;
  hidden clap commands stay unpublished; a help-snapshot check runs under
  `pnpm docs:public:check`.
- **Files:** `scripts/docs/generate-anvil-public-reference.mjs`,
  `docs/public/anvil/reference/cli.md`,
  `scripts/docs/fixtures/anvil-cli-help/`,
  `scripts/docs/docs-check.test.sh`,
  `crates/anvil-cli/src/main.rs`,
  `crates/anvil-cli/src/commands/`
- **Dependencies:** DOCRB-011
- **Confidence:** medium
- **Validation:** `pnpm docs:public:check && pnpm docs:check`

### DOCDEF-005: Document policy packs, boundaries, and baseline

- **Status:** Draft
- **Intent:** Explain the user-visible policy pack lifecycle, architecture
  boundaries, and baseline without publishing the ADR-108 authoring corpus.
- **Expected Outcome:** Public pages cover pack install / show / validate /
  test / gate / exception; authoring is the installed
  `authoring-anvil-policy` skill; `policy list` / `explain` are not taught
  as pack authoring; `policy lint` is not documented as shipped.
- **Files:** `docs/public/anvil/concepts/`,
  `docs/public/anvil/tutorials/policies.md`,
  `docs/public/anvil/reference/checks.md`
- **Dependencies:** DOCDEF-002
- **Confidence:** high
- **Validation:** `pnpm docs:public:check && pnpm docs:check`

### DOCDEF-006: Link journeys through to the definition layer

- **Status:** Draft
- **Intent:** Every kept journey page points at the definition it used, and
  remaining safe existing pages are on live nav.
- **Expected Outcome:** Quickstart, first-gate, tutorials, and core how-tos
  have definition footers; remaining safe unhides (hooks, telemetry,
  uninstall, insights, watch-output, review-capsules, beta-testing-guide)
  are on live nav with local-only wording where required; dashboard stays
  off live nav.
- **Files:** `docs/public/anvil/**`
- **Dependencies:** DOCDEF-001
- **Confidence:** high
- **Validation:** `pnpm docs:public:check && pnpm docs:check && pnpm --filter @eddacraft/anvil-docs-private build`

## Release Posture

DOCDEF is an engineering-effectiveness programme. It is not part of a
release claim set and does not gate a release cut.
