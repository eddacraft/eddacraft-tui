# Adapter Packages — As-Built

| Type     | Authority | Owner | Status | Freshness                                                                                                                  |
| -------- | --------- | ----- | ------ | -------------------------------------------------------------------------------------------------------------------------- |
| As-built | Derived   | APSMD | Live   | Last reviewed 2026-05-07 against `v0.6.0-beta` and `packages/adapters/`, `packages/aps/`, `packages/kindling-integration/` |

| Upstream                                                                                    | Downstream                                                                                                 |
| ------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| `packages/adapters/`, `packages/aps/`, `packages/kindling-integration/`, `crates/anvil-cli` | anvil validate / gate / export CLI, APS document tooling, Kindling capture bridge, edda-stack memory layer |

> **Status:** Live (beta) for all three packages. Specific adapters within
> `packages/adapters/` have varying readiness (SpecKit, BMAD, APS Markdown, and
> Generic are complete; Open-Spec and BMAD v4 backward-compat are still in
> progress per public docs and `plans/modules/`). **Last reviewed:** 2026-05-07
> against `v0.6.0-beta` slate (HEAD `cf7ca040`). **Locations:**
> `packages/adapters/`, `packages/aps/`, `packages/kindling-integration/` — all
> TypeScript packages built via `pnpm` / `nx`
> (`packages/adapters/package.json:11`, `packages/aps/package.json:18`,
> `packages/kindling-integration/package.json:91`). **Module owners (APS):**
>
> - `packages/adapters/` — APSMD (Complete,
>   `plans/archive/modules/aps-markdown-adapter.aps.md`); BMAD4 (Proposed,
>   `plans/modules/bmad-v4-backward-compat.aps.md`); OPENSPEC (Draft,
>   `plans/modules/open-spec-adapter.aps.md`).
> - `packages/aps/` — no dedicated module today (the `@eddacraft/anvil-aps`
>   library is foundational and shipped under the original
>   `anvil-file-format.aps.md` / `aps-markdown-adapter.aps.md` work; both
>   archived).
> - `packages/kindling-integration/` — KINDLING (Complete,
>   `plans/archive/modules/kindling-integration.aps.md`). **Used by:** the anvil
>   ecosystem's external-system bridges — agent harnesses (BMAD, SpecKit,
>   Generic markdown) feeding `anvil validate` / `anvil gate` / `anvil export`;
>   the APS document tooling (parse / load / validate / state / templates)
>   consumed by the Rust CLI through FFI and by
>   `packages/adapters/aps-markdown/`; the Kindling capture / observation bridge
>   into the Edda-stack memory layer
>   ([`docs/architecture/edda-stack.md`](edda-stack.md)).

## 1. Overview

Anvil ships three first-party TypeScript packages that sit on the boundary
between its Rust core (`crates/anvil-checks`, `crates/anvil-cli`,
`crates/anvil-kernel`) and the external systems those crates have to negotiate
with. They are independent, but documented together because they share idiom (TS
package, vitest, ESM, `workspace:*` deps onto `@eddacraft/anvil-core`) and
they're all secondary surfaces compared to the Rust core:

- **`packages/adapters/`** (`@eddacraft/anvil-adapters`) — format adapters that
  bridge external planning formats (SpecKit, BMAD, generic markdown, APS
  markdown) into the canonical APS plan shape. Auto-detects format by content +
  path hints.
- **`packages/aps/`** (`@eddacraft/anvil-aps`) — the APS library. Parser,
  loader, validator, filter, state manager, and template generator for `.aps.md`
  planning documents. Eight-rule validator, Zod-backed schemas.
- **`packages/kindling-integration/`** (`@eddacraft/anvil-kindling-integration`)
  — the read-only-queryable, write-only-emit contract layer between Anvil and
  Kindling. Eleven observation kinds, four query scopes, malicious-AI test suite
  proving read-only enforcement.

All three are `0.5.1-beta` package versions ahead of the `v0.6.0-beta` slate
(`packages/adapters/package.json:3`, `packages/aps/package.json:3`,
`packages/kindling-integration/package.json:3`). Versioning is package-local;
none is gated to the Rust binary version.

## 2. Architecture diagram

```text
   ┌────────────────────┐  ┌────────────────────┐  ┌───────────────────────┐
   │ External system    │  │ External system    │  │ External system       │
   │ (BMAD agent /      │  │ (.aps.md docs in   │  │ (Anvil CLI run +      │
   │  SpecKit project / │  │  plans/, agent     │  │  Kindling SQLite db)  │
   │  generic markdown) │  │  output)           │  │                       │
   └─────────┬──────────┘  └────────┬───────────┘  └─────────┬─────────────┘
             │                      │                        │
             ▼                      ▼                        ▼
   ┌────────────────────┐  ┌────────────────────┐  ┌───────────────────────┐
   │ packages/adapters/ │  │ packages/aps/      │  │ packages/kindling-    │
   │ FormatAdapter      │  │ parse/load/valid./ │  │  integration/         │
   │ registry           │  │ state/templates    │  │ Observation +         │
   │  - speckit         │  │  - parser/         │  │  Query contracts      │
   │  - bmad            │  │  - loader/         │  │  - 11 emitters        │
   │  - generic         │◀─┤  - validator/      │  │  - 4 query scopes     │
   │  - aps-markdown   ─┼─▶│  - state/         │  │  - malicious-ai tests │
   │  - common (legacy) │  │  - filter/         │  │  - sensitive-data     │
   └─────────┬──────────┘  │  - templates/      │  │    redaction          │
             │             │  - types/ (zod)    │  │  - retention pruner   │
             │             └────────┬───────────┘  └─────────┬─────────────┘
             │                      │                        │
             │  APSPlan             │  ParsedDocument /      │  Observation /
             │  (from anvil-core)   │  LoadedPlan / Task     │  QueryRequest
             ▼                      ▼                        ▼
   ┌─────────────────────────────────────────────────────────────────────┐
   │ Anvil ecosystem (Rust + TS consumers)                                │
   │  - crates/anvil-cli (validate, gate, export, run)                    │
   │  - crates/anvil-checks (rule registry consuming APSPlan)             │
   │  - apps/anvil-api (no direct adapter dep; admin surface)             │
   │  - kindling-core SDK (writes into local SQLite db)                   │
   └─────────────────────────────────────────────────────────────────────┘
```

The three packages are siblings on the boundary; they don't depend on each other
in a single chain. `packages/adapters/aps-markdown/` imports
`@eddacraft/anvil-aps` for `parseDocument`
(`packages/adapters/src/aps-markdown/adapter.ts:17`); that's the only
intra-package edge between the three. `packages/kindling-integration/` is
independent of both other packages and depends on `@eddacraft/kindling-core`
(`packages/kindling-integration/package.json:98`).

---

## Part 1: `packages/adapters/`

## 3. `packages/adapters/` — overview

Format-conversion framework. Bridges external planning-document formats into the
canonical `APSPlan` shape from `@eddacraft/anvil-core` and serialises in the
other direction. Pluggable: every adapter implements the same `FormatAdapter`
interface and registers itself with a singleton `AdapterRegistry`.

