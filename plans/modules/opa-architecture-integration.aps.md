<!-- APS: See https://github.com/EddaCraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# OPA & Architecture Integration

| Scope | Owner | Priority | Status |
| ----- | ----- | -------- | ------ |
| OPA   | —     | high     | Ready  |

## Purpose

Unify architecture enforcement and policy evaluation into a cohesive system
where:

1. Users without OPA still get dependency-cruiser analysis
2. Users with OPA get DC results fed into policy evaluation
3. Architecture rules defined in YAML auto-generate both DC config and Rego
   policies

## In Scope

- Architecture definition YAML schema and parser
- DC config generation from architecture.yaml
- Rego policy generation from architecture.yaml
- DC → OPA bridge (inject architecture context into OPA input)
- Architecture templates (Layered, Hexagonal, Clean, DDD)
- Remote policy bundle support (basic auth + signature verification)
- Custom TypeScript-based import analyser (DC replacement, experimental)

## Out of Scope

- Replacing existing anti-pattern detection (stays AST-grep for speed)
- Real-time Rego evaluation on save (gate-time only)
- Multi-language support (TypeScript/JavaScript only)
- Visual dependency graph (v1.2+)
- Entra ID / Azure AD authentication (later release)

## Interfaces

**Depends on:**

- `architecture-safety` — baseline system, types
- `save-time-trust` — check runner, warning schema
- OPA binary manager (existing in `core/src/gate/policy/`)
- Policy loader (existing in `core/src/gate/policy/`)

**Exposes:**

- `ArchitectureDefinitionSchema` — Zod schema for architecture.yaml
- `ArchitectureCompiler` — generates DC config + Rego from YAML
- `ArchitectureContext` — DC results structured for OPA input
- Architecture templates (4 presets: layered, hexagonal, clean, ddd)
- `BundleManager` — remote policy distribution with caching

## Boundary Rules

- OPA module must not modify anti-pattern detection
- Generated files go to `.anvil/` only (never project root)
- Templates are read-only presets, not user-editable in place
- PolicyCheck must remain functional when OPA is not installed

## Acceptance Criteria

- [ ] `anvil architecture init` creates architecture.yaml from template
- [ ] DC config auto-generated to `.anvil/dependency-cruiser.js`
- [ ] Rego policies auto-generated to `.anvil/policies/.generated/`
- [ ] PolicyCheck receives DC violations in input context
- [ ] Users without OPA still get architecture warnings from DC
- [ ] Remote bundles downloadable with signature verification
- [ ] All four architecture templates validated and documented

## Risks & Mitigations

| Risk                                  | Mitigation                                  |
| ------------------------------------- | ------------------------------------------- |
| Generated files out of sync with YAML | Auto-regenerate on yaml change; hash check  |
| OPA binary download fails (airgapped) | Support ANVIL_OPA_PATH environment override |
| Template doesn't fit user's structure | Allow custom layer definitions in YAML      |
| Remote bundle compromised             | Signature verification required             |
| DC results too large for OPA input    | Summarise violations, not full graph        |

## Tasks

### Phase A: Architecture Definition System

#### OPA-001: Architecture YAML schema

- **Intent:** Define Zod schema for architecture.yaml with template support
- **Expected Outcome:** Schema validates layer definitions, rules, template
  selection
- **Scope:** `core/src/architecture/`
- **Non-scope:** Parsing or file I/O
- **Files:** `core/src/architecture/definition-schema.ts`
- **Dependencies:** —
- **Validation:** `nx test core --testNamePattern="definition-schema"`
- **Confidence:** high

#### OPA-002: YAML parser with template expansion

- **Intent:** Parse architecture.yaml and expand template defaults
- **Expected Outcome:** Parser loads YAML, merges with template, returns typed
  definition
- **Scope:** `core/src/architecture/`
- **Non-scope:** Config generation
- **Files:** `core/src/architecture/yaml-parser.ts`
- **Dependencies:** OPA-001
- **Validation:** `nx test core --testNamePattern="yaml-parser"`
- **Confidence:** high

#### OPA-003: DC config generator

- **Intent:** Generate dependency-cruiser config from architecture definition
- **Expected Outcome:** Creates `.anvil/dependency-cruiser.js` with layer rules
- **Scope:** `core/src/architecture/`
- **Non-scope:** Rego generation
- **Files:** `core/src/architecture/dc-generator.ts`
- **Dependencies:** OPA-002
- **Validation:** `nx test core --testNamePattern="dc-generator"`
- **Confidence:** high

#### OPA-004: Architecture init command

- **Intent:** Interactive wizard to create architecture.yaml from template
- **Expected Outcome:** `anvil architecture init --template hexagonal` creates
  YAML
