# anvil public documentation rebuild implementation plan

**Goal:** Give a first-time user one trustworthy path from “What is anvil?” to
verified first value, with no internal references or assumed product knowledge.

**Architecture:** Keep `docs/public/anvil/` as the public content authority and
organise it by user intent. Keep one canonical install and activation procedure;
other pages link to it. Generate volatile reference material from product
sources and enforce public-document boundaries through `docs:check`.

**Tech stack:** Docusaurus, Markdown/MDX, Node.js documentation validators,
Rust CLI help, and compiled rule registries.

## Authority and evidence

- APS owner: DOCSYNC-028.
- Design source: user-approved clean-room review, 2026-07-18.
- Truth order: shipped behaviour, CLI help, schemas, compiled registries, tests,
  and release artefacts before existing prose.
- Host coordination: DSITE-001 owns shared Docusaurus wiring.
- Validation coordination: extend `docs:check`; do not create a competing
  validation workflow.

## Documentation standard

Every task page answers, in order:

1. Who the task is for and what it accomplishes.
2. How long the happy path usually takes.
3. What must already be installed, configured, or understood.
4. The exact command or action.
5. The observable result proving success.
6. Common failure states and safe recovery.
7. One clear next task.

Public pages must use lowercase `anvil`, `eddacraft`, and `kindling`. They must
define unavoidable terms on first use, separate macOS/Linux and Windows
PowerShell commands, avoid unsupported claims, avoid internal paths and
identifiers, and link to canonical procedures instead of repeating them.

## Target information architecture

```text
anvil
├── Start here
│   ├── What anvil does
│   ├── Install and get first value
│   ├── Ten-minute protection tutorial
│   └── Glossary
├── How-to guides
├── Concepts
├── Reference
├── Beta
└── Releases
```

## Delivery waves

### Wave 1 — truth and boundaries

1. Inventory public claims against current sources.
2. Add a public-doc validator for internal leakage, product-name casing,
   navigation coverage, duplicated canonical procedures, and generated output.

### Wave 2 — first-time-user journey

3. Rebuild overview → quickstart → first protection as one canonical path.
4. Rebuild sidebars and discovery around user intent; add a glossary.

### Wave 3 — deeper guidance and reference

5. Correct or consolidate deeper guides and unsupported integration claims.
6. Generate CLI, rule, and support reference pages from current product sources.

### Wave 4 — regression and clean-room proof

7. Add positive and negative validator fixtures to the existing docs tests.
8. Build the site and execute the clean-room acceptance script.

## TDD and replacement evidence

Validator behaviour is developed red → green through fixture tests. Prose and
navigation changes are not usefully unit-testable before authoring; their
replacement evidence is the public-boundary validator, deterministic reference
freshness check, Docusaurus production build, command-truth check, and the
clean-room task rubric below.

## Clean-room acceptance

A tester starts at the docs homepage without repository access or terminology
supplied out of band. They must be able to explain what anvil does, identify
support, install the correct binary, verify it, run a first check, activate
ongoing protection, distinguish protection modes, recover from common states,
understand local/network data boundaries, and report a reproducible problem.

Acceptance fails if the tester must open an internal link, infer an undefined
term, choose between duplicated setup procedures, guess expected output, or ask
which page is authoritative.

## Completion evidence

```bash
node scripts/docs/generate-anvil-public-reference.mjs --check
node scripts/docs/check-public-docs.mjs
node scripts/docs/check-anvil-public-commands.mjs
bash scripts/docs/docs-check.test.sh
pnpm docs:check
pnpm --dir apps/docs-site build
pnpm lint:md
pnpm format:check
pnpm aps:active-lint
pnpm aps:index:check
pnpm validate:changed
git diff --check
```

## Verification record — 2026-07-18

The clean-room walkthrough began at the docs homepage and followed the public
links through overview, fit and support, account-free installation and first
value, approved-access authentication, activation, the ten-minute protection
tutorial, troubleshooting, security boundaries, and beta feedback. Each step
defines its prerequisites, observable success, safe recovery, and next action;
the waiting-for-access branch remains on account-free material.

Recorded evidence:

- Public boundary: 52 files checked with no internal references, casing drift,
  hidden navigation pages, duplicated installers, or stale generated output.
- Command truth: 104 of 104 fenced command examples, including inline GitHub
  Actions `run:` steps, parse against `anvil 0.9.0-beta`.
- Generated reference: CLI, compiled-rule, language, and client pages match the
  `v0.9.0-beta` release tag.
- Contract suite: all positive, negative, fail-closed, and stale-output cases
  pass.
- Site: the Docusaurus production build succeeds.
- Governance: `docs:check` passes all nine surfaces; pre-existing baselined
  warnings remain outside this work item.
- Council: newcomer journey, documentation quality, and security/operations
  reviewers report no remaining critical or major findings.