**Layout** (`packages/adapters/src/`, `packages/adapters/AGENTS.md:9-23`):

| Directory                | Lines | Role                                                                                                    |
| ------------------------ | ----- | ------------------------------------------------------------------------------------------------------- |
| `base/`                  | 1 250 | `FormatAdapter` interface, `AdapterRegistry`, helpers, file-discovery, testing harness                  |
| `speckit/`               | 2 451 | GitHub spec-kit adapter (spec.md / plan.md / tasks.md), v1 + v2 import paths, export, three sub-parsers |
| `bmad/`                  | 2 459 | BMAD adapter — PRD, Architecture, Epic, Story, Agent (md + yaml), v6 YAML + v5 legacy                   |
| `generic/`               | 731   | Generic-markdown fallback adapter (PRD/TODO/RFC/ADR — capped 30–45 % confidence)                        |
| `aps-markdown/`          | 462   | Native APS Markdown adapter (`.aps.md`)                                                                 |
| `common/`                | 158   | Legacy `SpecToolAdapter` interface, kept for backward compatibility                                     |
| `__tests__/` (top-level) | 5 022 | Cross-adapter integration tests + fixtures (BMAD v4/v5/v6, SpecKit official, generic, aps)              |

`packages/adapters/src/index.ts:32-48` auto-registers four adapters (APSMarkdown
→ BMAD → SpecKit → Generic) on module import — registration order is registry
insertion order; precedence is by **detection confidence**, not registration.
`common/` exports legacy types (`SpecContext`, `ExternalSpec`,
`ConversionResult`, `ConversionError`, `ConversionWarning`,
`packages/adapters/src/index.ts:24-30`) but no live adapter today.

**Package shape** (`packages/adapters/package.json`):

- ESM only (`"type": "module"`).
- Single export entry: `./dist/index.js`.
- Dependencies: `@eddacraft/anvil-core`, `@eddacraft/anvil-aps` (both
  `workspace:*`).
- Dev-dep: `vitest ^4.1.5`.

The package runs no scripts at build time beyond `tsc -p tsconfig.lib.json`;
auto-registration happens on the **import side** (`src/index.ts:42-45`).

## 4. SpecKit adapter

GitHub's official spec-kit format. **Status: Complete**
(`packages/adapters/README.md:96`, `plans/modules/launch-flow-readiness.aps.md`
ships SpecKit as the flagship format in `v0.6.0-beta`). Largest adapter in the
package by file count and test surface.

**Surface** (`packages/adapters/src/speckit/`):

- `format-adapter.ts` (595 lines) — `SpecKitFormatAdapter` class. Confidence
  scoring across eight indicators, including agent-first heuristics
  (`hasSpeckitNamespace`, `hasAgentsMdSibling`, `format-adapter.ts:35-50`). 50 %
  detection threshold (`format-adapter.ts:97`). Implements `detect()`,
  `detectWithPath()`, `parse()`, `serialize()`, `validate()`.
- `import.ts` (305 lines) — V1 single-file import path.
- `import-v2.ts` (445 lines) — V2 official spec-kit import — takes a
  `{ spec, plan, tasks }` triple and stitches all three documents into one APS
  plan with rich metadata (`packages/adapters/README.md:136-168`).
- `export.ts` (489 lines) — APS → spec-kit export.
- `parser.ts` (389 lines) — core markdown / section parser.
- `parsers/spec-parser.ts` (379 lines) — spec.md (requirements + scenarios).
- `parsers/plan-parser.ts` (342 lines) — plan.md (technical context +
  constitution check + complexity tracking).
- `parsers/tasks-parser.ts` (246 lines) — tasks.md (phases + parallel markers +
  checkpoints).

**Metadata declaration**
(`packages/adapters/src/speckit/format-adapter.ts:58-65`):

```ts
{ name: 'speckit', version: '2.0.0',
  formats: ['speckit', 'spec-kit', 'spec.md', 'plan.md', 'tasks.md'],
  extensions: ['.md'] }
```

**APS mapping** (`packages/adapters/README.md:170-182`):

- User Scenarios → `proposed_changes` with scenario metadata (acceptance
  criteria, edge cases).
- Functional Requirements → `metadata.requirements.functional[]`.
- Key Entities → `metadata.requirements.entities[]`.
- Clarifications (`[NEEDS CLARIFICATION]` markers) →
  `metadata.clarifications[]`.
- Phases & Tasks → `metadata.phases[]`, `metadata.taskDependencies[]`.

**Test coverage**: 114 SpecKit tests claimed in
`packages/adapters/README.md:104` and `AGENTS.md:177` (45 format-adapter tests +
69 parser tests). Live counts: `__tests__/speckit-format-adapter.test.ts` (832
lines), `speckit-import.test.ts` (209), `speckit-import-v2.test.ts` (253),
`speckit-export.test.ts` (233), `speckit-parser.test.ts` (243),
`speckit-spec-parser.test.ts` (120). Real-world fixtures live in
`__tests__/fixtures/speckit/` and `__tests__/fixtures/speckit-official/`.

## 5. BMAD adapter

BMAD (Breakthrough Method for Agile AI-Driven Development) — PRD +
Architecture + Epic + Story + Agent + Workflow + Team + Module. **Status:
Complete for v6.0.3 + v5 legacy paths; v4 backward-compat is in progress**
(BMAD4 Proposed, 0/8 tasks, `plans/modules/bmad-v4-backward-compat.aps.md`).

**Surface** (`packages/adapters/src/bmad/`):

- `format-adapter.ts` (278 lines) — `BMADFormatAdapter` class. Confidence
  scoring across YAML front-matter, FR/NFR/US identifiers, user-story format,
  change log table, document title (weights documented in
  `format-adapter.ts:53-59`). 50 % threshold.
- `parser.ts` (430 lines) — BMAD → APS conversion.
- `serializer.ts` (215 lines) — APS → BMAD generation.
- `types.ts` (311 lines) — `BMADDocumentType` enum + `BMAD_UPSTREAM_VERSION`
  constant for declared upstream tracking.
- `utils.ts` (1 275 lines) — heaviest single file in the package; metadata
  extraction, requirement parsing, agent / workflow / team / module YAML
  parsers, `analyzePath()` folder detection, content analysis. The bulk of the
  BMAD-specific knowledge.

**Metadata declaration** (`packages/adapters/src/bmad/format-adapter.ts:41-48`):

```ts
{ name: 'bmad', version: '0.1.2',
  formats: ['bmad', 'prd', 'architecture', 'agent', 'workflow', 'team', 'module'],
  extensions: ['.md', '.yaml', '.yml'] }
```

**Test coverage**: `__tests__/bmad-format-adapter.test.ts` is 1 797 lines and is
the largest single test file in the package. Fixtures cover valid + invalid
PRDs, architecture docs, epics, stories, agents, v6 YAMLs (`valid-v6-prd.md`,
`valid-v6-team.yaml`, `valid-v6-agent-with-actions.yaml`,
`__tests__/fixtures/bmad/`). README claims 86 tests with >95 % coverage
(`packages/adapters/README.md:193`).