- **Scope:** `cli/src/commands/`
- **Non-scope:** TUI components (use existing)
- **Files:** `cli/src/commands/architecture.ts`
- **Dependencies:** OPA-002
- **Validation:** Manual test of `anvil architecture init`
- **Confidence:** medium

### Phase B: DC → OPA Bridge

#### OPA-005: Architecture context extraction

- **Intent:** Extract DC results into structured context for OPA
- **Expected Outcome:** ArchitectureCheck exposes violations, layers, module
  stats
- **Scope:** `core/src/gate/checks/`
- **Non-scope:** OPA execution
- **Files:** `core/src/gate/checks/architecture.check.ts`
- **Dependencies:** OPA-003
- **Validation:** `nx test core --testNamePattern="architecture.check"`
- **Confidence:** high

#### OPA-006: OPA input enhancement

- **Intent:** Add architecture context to OPA input schema
- **Expected Outcome:** PolicyCheck receives DC violations, dependency info,
  layers
- **Scope:** `core/src/gate/policy/`
- **Non-scope:** Architecture analysis
- **Files:** `core/src/gate/policy/opa-executor.ts`
- **Dependencies:** OPA-005
- **Validation:** `nx test core --testNamePattern="opa-executor"`
- **Confidence:** high

#### OPA-007: Gate runner integration

- **Intent:** Wire architecture context flow between checks
- **Expected Outcome:** ArchitectureCheck runs before PolicyCheck, results
  passed through
- **Scope:** `core/src/gate/`
- **Non-scope:** Check implementations
- **Files:** `core/src/gate/gate-runner.ts`
- **Dependencies:** OPA-005, OPA-006
- **Validation:** `nx test core --testNamePattern="gate-runner"`
- **Confidence:** high

### Phase C: Rego Generation

#### OPA-008: Rego generator from architecture

- **Intent:** Generate architecture boundary policies as Rego
- **Expected Outcome:** `.anvil/policies/.generated/architecture.rego` created
  from YAML
- **Scope:** `core/src/architecture/`
- **Non-scope:** Policy evaluation
- **Files:** `core/src/architecture/rego-generator.ts`
- **Dependencies:** OPA-002
- **Validation:** `nx test core --testNamePattern="rego-generator"`
- **Confidence:** medium

#### OPA-009: Generated policy marker

- **Intent:** Mark generated policies to distinguish from user policies
- **Expected Outcome:** Policy loader identifies and labels generated policies
- **Scope:** `core/src/gate/policy/`
- **Non-scope:** Policy generation
- **Files:** `core/src/gate/policy/policy-loader.ts`
- **Dependencies:** OPA-008
- **Validation:** `nx test core --testNamePattern="policy-loader"`
- **Confidence:** high

#### OPA-010: Auto-regeneration on YAML change

- **Intent:** Regenerate DC config + Rego when architecture.yaml changes
- **Expected Outcome:** Pre-check regeneration or file watcher triggers rebuild
- **Scope:** `cli/src/commands/`, `core/src/architecture/`
- **Non-scope:** Live reload
- **Files:** `cli/src/commands/architecture.ts`,
  `core/src/architecture/compiler.ts`
- **Dependencies:** OPA-003, OPA-008
- **Validation:** Manual test of YAML change detection
- **Confidence:** medium

### Phase D: Architecture Templates

#### OPA-011: Layered architecture template

- **Intent:** Traditional 3-tier template (presentation → business → data)
- **Expected Outcome:** Template YAML with standard layer definitions
- **Scope:** `core/src/architecture/templates/`
- **Non-scope:** Template loading logic
- **Files:** `core/src/architecture/templates/layered.yaml`
- **Dependencies:** OPA-001
- **Validation:** Template validates against schema
- **Confidence:** high

#### OPA-012: Hexagonal architecture template

- **Intent:** Ports & Adapters template with core isolation
- **Expected Outcome:** Template YAML with port/adapter layer definitions
- **Scope:** `core/src/architecture/templates/`
- **Non-scope:** Template loading logic
- **Files:** `core/src/architecture/templates/hexagonal.yaml`
- **Dependencies:** OPA-001
- **Validation:** Template validates against schema
- **Confidence:** high

#### OPA-013: Clean Architecture template

- **Intent:** Uncle Bob's Clean Architecture with inward dependencies
- **Expected Outcome:** Template YAML with entities → use cases → adapters →
  frameworks
- **Scope:** `core/src/architecture/templates/`
- **Non-scope:** Template loading logic
- **Files:** `core/src/architecture/templates/clean.yaml`
- **Dependencies:** OPA-001
- **Validation:** Template validates against schema
- **Confidence:** high

#### OPA-014: DDD template with bounded contexts

- **Intent:** Domain-Driven Design with context boundaries
- **Expected Outcome:** Template YAML supporting bounded contexts and context
  mapping rules