`format-adapter.ts:43` declares version `0.1.2` while
`packages/adapters/README.md:191` documents version `1.0.0` — minor metadata
drift, see G-03 below.

## 6. Other adapters

### 6.1 Generic Markdown (Complete, fallback)

`packages/adapters/src/generic/`. Fallback adapter for documents that don't
match BMAD or SpecKit. Deliberately capped to **45 %** confidence
(`packages/adapters/src/generic/format-adapter.ts:25, 59`) and threshold lowered
to **30 %** so it triggers only when no specific adapter wins
(`format-adapter.ts:24, 62`).

- `format-adapter.ts` (200 lines), `parser.ts` (134), `serializer.ts` (134),
  `types.ts` (53), `utils.ts` (250).
- Supported document types: PRD, TODO, plan, spec, RFC, ADR.
- Tests: `__tests__/generic-format-adapter.test.ts` (398 lines),
  `generic/__tests__/parser.test.ts`, `generic/__tests__/serializer.test.ts`.
  README claims 32 tests with
  > 95 % coverage (`packages/adapters/README.md:325`).

### 6.2 APS Markdown (Complete, native)

`packages/adapters/src/aps-markdown/`. Native APS-format adapter — the adapter
system's identity element. Imports `parseDocument` from `@eddacraft/anvil-aps`
(`adapter.ts:17`) and `generatePlanId`, `generateHash`, `APS_SCHEMA_VERSION`
from `@eddacraft/anvil-core` (`adapter.ts:9-16`). Detection scores against H1 +
Tasks / Modules sections + SCOPE-NNN task IDs + `**Intent:**` field + `.aps.md`
links + Scope / Confidence / Owner / Priority / Packages fields. Module status
APSMD is Complete (`plans/archive/modules/aps-markdown-adapter.aps.md:1-7`).

- `adapter.ts` (462 lines); single class.
- Test fixture: `__tests__/__fixtures__/simple-leaf.aps.md`.
- Cross-cutting tests via the package-level
  `__tests__/adapter-edge-cases.test.ts` (937 lines).

### 6.3 Open-Spec adapter (in progress, Draft)

`plans/modules/open-spec-adapter.aps.md` — OPENSPEC, Draft. Source code **not
yet present in `packages/adapters/src/`** as of HEAD `cf7ca040`; the module's
last review (2026-04-26) reaffirms it lives in the TS adapters layer rather than
Rust. Will add `OpenSpecFormatAdapter` once implementation lands. Tracked in
`plans/modules/open-spec-adapter.aps.md`.

### 6.4 BMAD v4 backward compatibility (in progress, Proposed)

`plans/modules/bmad-v4-backward-compat.aps.md` — BMAD4, Proposed, 0/8. Adds
v4.0.0–v4.44.1 folder + agent + workflow detection on top of the existing BMAD
adapter. Tracked gaps:

- Folder detection (`bmad-core/`, `.bmad-core/`).
- Agent format (v4 agents are `.md` with flat YAML — `agent:` + `persona:` +
  `commands:` at the top).
- Workflow schema (v4 uses `id/type/sequence[]`; v6 uses
  `instructions/config_source`).
- Team nesting (v4 nests `agents:` inside `bundle:`).
- `{root}` variable expansion.
- v4 test fixtures.

The adapter ships **without v4 detection** in `v0.6.0-beta`; v4 documents
currently fall through to the Generic fallback.

## 7. Adapter contract

All adapters implement `FormatAdapter`
(`packages/adapters/src/base/types.ts:161-232`):

```ts
interface FormatAdapter {
  readonly metadata: AdapterMetadata;
  detect(content: string): DetectionResult;
  parse(
    content: string,
    context?: ParseContext,
    options?: AdapterOptions
  ): Promise<ParseResult>;
  serialize(plan: APSPlan, options?: AdapterOptions): Promise<SerializeResult>;
  validate(
    content: string,
    options?: AdapterOptions
  ): Promise<ValidationResult>;
  detectWithPath?(content: string, hint: PathDetectionHint): DetectionResult; // optional
  canImport(format: string): boolean;
  canExport(format: string): boolean;
}
```

`BaseFormatAdapter` (`base/types.ts:239-342`) is the abstract class every
concrete adapter extends; it implements `canImport` / `canExport` against
`metadata.formats` ∪ `metadata.extensions` and provides `createParseSuccess` /
`createParseError` / `createSerializeSuccess` / `createSerializeError` /
`addError` / `addWarning` helpers so concrete adapters never throw — they return
structured `AdapterError[]` / `AdapterWarning[]` shapes
(`packages/adapters/AGENTS.md:170-174`).

`AdapterRegistry` (`base/registry.ts:17-258`) is a typed singleton:

- `register()` (line 46) rejects duplicates by `metadata.name`.
- `detectAdapter(content, minConfidence = 50)` (line 97) iterates all registered
  adapters' `detect()` methods, returns the highest-confidence match above the
  threshold.
- `detectAdapterWithPath()` (line 126) is the path-aware variant — falls back to
  content-only `detect()` for adapters that don't implement `detectWithPath`.
- `detectAll()` (line 156) returns every detection result sorted by confidence —
  the debug surface for "why didn't my doc match?".
- `listSupportedFormats()` / `listSupportedExtensions()` — registry
  introspection used by the CLI's `FormatDetectionService`.

The registry is exported as a module-level singleton (`registry`) that is
re-exported from `packages/adapters/src/index.ts:48`.

---

## Part 2: `packages/aps/`

## 8. `packages/aps/` — overview

Anvil Planning Spec library. Pure TypeScript; no CLI of its own — it's consumed
as a library by the Rust CLI (via FFI through anvil-core shapes), by
`packages/adapters/aps-markdown/`, and by tooling that generates / validates
`.aps.md` documents.

**Layout** (`packages/aps/src/`, `packages/aps/AGENTS.md:9-26`, combined ~7 600
LOC of source + tests):

| Module       | Source LOC | Test LOC | Role                                                                                        |
| ------------ | ---------- | -------- | ------------------------------------------------------------------------------------------- |
| `parser/`    | 929        | 1 420    | remark / unified AST parser; `parseDocument`, `parseIndex`, `parseTask`                     |
| `loader/`    | 448        | 294      | Load + recursively resolve plan graphs (index → leaf modules)                               |
| `validator/` | 807        | 287      | Eight-rule validator (errors + warnings)                                                    |
| `filter/`    | 561        | 317      | Task / module filtering + `ContextBundleJSON` for LLM consumption                           |
| `state/`     | 1 007      | 821      | `.anvil/state.json` with first-lock-wins concurrency, execution-plan generation, provenance |
| `templates/` | 781        | 378      | Template generator with three variants (minimal / standard / full)                          |
| `types/`     | 168        | —        | Zod schemas + TypeScript types for `Task`, `ModuleMetadata`, `ParsedDocument`               |

`packages/aps/src/index.ts:10-16` re-exports everything; subpath exports
(`./parser`, `./loader`, `./filter`, `./validator`, `./state`, `./templates`,
`./types`) are declared in `packages/aps/package.json:9-15` so consumers can
deep-import.

**Package shape**:

- ESM (`"type": "module"`).
- Eight subpath exports.
- Runtime deps: `@types/mdast`, `remark-parse`, `unified`, `unist-util-visit`,
  `zod` (`packages/aps/package.json:24-29`).
- Dev-dep: `vitest`.

## 9. APS validation tooling

The validator is the load-bearing surface — `anvil validate` (Rust CLI) and
`packages/adapters/aps-markdown/` both depend on it.

**Entry points** (`packages/aps/src/validator/index.ts`):

- `validatePlanningDoc(filePath, options)` (line 102) — main entry. Reads the
  file, detects index vs leaf, runs structural + cross-reference rules.
- `formatValidationIssues(result)` — pretty-prints to a string for CLI
  consumption.

**Eight built-in rules** (`packages/aps/AGENTS.md:38-51`, implementations in
`validator/index.ts`):

| Rule                    | Severity | What it checks                                                         | Implementation                                              |
| ----------------------- | -------- | ---------------------------------------------------------------------- | ----------------------------------------------------------- |
| `required-sections`     | error    | `## Modules` for index, `## Tasks` for leaf                            | `validateIndexStructure`, `validateLeafStructure`           |
| `task-format`           | error    | `SCOPE-NNN` task ID pattern (1-10 upper alphanumeric, hyphen, 3-digit) | `validateTaskFormat`; regex `types/index.ts:30`             |
| `task-intent`           | error    | Task must declare `**Intent:**`                                        | `validateTaskContent`                                       |
| `broken-links`          | error    | Referenced module file must exist                                      | `validateModuleLinks`, `validateFileExists`                 |
| `duplicate-ids`         | error    | No duplicate task IDs across loaded plan graph                         | `validateDuplicateTaskIds`                                  |
| `circular-dependencies` | error    | Module dependency graph must be a DAG                                  | `validateCircularDependencies` (uses `loader/detectCycles`) |
| `scope-mismatch`        | warning  | Task ID prefix matches module's declared scope                         | `validateScopeMismatches`                                   |
| `orphan-modules`        | warning  | Modules must be referenced from an index                               | `validateOrphanModules`                                     |

`ValidationResult` (`validator/index.ts:57-69`) splits issues into `issues` /
`errors` / `warnings`; `valid` is true when there are zero errors (warnings are
allowed).

Validation is **recursive by default** — the validator walks the plan graph
through `loadPlan()` (`loader/index.ts:80`) and surfaces errors across the whole
graph. `recursive: false` and `skipRules: string[]` options
(`validator/index.ts:74-83`) opt out for fast-feedback paths.

## 10. Templates (`packages/aps/templates/`)

Static template files committed to the repo (regenerated via
`scripts/generate-templates.js`). Three variants × three document types = nine
files (`packages/aps/templates/`, `packages/aps/AGENTS.md:140`):

| File                 | Lines | Variant  | Document type                                                                                                   |
| -------------------- | ----- | -------- | --------------------------------------------------------------------------------------------------------------- |
| `index-minimal.md`   | 16    | minimal  | Index file (multi-module navigation only)                                                                       |
| `index-template.md`  | 63    | standard | Index file (recommended)                                                                                        |
| `index-full.md`      | 94    | full     | Index file (Problem/Success Criteria, System Map, Milestones, Modules, Epics, Decisions, Risks, Open Questions) |
| `leaf-minimal.md`    | 14    | minimal  | Leaf spec (single module, bare task list)                                                                       |
| `leaf-template.md`   | 55    | standard | Leaf spec (recommended)                                                                                         |
| `leaf-full.md`       | 76    | full     | Leaf spec (full sections)                                                                                       |
| `simple-minimal.md`  | 14    | minimal  | Single-file plan                                                                                                |
| `simple-template.md` | 30    | standard | Single-file plan                                                                                                |
| `simple-full.md`     | 56    | full     | Single-file plan (full sections)                                                                                |

Generated by `generateAllTemplates()`
(`packages/aps/src/templates/generator.ts:776-line export region`, called from
`scripts/generate-templates.js:19`). Templates are consumed by the (future)
`anvil aps init` family — the Rust CLI's APS init command reads from
`dist/templates/` after `pnpm generate-templates`.

## 11. Examples (`packages/aps/examples/`)

Real APS documents that exercise the parser, validator, and loader as
integration fixtures and as user-facing learning material
(`packages/aps/examples/README.md:1-30`):

| Example                                                                               | Type                            | Demonstrates                                                                                              |
| ------------------------------------------------------------------------------------- | ------------------------------- | --------------------------------------------------------------------------------------------------------- |
| `feature-auth.aps.md`                                                                 | Single-file plan                | Sequential dependencies, multi-scope tasks, deferred work in Notes                                        |
| `system-ecommerce/APS.md` + `modules/{auth,products,cart,payments}.aps.md`            | Multi-file plan, 4 leaf modules | Index file with module metadata, dependency graph, Decisions, Open Questions, mixed confidence            |
| `refactor-error-handling.aps.md`                                                      | Single-file plan                | Refactor with uncertainty (low/medium/high confidence mix), audit/research tasks, parallel refactor tasks |
| `false-positive-modules/APS.md` + `leaf-with-heading.aps.md`                          | Validator regression fixture    | Leaves that look like modules but aren't                                                                  |
| `nested-index/APS.md` + `subsystem/APS.md` + `subsystem/modules/{api,workers}.aps.md` | Nested index                    | Recursive index loading (`maxDepth` option in loader)                                                     |

## 12. Scripts (`packages/aps/scripts/`)

Single script: `scripts/generate-templates.js` (33 lines). Imports
`generateAllTemplates` from `dist/templates/generator.js` (built output) and
writes the nine template files into the configured output directory (defaults to
`./templates`, `scripts/generate-templates.js:12`). Wired through
`packages/aps/package.json:22` as `pnpm generate-templates` (depends on
`pnpm build` first because it imports from `dist/`).

## 13. Schema location

The APS package is the **canonical home** for schemas. Zod schemas live in
`packages/aps/src/types/index.ts`:

- `TaskSchema` (line 32) — `id`, `title`, `intent`, `expectedOutcome`,
  `validation`, `confidence`, `scopes`, `nonScope`, `files`, `tags`,
  `dependencies`, `inputs`, `risks`, `packages`, `link`, `status`, `sourcePath`,
  `sourceLineNumber`.
- `ModuleMetadataSchema` (line 101) — `id`, `title`, `path`, `scope`, `owner`,
  `status`, `priority`, `tags`, `dependencies`, `packages`.
- `ConfidenceSchema` (line 10) = `'low' | 'medium' | 'high'`.
- `TaskStatusSchema` (line 17) =
  `'open' | 'locked' | 'completed' | 'cancelled'`.
- `ModuleStatusSchema` (line 95) =
  `'Proposed' | 'Ready' | 'In Progress' | 'Done' | 'Blocked'`.
- `TASK_ID_REGEX` (line 30) = `/^[A-Z0-9]{1,10}-\d{3}$/`.