- **Scope:** `core/src/architecture/templates/`
- **Non-scope:** Template loading logic
- **Files:** `core/src/architecture/templates/ddd.yaml`
- **Dependencies:** OPA-001
- **Validation:** Template validates against schema
- **Confidence:** medium
- **Risks:** DDD contexts are more complex than simple layers

#### OPA-015: Template loader

- **Intent:** Load and validate architecture templates
- **Expected Outcome:** Template loader with list, get, validate operations
- **Scope:** `core/src/architecture/templates/`
- **Non-scope:** Template content
- **Files:** `core/src/architecture/templates/index.ts`
- **Dependencies:** OPA-011, OPA-012, OPA-013, OPA-014
- **Validation:** `nx test core --testNamePattern="templates"`
- **Confidence:** high

### Phase E: TypeScript Import Analyser (DC Replacement)

#### OPA-016: TypeScript analyser foundation

- **Intent:** Build TypeScript compiler API-based import analyser
- **Expected Outcome:** Accurate multi-line import detection, handles all import
  types
- **Scope:** `core/src/architecture/`
- **Non-scope:** Path resolution
- **Files:** `core/src/architecture/ts-analyser.ts`
- **Dependencies:** —
- **Validation:** `nx test core --testNamePattern="ts-analyser"`
- **Confidence:** medium
- **Risks:** TypeScript API complexity; edge cases with barrel files

#### OPA-017: Path alias resolver

- **Intent:** Resolve tsconfig paths and barrel files
- **Expected Outcome:** Handles @/foo aliases, index.ts re-exports correctly
- **Scope:** `core/src/architecture/`
- **Non-scope:** Import extraction
- **Files:** `core/src/architecture/path-resolver.ts`
- **Dependencies:** OPA-016
- **Validation:** `nx test core --testNamePattern="path-resolver"`
- **Confidence:** medium

#### OPA-018: Analyser feature flag

- **Intent:** Allow switching between DC and TS analyser
- **Expected Outcome:** Config option to use experimental TS analyser
- **Scope:** `core/src/architecture/`
- **Non-scope:** Analyser implementations
- **Files:** `core/src/architecture/index.ts`
- **Dependencies:** OPA-016, OPA-017
- **Validation:** Both analysers produce equivalent results on test codebase
- **Confidence:** medium

### Phase F: Remote Policy Bundles

#### OPA-019: Bundle download and caching

- **Intent:** Download policy bundles from URLs with local caching
- **Expected Outcome:** Bundles downloaded to ~/.anvil/policy-cache with TTL
- **Scope:** `core/src/gate/policy/`
- **Non-scope:** Authentication, verification
- **Files:** `core/src/gate/policy/bundle-manager.ts`
- **Dependencies:** —
- **Validation:** `nx test core --testNamePattern="bundle-manager"`
- **Confidence:** high

#### OPA-020: Signature verification

- **Intent:** Verify bundle signatures before use
- **Expected Outcome:** Bundles rejected if signature invalid or missing
- **Scope:** `core/src/gate/policy/`
- **Non-scope:** Bundle download
- **Files:** `core/src/gate/policy/bundle-verifier.ts`
- **Dependencies:** OPA-019
- **Validation:** `nx test core --testNamePattern="bundle-verifier"`
- **Confidence:** high

#### OPA-021: Basic auth and CLI commands

- **Intent:** Support basic auth for bundle downloads and CLI management
- **Expected Outcome:** `anvil policy bundle list|add|remove|sync` commands work
- **Scope:** `core/src/gate/policy/`, `cli/src/commands/`
- **Non-scope:** OAuth/Entra ID
- **Files:** `core/src/gate/policy/bundle-manager.ts`,
  `cli/src/commands/policy.ts`
- **Dependencies:** OPA-019, OPA-020
- **Validation:** `nx test core --testNamePattern="bundle"`, manual CLI test
- **Confidence:** high

## Decisions

- **D-006:** Hybrid DC + OPA — DC standalone for non-OPA users; DC feeds OPA
  when enabled ([ADR](../decisions/006-hybrid-dc-opa.md))
- **D-007:** YAML-first architecture — Human-friendly definition generates DC +
  Rego
- **D-008:** Generated files in `.anvil/` — Never pollute project root
- **D-009:** AST-grep for anti-patterns — Live/save detection needs speed; Rego
  gate-time only
- **D-010:** Basic auth first — Start simple; Entra ID deferred to later release

## Notes

- Existing OPA infrastructure (binary manager, executor, loader) is solid
  foundation
- dependency-cruiser integration already works, needs context extraction only
- Template design should allow future templates without code changes
- Bundle signature verification is non-negotiable for enterprise trust
- Phase E (TS analyser) is experimental; DC remains primary until proven