`packages/aps/src/state/index.ts` adds runtime state schemas
(`TaskSourceSchema`, `TaskStateSchema`, `StateFileSchema`, `ProvenanceSchema`,
`ExecutionPlanSchema`, `packages/aps/src/state/index.ts:27-100`).

The public-docs JSON Schema reference at
`docs/public/aps/schemas/json-schema.md` is **derived from these schemas** and
at time of review carries a documented status drift — see Known Gaps G-05.

---

## Part 3: `packages/kindling-integration/`

## 14. `packages/kindling-integration/` — overview

The mechanical contract layer between Anvil and Kindling. Defines **what** Anvil
records and **how** the records are queried; enforces that user-supplied AI can
read but never mutate, infer, or annotate. **Status: Complete** (KINDLING
module, `plans/archive/modules/kindling-integration.aps.md:1-9`).

**Layout** (`packages/kindling-integration/src/`,
`packages/kindling-integration/README.md:50-79`, ~5 200 LOC):

| File / dir                                                                    | Lines          | Role                                                                                                                                    |
| ----------------------------------------------------------------------------- | -------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| `observation-contract.ts`                                                     | 500            | Eleven Zod schemas (write-only contract) — see §15                                                                                      |
| `query-contract.ts`                                                           | 389            | Four query scopes (read-only contract) — see §15                                                                                        |
| `kindling-service.ts`                                                         | 272            | `KindlingService` orchestrator (validate → redact → store delegate)                                                                     |
| `query-service.ts`                                                            | 217            | `KindlingQueryService` convenience wrapper                                                                                              |
| `query-limits.ts`                                                             | 106            | Per-config query limits (anti-vacuum-cleaner)                                                                                           |
| `sensitive-data-validator.ts`                                                 | 167            | Pattern-based secret detection + redaction                                                                                              |
| `retention.ts`                                                                | 153            | `IRetentionCapableStore` interface + `pruneOldObservations` / `getStorageStats`                                                         |
| `status.ts`                                                                   | 221            | `getKindlingStatus` / `formatKindlingStatus` (decoupled from CLI)                                                                       |
| `config.ts`                                                                   | 202            | `KindlingConfig` Zod schema, `loadKindlingConfig`, `shouldCapture`                                                                      |
| `adapter.ts`                                                                  | 117            | `AnvilKindlingAdapter` — bridges 11 Anvil kinds → kindling-core's 3 generic kinds                                                       |
| `index.ts`                                                                    | 254            | Public barrel exports                                                                                                                   |
| `malicious-ai.test.ts`                                                        | 949            | 60 tests proving read-only enforcement (no `write` / `update` / `delete` / `annotate` / `tag` / `learn` / `embed` / `infer` operations) |
| `emitters/{session,gate,action,plan,human-input,constraint,error}-emitter.ts` | 671 (combined) | Fire-and-forget emitter helpers per observation kind                                                                                    |
| `utils/debug.ts`                                                              | 65             | `createDebugger` namespace gate                                                                                                         |

**Package shape** (`packages/kindling-integration/package.json`):

- ESM, sixteen subpath exports (`./query`, `./observation`, `./config`,
  `./service`, `./emitters`,
  `./emitters/{session,gate,action,plan,human-input,constraint,error}`,
  `./query-service`, `./query-limits`, `./sensitive-data`, `./retention`,
  `./status`, `./adapter` — `packages/kindling-integration/package.json:8-85`).
- Runtime dep: `@eddacraft/kindling-core@0.1.1` (pinned, not workspace —
  Kindling is a separate published package).
- Dev-deps: `vitest`, `typescript ~6.0.3`.
- Licence: `PROPRIETARY` — different from the workspace default.

## 15. Contract surface (`CONTRACTS.md`)

Two contracts, governed by one rule
(`packages/kindling-integration/CONTRACTS.md:182-195`):

> Kindling is a system of record, not a reasoning engine. Queries may retrieve
> facts; interpretation is the caller's responsibility.

### 15.1 Observation contract (write-only)

`packages/kindling-integration/src/observation-contract.ts`. Eleven observation
kinds, each with a Zod schema:

| #   | Kind                 | Source line | When emitted              |
| --- | -------------------- | ----------- | ------------------------- |
| 1   | `session_start`      | line 27     | Every Anvil run starts    |
| 2   | `session_end`        | line 54     | Every Anvil run completes |
| 3   | `plan_created`       | line 83     | New plan authored         |
| 4   | `plan_edited`        | line 103    | Plan modified             |
| 5   | `plan_approved`      | (line ~140) | Human approves plan       |
| 6   | `plan_rejected`      | (line ~160) | Human rejects plan        |
| 7   | `action_executed`    | (line ~200) | Command / tool / file op  |
| 8   | `gate_evaluated`     | (line ~250) | Every gate check          |
| 9   | `constraint_applied` | (line ~310) | Action prevented by rule  |
| 10  | `human_input`        | (line ~360) | Approval / override       |
| 11  | `error`              | (line ~410) | Failure recorded          |

Properties (enforced by Zod): immutable (consumer-side), ISO-8601 timestamp,
linked (`session_id`, `plan_id`, `gate_id`, `action_id`), sanitised (passes
through `validateNoSensitiveData`), facts only (no inference fields). Validated
via `validateObservation()` (re-exported from
`packages/kindling-integration/src/index.ts:67`).

### 15.2 Query contract (read-only)

`packages/kindling-integration/src/query-contract.ts`. Four bounded scopes
(`QueryScopeSchema`, line 30):

| Scope     | Required ID    | Question                            | Returns                                            |
| --------- | -------------- | ----------------------------------- | -------------------------------------------------- |
| `session` | `session_id`   | What happened in this run?          | Timeline of observations                           |
| `plan`    | `plan_id`      | What happened because of this plan? | Plan + linked executions (only cross-session read) |
| `gate`    | `gate_eval_id` | Why did this gate pass / fail?      | Gate evaluation details                            |
| `action`  | `action_id`    | What exactly did this action do?    | Action execution details                           |

Mandatory request constraints (`query-contract.ts:75-99`):

- `scope` + scope-specific ID (no free-text search).
- `shape ∈ { timeline, list, entity }`.
- `format ∈ { json, text }`, default `json`.
- `max_results`: positive integer ≤ 1 000, default 100.
- `max_payload_bytes`: positive integer ≤ 10 MB, default 1 MB.

Output guarantees (the read side of the contract,
`packages/kindling-integration/CONTRACTS.md:84-92`): stable field names;
explicit ISO-8601 timestamps; explicit links (`caused_by`, `governed_by`,
`approved_by` via `ProvenanceLinkSchema`); no hidden inference; no reordered
history.

### 15.3 Read-only enforcement

`malicious-ai.test.ts` (949 lines, 60 tests) is the proof-by-contradiction test
suite. Sample assertions
(`packages/kindling-integration/src/malicious-ai.test.ts:1-40`):

- `QueryRequest` schema rejects any `write` / `update` / `delete` field.
- `QueryRequest` rejects `annotate` / `tag` / `learn` / `embed` / `infer`.
- `'global'` scope is rejected by `QueryScopeSchema`.
- Free-text fields are rejected; only the scoped IDs are accepted.

If the contract regresses, this suite fails first.

## 16. Capture-session bridge

The bridge is a chain of three layers that wraps Kindling's storage SDK:

1. **Anvil call site** uses an emitter helper (e.g. `emitSessionStart`,
   `emitGateEvaluated`).
   - `packages/kindling-integration/src/emitters/session-emitter.ts:1-131`,
     `gate-emitter.ts`, etc. The seven emitter files share a fire-and-forget
     shape; emit failures are swallowed so the caller's happy path is never
     broken.
2. **`KindlingService`** validates the observation against its Zod schema, runs
   `validateNoSensitiveData()`, redacts via `redactSensitiveFields()` if
   anything matches, and delegates to the abstract `IKindlingStore`
   (`packages/kindling-integration/src/kindling-service.ts:122-195`).
   - `IKindlingStore` is **abstract** (`kindling-service.ts:36-53`) — emit +
     query + close. The package compiles without `@kindling/core` /
     `@kindling/store-sqlite` because of this decoupling
     (`kindling-service.ts:7-12`).
   - `NoOpKindlingStore` (`kindling-service.ts:63-85`) is the fallback when
     Kindling is disabled or no store is provided.
   - `createKindlingService(config, store?)` (line 265) is the factory.
3. **`AnvilKindlingAdapter`** (`adapter.ts:51-117`) bridges Anvil's eleven
   domain-specific schemas onto Kindling-core's three generic kinds (`message`,
   `command`, `error`) via `KIND_MAP` (`adapter.ts:20-32`). The original Anvil
   kind is preserved as `provenance.anvil_kind`. `startSession` (line 65) opens
   a Kindling capsule; `endSession` (line 81) closes it; `emit` (line 94)
   appends an observation.

**Configuration** (`config.ts:68-92`,
`packages/kindling-integration/README.md:128-156`):

- `enabled`: default **false** (opt-in).
- `database_path`: default `.anvil/kindling.db`.
- `retention.days`: default 90; `retention.auto_prune`: default false.
- `capture.{sessions,plans,gates,actions,constraints,human_inputs,errors}`:
  default true (each).
- `query_limits.max_results` / `max_payload_bytes`: as in §15.2.
- `loadKindlingConfig(projectRoot)` (line 125) reads `.anvilrc` or
  `anvil.config.json`; falls back to `DEFAULT_KINDLING_CONFIG` on any parse /
  shape error so the service degrades to disabled rather than crashing.

`shouldCapture(config, kind)` (line 174) is the per-kind switch; if
`config.enabled` is false, all kinds are dropped.

**Sensitive-data redaction**
(`packages/kindling-integration/src/sensitive-data-validator.ts:29-60`):
patterns include `sk-…` (OpenAI), `ghp_…` (GitHub PAT), `github_pat_…`, `AKIA…`
(AWS access key), `aws_secret_access_key=…`, hex tokens (40+ chars). Redaction
replaces matches with `[REDACTED:<label>]` markers.

## 17. Scripts (`packages/kindling-integration/scripts/`)

Single script: `scripts/generate-openapi.ts` (576 lines). Generates an OpenAPI
3.1 spec from the four query endpoints by **manual** schema extraction (no
`zod-to-json-schema` dep, `scripts/generate-openapi.ts:6-7`):

- `GET /sessions/{id}` → `SessionQuery`.
- `GET /plans/{id}` → `PlanQuery`.
- `GET /gates/{id}` → `GateQuery`.
- `GET /actions/{id}` → `ActionQuery`.

Output: `openapi.json` at the package root. Wired through
`packages/kindling-integration/package.json:94` as `pnpm generate:openapi`. Use
cases (per `packages/kindling-integration/README.md:478-489`): client-library
generation across TS / Python / Go / Rust, Swagger-UI / Redoc rendering,
contract-test scaffolding.

## 18. Benchmarks (`packages/kindling-integration/benchmarks/`)

Single bench file: `benchmarks/emission-overhead.bench.ts` (264 lines).
Validates the **< 50 ms** emission acceptance criterion from
`plans/archive/modules/kindling-integration.aps.md` KINDLING-017.

- Run: `pnpm bench --filter kindling-integration`
  (`packages/kindling-integration/package.json:93`,
  `packages/kindling-integration/README.md:497-499`).
- With a no-op store, observed budget is **< 1 ms per observation**
  (`packages/kindling-integration/README.md:501-503`).
- Exercised observation kinds: `session_start`, `session_end`, `gate_evaluated`,
  `action_executed`, `error` (`benchmarks/emission-overhead.bench.ts:13-28`).
  Each uses a fixed fixture so the benchmark is deterministic across runs.

Results are not committed to the repo — the bench is a CI-side guard, not an
artefact-producing pipeline.

---

## Combined sections

## 19. Cross-cutting concerns

### 19.1 TS / Rust boundary

Of the three packages, only **`packages/aps/`** has a structural Rust
counterpart — the Rust CLI's `anvil validate` and `anvil aps load` surfaces
re-implement parsing in Rust for performance, and the TS APS library is consumed
by the build pipeline + `packages/adapters/aps-markdown/`. `packages/adapters/`
is **TS only**: SpecKit / BMAD / Generic / APS- markdown all run in the JS
toolchain (linked into Rust via the agent-facing CLI shell, not in-process).
`packages/kindling-integration/` is **TS only** — the Rust CLI calls into
Kindling through the SQLite file directly when it needs to (a Rust port is not
on the slate per `plans/archive/modules/kindling-integration.aps.md:182-194`).

### 19.2 Versioning

All three packages are at `0.5.1-beta` package version
(`packages/adapters/package.json:3`, `packages/aps/package.json:3`,
`packages/kindling-integration/package.json:3`) — they track the **most recent
published anvil release**, not the `v0.6.0-beta` slate under construction. None
is gated on a Rust-binary version pin. This matches the wider monorepo pattern:
Rust crates and TS packages move together at release tags, but the in-flight
branch can carry mismatched versions.

`@eddacraft/kindling-core` is pinned at `0.1.1`
(`packages/kindling-integration/package.json:98`), **not workspace**. A bump
there is a deliberate dep change rather than a co-build.

### 19.3 Test infrastructure

All three packages use **`vitest ^4.1.5`** as the test runner
(`packages/adapters/package.json:21`, `packages/aps/package.json:32`,
`packages/kindling-integration/package.json:104`). Each carries its own
`vitest.config.ts`. There are **no shared fixtures** across the three — fixtures
live under each package's `__fixtures__/` or `__tests__/fixtures/`.

Test surface at HEAD `cf7ca040`:

- `packages/adapters/src/__tests__/`: 9 top-level test files, 5 022 LOC total.
  Per-feature `__tests__/` dirs in `base/`, `aps-markdown/`, `generic/` add
  another ~1 200 LOC.
- `packages/aps/src/`: per-module `*.test.ts` next to source. Validator, parser,
  loader, filter, state, templates each have a dedicated test file totalling ~3
  600 LOC.
- `packages/kindling-integration/src/malicious-ai.test.ts`: 949 LOC, 60 tests
  (sole test file in the live codebase besides the bench).

### 19.4 Determinism

All three packages produce deterministic outputs for fixed inputs:

- **Adapters**: `generateDeterministicPlanId(content)` hashes content with
  SHA-256 and prefixes `aps-` (`packages/adapters/src/base/utils.ts:11-14`).
  `generateHash(plan)` (from `@eddacraft/anvil-core`) is called consistently
  across SpecKit, BMAD, Generic, APS-markdown adapters.
- **APS**: `parseDocument` / `parseIndex` are AST-deterministic via
  remark-parse + unist-util-visit. `computeTaskHash`
  (`packages/aps/src/state/index.ts`) is content-addressed.
- **Kindling-integration**: emission has no randomness in the validation path.
  Observations carry `crypto.randomUUID()` (e.g. NoOpKindlingStore query
  response, `kindling-service.ts:71`) which is non-deterministic by design — but
  the **contract validation** is deterministic.

## 20. Known gaps

Dated against `v0.6.0-beta` slate (HEAD `cf7ca040`).

### G-01: Open-Spec adapter not yet shipped

`plans/modules/open-spec-adapter.aps.md` declares OPENSPEC at status **Draft**;
no source for it is present in `packages/adapters/src/`. Open-spec documents
currently fall through to the Generic fallback adapter, which scores them at
30–45 % confidence and parses them as free-form markdown rather than
understanding the open-spec schema. **Risk:** Low — Open-Spec is not yet a
load-bearing format for any v0.6.0-beta consumer. **Fix:** OPENSPEC-001 through
OPENSPEC-006 in `plans/modules/open-spec-adapter.aps.md`.

### G-02: BMAD v4 backward compatibility absent

`plans/modules/bmad-v4-backward-compat.aps.md` (BMAD4, Proposed, 0/8 tasks). The
current BMAD adapter recognises v6.0.3 and v5 legacy paths only; v4 documents
(June 2025–September 2025) fall through to the Generic adapter. Detection gaps
documented in `plans/modules/bmad-v4-backward-compat.aps.md:34-52`. **Risk:**
Medium for users on a v4 BMAD project — silent format misidentification.
**Fix:** BMAD4-001 through BMAD4-008.

### G-03: BMAD adapter version drift between code and README

`packages/adapters/src/bmad/format-adapter.ts:43` declares `version: '0.1.2'`;
`packages/adapters/README.md:191` documents BMAD adapter `version: 1.0.0`. The
string is metadata-only (consumed by `AdapterMetadata` for telemetry-style
reporting), not load-bearing for parsing, but the documented version is wrong.
**Risk:** Low. **Fix:** update either side; preference is to bump the live
constant since the adapter is documented as Complete.

### G-04: Kindling observation count drift between header comment and body

`packages/kindling-integration/src/observation-contract.ts:4` says "Defines the
9 observation kinds that Anvil must emit to Kindling." The file actually defines
**eleven** (verified by `grep -E '^export const \w+ObservationSchema'` — 11
hits; `packages/kindling-integration/CONTRACTS.md:38-54` and `README.md:88-101`
also say 11; the test assertions count 11; the OpenAPI generator at
`scripts/generate-openapi.ts:35-44` enumerates 11 kinds). The "9" comment is a
stale carry-over from an earlier scoping pass. **Risk:** Low
(documentation-only). **Fix:** one-line comment update.

### G-05: APS module-status enum drift between live schema and public docs

`packages/aps/src/types/index.ts:95` declares
`ModuleStatusSchema = z.enum(['Proposed', 'Ready', 'In Progress', 'Done', 'Blocked'])`
with a parser-side normalisation note that legacy values (`Draft`, `Complete`)
are accepted on parse and normalised to the canonical `Proposed` / `Done`. The
public APS schema doc at `docs/public/aps/schemas/json-schema.md` (around the
`ModuleMetadata` section) declares the enum as
`'Draft' | 'Ready' | 'In Progress' | 'Complete' | 'Blocked'` — the **legacy**
values, not the canonical ones. New users of the public spec will write
`Status: Draft` and the parser will normalise it silently, but agents that read
the schema verbatim will produce non-canonical output. **Risk:** Medium for
downstream agents that generate APS documents from the public schema. **Fix:**
update `docs/public/aps/schemas/json-schema.md` to declare the canonical enum
and document the legacy aliases as parser-side compatibility.

### G-06: APS package has no per-package CLI wrapper

`packages/aps/package.json` exposes no `bin` entry — the only callable script is
`scripts/generate-templates.js`, and it depends on `dist/` so it requires a
build first. The `anvil validate` Rust CLI is the operator-facing surface;
there's no `anvil-aps validate` or similar direct wrapper, which means users
wanting to validate `.aps.md` files **without the Rust CLI** have to import the
library programmatically. **Risk:** Low — by design, but worth flagging for
operators expecting a standalone tool.

### G-07: `packages/adapters/common/` legacy shape lingers

`packages/adapters/src/common/{types,registry}.ts` (158 LOC combined) exports
the legacy `SpecToolAdapter` interface and a parallel registry that **no live
adapter implements** (`packages/adapters/src/index.ts:24-30` only re-exports
types for backward compatibility). The package marks the directory as
"deprecated" in `README.md:460`. **Risk:** Low — it's tree-shakable and the
types still satisfy old downstream callers. **Fix:** schedule a removal pass
once the install-base survey shows no live consumer.

### G-08: Auto-registration coupling on import

`packages/adapters/src/index.ts:32-45` calls
`baseRegistry.register(new <Adapter>())` at module load. Importing this module
twice in a single process throws "Adapter '<name>' is already registered"
(`packages/adapters/src/base/registry.ts:46-50`). Test suites that need a clean
registry must call `AdapterRegistry.resetInstance()` (`registry.ts:36`) before
re-importing — otherwise the second import crashes. **Risk:** Low (only bites in
dual-bundler / hot-reload setups). **Fix:** make `register()` idempotent or
guard the auto-registration block with a `has()` check.

## 21. Source references

### 21.1 `packages/adapters/`

- `packages/adapters/src/index.ts` — barrel + auto-registration.
- `packages/adapters/src/base/types.ts` — `FormatAdapter` interface,
  `BaseFormatAdapter` abstract, `AdapterMetadata`, `DetectionResult`,
  `ParseResult`, `SerializeResult`, `AdapterError`, `AdapterWarning`,
  `ParseContext`, `PathDetectionHint`, `AdapterOptions`.
- `packages/adapters/src/base/registry.ts` — `AdapterRegistry` singleton,
  `detectAdapter`, `detectAdapterWithPath`, `detectAll`.
- `packages/adapters/src/base/utils.ts` — `generateDeterministicPlanId`,
  `createDetection`, `createError`, `createWarning`.
- `packages/adapters/src/base/file-discovery.ts` — repo-walking document
  discovery (used by `findPlanningDocuments`).
- `packages/adapters/src/base/testing.ts` — fixture / harness helpers.
- `packages/adapters/src/speckit/format-adapter.ts` — `SpecKitFormatAdapter`
  (595 lines).
- `packages/adapters/src/speckit/parser.ts`,
  `parsers/{spec,plan,tasks}-parser.ts`, `import.ts`, `import-v2.ts`,
  `export.ts`.
- `packages/adapters/src/bmad/format-adapter.ts` — `BMADFormatAdapter` (278
  lines) plus `parser.ts`, `serializer.ts`, `types.ts`, `utils.ts` (1 275
  lines).
- `packages/adapters/src/generic/format-adapter.ts` — `GenericMarkdownAdapter`,
  with supporting `parser.ts`, `serializer.ts`, `utils.ts`.
- `packages/adapters/src/aps-markdown/adapter.ts` — `APSMarkdownAdapter` (462
  lines).
- `packages/adapters/src/common/{types,registry}.ts` — legacy `SpecToolAdapter`
  shape (kept for backward compatibility, no live registration).
- `packages/adapters/src/__tests__/*` — 9 top-level test files + fixtures.

### 21.2 `packages/aps/`

- `packages/aps/src/index.ts` — barrel re-export.
- `packages/aps/src/types/index.ts` — Zod schemas and TypeScript types (`Task`,
  `ModuleMetadata`, `ParsedDocument`, `Confidence`, `Priority`, `TaskStatus`,
  `ModuleStatus`, `TASK_ID_REGEX`).
- `packages/aps/src/parser/{parse-document,parse-index,parse-task}.ts` and
  `index.ts` — remark / unified AST parsers.
- `packages/aps/src/loader/index.ts` — `loadPlan`, `LoadedPlan`, `LoadedModule`,
  `detectCycles`, `resolvePath`.
- `packages/aps/src/validator/index.ts` — `validatePlanningDoc`,
  `formatValidationIssues`, eight rules.
- `packages/aps/src/filter/{index,context-bundle}.ts` — `FilterCriteria`,
  context-bundle generation for LLM consumption.
- `packages/aps/src/state/index.ts` — `TaskLocker`, `StateFile`,
  `ProvenanceSchema`, `ExecutionPlanSchema`, lock / unlock / status /
  execution-plan generation.
- `packages/aps/src/templates/{generator,index}.ts` — three-variant template
  generator.
- `packages/aps/scripts/generate-templates.js` — emit nine template files into a
  directory.
- `packages/aps/templates/*.md` — nine static template files (committed).
- `packages/aps/examples/*` — five example documents (incl. `system-ecommerce/`
  multi-module index and `nested-index/`).

### 21.3 `packages/kindling-integration/`

- `packages/kindling-integration/CONTRACTS.md` — one-page contract summary.
- `packages/kindling-integration/src/observation-contract.ts` — eleven
  observation Zod schemas + `ObservationSchema` union + `validateObservation` +
  `containsSensitiveData`.
- `packages/kindling-integration/src/query-contract.ts` — four query scopes +
  `QueryRequestBaseSchema` + `QueryResponseSchema` + `ProvenanceLinkSchema` +
  `validateQueryRequest` / `validateQueryResponse`.
- `packages/kindling-integration/src/kindling-service.ts` — `KindlingService`,
  `IKindlingStore`, `NoOpKindlingStore`, `createKindlingService`,
  `ObservationValidationError`, `QueryValidationError`.
- `packages/kindling-integration/src/query-service.ts` — `KindlingQueryService`
  convenience.
- `packages/kindling-integration/src/query-limits.ts` — `enforceQueryLimits`,
  `limitsFromConfig`.
- `packages/kindling-integration/src/sensitive-data-validator.ts` —
  `validateNoSensitiveData`, `redactSensitiveFields`.
- `packages/kindling-integration/src/retention.ts` — `IRetentionCapableStore`,
  `pruneOldObservations`, `getStorageStats`.
- `packages/kindling-integration/src/status.ts` — `getKindlingStatus`,
  `formatKindlingStatus`.
- `packages/kindling-integration/src/config.ts` — `KindlingConfigSchema`,
  `loadKindlingConfig`, `shouldCapture`, `DEFAULT_KINDLING_CONFIG`.
- `packages/kindling-integration/src/adapter.ts` — `AnvilKindlingAdapter`
  bridging eleven Anvil kinds → Kindling-core's three generic kinds.
- `packages/kindling-integration/src/emitters/*.ts` — seven emitter helpers.
- `packages/kindling-integration/src/malicious-ai.test.ts` — 60-test
  read-only-enforcement suite.
- `packages/kindling-integration/scripts/generate-openapi.ts` — OpenAPI 3.1 spec
  generator.
- `packages/kindling-integration/benchmarks/emission-overhead.bench.ts` — < 50
  ms emission budget guard.

## 22. Related docs

- Public APS spec — [`docs/public/aps/spec/`](../public/aps/spec/)
  (`taxonomy.md`, `file-layout.md`, `determinism.md`).
- Public APS schemas —
  [`docs/public/aps/schemas/json-schema.md`](../public/aps/schemas/json-schema.md)
  (note enum drift in G-05).
- Public APS examples —
  [`docs/public/aps/examples.md`](../public/aps/schemas/examples.md).
- Public Kindling docs — [`docs/public/kindling/`](../public/kindling/)
  (`overview.md`, `concepts/{capsules,observations,retrieval,storage}.md`).
- [`docs/architecture/edda-stack.md`](edda-stack.md) — Kindling sits at layer 1
  of the Edda stack (Kindling = observation; Ember = interpretation; Edda =
  memory). `packages/kindling-integration/` ships the layer-1 contract.
- [`docs/architecture/checks-as-built.md`](checks-as-built.md) — the
  `anvil-checks` pipeline cross-references artefact-kind feeds; adapter-produced
  APSPlans flow through `anvil validate` / `anvil gate` into the same registry.
- [`docs/architecture/api-as-built.md`](api-as-built.md) — sister TypeScript
  as-built (apps/anvil-api). Shape reference for this document.
- [`docs/architecture/auth-as-built.md`](auth-as-built.md) — reference
  implementation for the as-built shape.
- [`RELEASE-PLAN.md`](../../RELEASE-PLAN.md) — adapter readiness shipping
  context for `v0.6.0-beta` (SpecKit + BMAD complete, OPENSPEC + BMAD4 in
  progress).
- [`CHANGELOG.md`](../../CHANGELOG.md) — `0.5.0-beta` Added: BMAD v6 YAML
  support, APS Markdown adapter, watch event adapter double-counting fix.
- APS modules:
  - [`plans/archive/modules/aps-markdown-adapter.aps.md`](../../plans/archive/modules/aps-markdown-adapter.aps.md)
    — APSMD, Complete.
  - [`plans/archive/modules/kindling-integration.aps.md`](../../plans/archive/modules/kindling-integration.aps.md)
    — KINDLING, Complete.
  - [`plans/modules/bmad-v4-backward-compat.aps.md`](../../plans/modules/bmad-v4-backward-compat.aps.md)
    — BMAD4, Proposed.
  - [`plans/modules/open-spec-adapter.aps.md`](../../plans/modules/open-spec-adapter.aps.md)
    — OPENSPEC, Draft.
