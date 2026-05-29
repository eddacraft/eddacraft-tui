<!--
APS Module: Unified Config Format
=========================
Consolidate .anvilrc, gate-config.json, and architecture.yaml into a
single TOML file with generalised source delegation.

Scopes: UCFG (main)
-->

# Unified Config Format

| ID   | Owner | Status   | Progress |
| ---- | ----- | -------- | -------- |
| UCFG | —     | Proposed | 0/18     |

**Last reviewed:** 2026-04-26

## Purpose

Consolidate Anvil's three configuration files (`.anvilrc`,
`.anvil/gate-config.json`, `.anvil/architecture.yaml`) into a single `.anvilrc`
in TOML format with snake_case keys throughout. Sections that grow large can be
delegated to external TOML files via a `source` key.

**Why:** A Council review (council-09fc9567) found 4 critical and 7 major
documentation errors caused directly by schema drift across three files with
different formats, key casing, and versioning. AI agents — Anvil's primary
audience — must correlate three files to understand configuration. All projects
are greenfield; there are no existing users to migrate.

**ADR:** [016-unified-config-format](../decisions/016-unified-config-format.md)

## In Scope

- `SectionOrSource<T>` generic loader with source delegation
- Unified `AnvilConfig` struct with `[project]`, `[gate]`, `[architecture]`
  sections
- `anvil init` generates single TOML `.anvilrc`
- `anvil gate-config` reads/writes `[gate]` section in `.anvilrc`
- `anvil architecture` reads/writes `[architecture]` section (inline or
  delegated)
- `anvil doctor` validates all 4 topologies (inline/delegated per section)
- Updated documentation (config.md, first-project.md, agent-harness.md)

## Out of Scope

- Migration tooling for legacy formats (no existing users)
- Non-TOML format support for `.anvilrc`
- New sections beyond `[project]`, `[gate]`, `[architecture]`
- Changes to `.anvil/policies/` directory (OPA bundles stay separate)

## Interfaces

**Depends on:**

- `crates/anvil-architecture/` — ArchitectureDefinition types, template defaults
- `crates/anvil-cli/src/util.rs` — workspace_root(), atomic_write()

**Exposes:**

- `crates/anvil-config/` — new crate: `AnvilConfig`, `SectionOrSource<T>`,
  `load_config()`, `save_config()`
- Updated `anvil init`, `anvil gate-config`, `anvil doctor` commands

## Constraints

- TOML only — no JSON/YAML support for the root config file
- snake_case everywhere — no format-dependent key casing
- Source delegation is exclusive — no merge semantics between inline and
  delegated content
- Source delegation is one level deep — delegated files cannot themselves
  delegate
- `[project]` is always inline (contains root `schema_version`)
- Atomic writes for all config mutations
- `toml` crate already in the workspace; no new format dependencies

## Ready Checklist

Change status to **Ready** when:

- [ ] ADR-016 approved
- [ ] Work items reviewed and estimated

---

## Work Items

### Phase 1 — Config Crate and Loader

New `crates/anvil-config/` crate with the unified config type and
`SectionOrSource<T>` delegation.

#### UCFG-001: Scaffold anvil-config crate

- **Status:** Proposed
- **Intent:** Create `crates/anvil-config/` with `Cargo.toml`, workspace
  registration, `lib.rs` stub. Dependencies: `serde`, `toml`, `thiserror`
- **Expected Outcome:** `cargo check -p eddacraft-anvil-config` passes
- **Validation:** Workspace builds cleanly with the new crate
- **Files:** `Cargo.toml`, `crates/anvil-config/Cargo.toml`,
  `crates/anvil-config/src/lib.rs`
- **Confidence:** High
- **Priority:** High
- **Dependencies:** None

---

#### UCFG-002: Define AnvilConfig struct

- **Status:** Proposed
- **Intent:** Define the unified `AnvilConfig` with `ProjectConfig`,
  `GateConfig`, and architecture config. `[project]` section holds
  `schema_version`, `planning_dir`, `format`, `checks`. `[gate]` holds
  `overall_score`, `checks` (Vec of GateCheck), optional `global_config`.
  `[architecture]` holds the full `ArchitectureDefinition` fields
  (`schema_version`, `template`, `layers`, `bounded_contexts`, `rules`,
  `options`)
- **Expected Outcome:** All config types derive `Serialize + Deserialize` with
  snake_case. Round-trip test: struct → TOML string → struct
- **Validation:** `cargo test -p eddacraft-anvil-config` passes with round-trip tests
- **Files:** `crates/anvil-config/src/types.rs`,
  `crates/anvil-config/src/types.test.rs`
- **Confidence:** High
- **Priority:** High
- **Dependencies:** UCFG-001

---

#### UCFG-003: SectionOrSource<T> with custom deserializer

- **Status:** Proposed
- **Intent:** Implement `SectionOrSource<T>` enum (Inline(T) | Delegated(SourceRef))
  with a custom serde deserializer that detects the `source` key and produces
  clear error messages. Must handle: source-only (valid), inline-only (valid),
  both present (error with actionable message), neither (falls back to T's
  Default impl)
- **Expected Outcome:** Custom deserializer produces clear errors:
  `"[architecture] has both 'source' and inline keys"` instead of serde's
  default untagged enum error
- **Validation:** Tests for all 4 states (inline, delegated, both, empty) with
  error message assertions
- **Files:** `crates/anvil-config/src/delegation.rs`,
  `crates/anvil-config/src/delegation.test.rs`
- **Confidence:** Medium — custom deserializer complexity
- **Priority:** High
- **Dependencies:** UCFG-002

---

#### UCFG-004: Config loader with source resolution

- **Status:** Proposed
- **Intent:** Implement `load_config(workspace: &Path) -> Result<AnvilConfig>`
  that reads `.anvilrc`, parses TOML, resolves any `source` references (relative
  to workspace root), validates one-level-deep constraint (delegated files
  cannot themselves contain `source`), and returns the fully resolved config.
  Also implement `save_config()` using atomic_write()
- **Expected Outcome:** Loading a config with delegated `[architecture]` section
  reads the external file and returns it as if it were inline. Saving preserves
  delegation structure (does not flatten)
- **Validation:** Tests with temp dirs: inline config, delegated config,
  nested delegation (must error), missing source file (must error with path)
- **Files:** `crates/anvil-config/src/loader.rs`,
  `crates/anvil-config/src/loader.test.rs`
- **Confidence:** High
- **Priority:** High
- **Dependencies:** UCFG-003

---

#### UCFG-005: Template defaults for architecture section

- **Status:** Proposed
- **Intent:** Port the template default logic from
  `crates/anvil-architecture/src/yaml_parser.rs` (`get_template_defaults`,
  `merge_with_template`, `create_definition_from_template`) to work with the
  TOML-based architecture section. Reuse the existing template enum and layer
  definitions — the types are the same, only the serialisation format changes
- **Expected Outcome:** `anvil init` with a template selection produces a
  `.anvilrc` with pre-populated `[architecture.layers.*]` tables matching the
  existing YAML template output
- **Validation:** Test that each of the 9 templates produces valid TOML with
  correct layer names and patterns
- **Files:** `crates/anvil-config/src/templates.rs`,
  `crates/anvil-config/src/templates.test.rs`
- **Confidence:** High
- **Priority:** Medium
- **Dependencies:** UCFG-002

---

### Phase 2 — Command Integration

Wire the unified config into existing CLI commands, replacing per-command
config loading.

#### UCFG-006: Update anvil init

- **Status:** Proposed
- **Intent:** Replace `AnvilConfig` in `init.rs` with the unified config from
  `anvil-config`. `anvil init` generates a single `.anvilrc` in TOML with
  `[project]` and `[gate]` sections (architecture section added only if user
  selects a template). Remove JSON/YAML format selection for the config file
  itself (TOML only). Keep `format` field for plan file format
- **Expected Outcome:** `anvil init` produces a valid unified `.anvilrc`.
  Interactive mode still works (template selection, planning dir). The init
  surface (TUI) is updated to remove config format choice
- **Validation:** `anvil init` in a temp dir produces parseable TOML that
  round-trips through `load_config()`
- **Files:** `crates/anvil-cli/src/commands/init.rs`,
  `crates/anvil-tui/src/surfaces/init/`
- **Confidence:** High
- **Priority:** High
- **Dependencies:** UCFG-004, UCFG-005

---

#### UCFG-007: Update anvil gate-config

- **Status:** Proposed
- **Intent:** Replace `gate_config.rs` config loading with reads/writes to the
  `[gate]` section of `.anvilrc` via `anvil-config`. `--list`, `--enable`,
  `--disable` operate on the unified file. Remove `.anvil/gate-config.json`
  handling entirely
- **Expected Outcome:** `anvil gate-config --list` reads from `.anvilrc`.
  `--enable lint` / `--disable coverage` modify the `[gate]` section in place.
  The `.anvil/gate-config.json` file is no longer read or written
- **Validation:** Enable/disable round-trips: toggle a check, re-read, verify
  state. TOML formatting is preserved (no reordering keys)
- **Files:** `crates/anvil-cli/src/commands/gate_config.rs`
- **Confidence:** Medium — TOML in-place editing may reorder keys
- **Priority:** High
- **Dependencies:** UCFG-004

---

#### UCFG-008: Update anvil gate

- **Status:** Proposed
- **Intent:** Replace the gate command's independent config loading
  (`parse_architecture_definition`, inline gate check list) with a single
  `load_config()` call at the top of `run()`. Architecture checks use the
  resolved `[architecture]` section. Gate check enabled/disabled state comes
  from `[gate]`. Profile overrides (dev/ci/production) apply to the loaded
  config in memory
- **Expected Outcome:** Gate command works identically to today but reads from
  unified config. All 7 checks still function. Profile skipping still works
- **Validation:** Existing gate tests pass with unified config files. Manual
  test with all 3 profiles
- **Files:** `crates/anvil-cli/src/commands/gate.rs`
- **Confidence:** Medium — gate.rs is 1796 lines, many implicit config
  assumptions
- **Priority:** High
- **Dependencies:** UCFG-004

---

#### UCFG-009: Update anvil watch

- **Status:** Proposed
- **Intent:** Replace the watch command's architecture.yaml existence check
  with a `load_config()` call. If `[architecture]` is present (inline or
  delegated), watch mode includes architecture file patterns. Watch still
  monitors the `.anvilrc` file itself for config changes
- **Expected Outcome:** Watch command detects architecture config regardless of
  whether it's inline or delegated. Config file change triggers re-validation
- **Validation:** Watch mode starts with unified config; architecture boundary
  changes are detected
- **Files:** `crates/anvil-cli/src/commands/watch.rs`
- **Confidence:** High
- **Priority:** Medium
- **Dependencies:** UCFG-004

---

#### UCFG-010: Update anvil architecture commands

- **Status:** Proposed
- **Intent:** Update `architecture.rs`, `architecture-validate.rs`, and any
  other architecture commands to read from the unified config's
  `[architecture]` section instead of `.anvil/architecture.yaml` directly.
  `anvil architecture validate` must handle both inline and delegated configs.
  `anvil architecture show` renders from the resolved config
- **Expected Outcome:** All architecture commands work with unified config.
  Delegated configs are resolved transparently
- **Validation:** `anvil architecture validate` passes with inline config and
  with `source = ".anvil/architecture.toml"` delegation
- **Files:** `crates/anvil-cli/src/commands/architecture.rs`,
  `crates/anvil-cli/src/commands/architecture-validate.rs`,
  `crates/anvil-architecture/src/yaml_parser.rs` (deprecate or remove)
- **Confidence:** Medium
- **Priority:** Medium
- **Dependencies:** UCFG-004

---

#### UCFG-011: Update anvil doctor

- **Status:** Proposed
- **Intent:** Replace the existing config-exists and config-valid checks in
  `doctor.rs` with unified config validation. Doctor should: (1) check
  `.anvilrc` exists and is valid TOML, (2) validate all sections parse
  correctly, (3) resolve and validate any `source` references, (4) check
  architecture schema_version is "0.1.0", (5) validate layer dependency
  references. Auto-fix: create minimal `.anvilrc` with `[project]` section
- **Expected Outcome:** `anvil doctor` validates all 4 delegation topologies.
  Reports clear errors for missing source files, invalid TOML, schema version
  mismatches, and dangling layer references
- **Validation:** Test all 4 topologies plus error cases (missing source file,
  invalid TOML, wrong schema_version)
- **Files:** `crates/anvil-cli/src/commands/doctor.rs`
- **Confidence:** High
- **Priority:** Medium
- **Dependencies:** UCFG-004

---

### Phase 3 — Cleanup and Documentation

Remove legacy config paths and update all documentation.

#### UCFG-012: Remove legacy config loading

- **Status:** Proposed
- **Intent:** Delete all code paths that read `.anvil/gate-config.json` and
  `.anvil/architecture.yaml`. Remove `yaml_serialise()` and `toml_serialise()`
  from `init.rs`. Remove `GateConfig` and `GateCheck` structs from
  `gate_config.rs` (now in `anvil-config`). Remove or deprecate
  `yaml_parser.rs` in `anvil-architecture` (the YAML parsing is replaced by
  TOML in `anvil-config`)
- **Expected Outcome:** `grep -r "gate-config.json" crates/` returns zero
  results. `grep -r "architecture.yaml" crates/` returns zero results (except
  comments/docs). The `serde_yaml` dependency can be removed from
  `anvil-architecture` if no other code uses it
- **Validation:** `cargo check --workspace` passes. `cargo test --workspace`
  passes. No references to old config file paths remain
- **Files:** `crates/anvil-cli/src/commands/init.rs`,
  `crates/anvil-cli/src/commands/gate_config.rs`,
  `crates/anvil-architecture/src/yaml_parser.rs`,
  `crates/anvil-architecture/Cargo.toml`
- **Confidence:** Medium — may find unexpected consumers of old paths
- **Priority:** Medium
- **Dependencies:** UCFG-006, UCFG-007, UCFG-008, UCFG-009, UCFG-010, UCFG-011

---

#### UCFG-013: Update public documentation

- **Status:** Proposed
- **Intent:** Rewrite `docs/public/anvil/operations/config.md` to document the
  unified TOML format exclusively. Update `first-project.md`,
  `agent-harness.md`, `quickstart.md`, and any other docs that reference
  `.anvil/gate-config.json` or `.anvil/architecture.yaml` as separate files.
  Document the source delegation pattern with examples for both inline and
  delegated configs
- **Expected Outcome:** All public docs reference a single `.anvilrc` in TOML.
  No references to `.anvil/gate-config.json` or `.anvil/architecture.yaml` as
  user-facing config files (`.anvil/` may still be mentioned for cache/policies)
- **Validation:** `grep -r "gate-config.json" docs/public/` returns zero.
  `grep -r "architecture.yaml" docs/public/` returns zero (except in
  delegation examples as a source target)
- **Files:** `docs/public/anvil/operations/config.md`,
  `docs/public/anvil/first-project.md`,
  `docs/public/anvil/guides/agent-harness.md`,
  `docs/public/anvil/quickstart.md`,
  `docs/public/anvil/tutorials/architecture.md`
- **Confidence:** High
- **Priority:** Medium
- **Dependencies:** UCFG-006 (need final init output to document accurately)

---

#### UCFG-014: Update MCP server config resources

- **Status:** Proposed
- **Intent:** Update the MCP server (`archive/anvil-mcp-server/`) to read from
  the unified `.anvilrc` instead of three separate files. The
  `anvil://config` resource should return the resolved unified config.
  The `anvil://boundaries` resource should resolve architecture from the
  unified config (handling delegation transparently)
- **Expected Outcome:** MCP resources return correct data from unified config.
  AI agents see a single config surface through MCP
- **Validation:** MCP server starts, resources return expected data with both
  inline and delegated architecture configs
- **Files:** `archive/anvil-mcp-server/src/resources/`,
  `archive/anvil-mcp-server/src/config/`
- **Confidence:** Medium — MCP server is TypeScript, needs a TOML parser
- **Priority:** Low
- **Dependencies:** UCFG-004

---

#### UCFG-015: Update VS Code extension config

- **Status:** Proposed
- **Intent:** Update the VS Code extension's config reading to parse `.anvilrc`
  as TOML instead of the current JSON/YAML multi-format approach. The
  `anvil.configPath` setting default remains `.anvilrc`
- **Expected Outcome:** Extension reads unified TOML config, diagnostics work
  with new format
- **Validation:** Extension loads, inline diagnostics appear, no regressions
- **Files:** VS Code extension source (`.vsix` build)
- **Confidence:** Medium
- **Priority:** Low
- **Dependencies:** UCFG-004

---

### Phase 4 — Hardening

#### UCFG-016: Fuzz the config loader

- **Status:** Proposed
- **Intent:** Add `cargo-fuzz` targets for the config loader: malformed TOML,
  oversized files, deeply nested tables, source paths with path traversal
  (`../../etc/passwd`), UTF-8 edge cases. The loader must never panic and must
  produce actionable error messages for all malformed inputs
- **Expected Outcome:** Fuzz targets run for 10 minutes with no crashes.
  Path traversal attempts are rejected with a clear error
- **Validation:** `cargo fuzz run config_loader -- -max_total_time=600`
  completes with zero crashes
- **Files:** `crates/anvil-config/fuzz/`, `crates/anvil-config/fuzz/Cargo.toml`
- **Confidence:** High
- **Priority:** Low
- **Dependencies:** UCFG-004

---

#### UCFG-017: Config schema documentation in binary

- **Status:** Proposed
- **Intent:** Add `anvil config schema` command that prints the expected TOML
  schema as annotated example output (like `cargo init` generates a commented
  `Cargo.toml`). Include all sections with defaults, all fields with
  descriptions, and delegation syntax examples
- **Expected Outcome:** `anvil config schema` prints a complete, valid,
  commented `.anvilrc` that users can pipe to a file
- **Validation:** Output is valid TOML. Parsing the output through
  `load_config()` succeeds
- **Files:** `crates/anvil-cli/src/commands/config.rs`
- **Confidence:** High
- **Priority:** Low
- **Dependencies:** UCFG-002

---

#### UCFG-018: CI workflow updates

- **Status:** Proposed
- **Intent:** Update `.github/workflows/rust.yml` and `ci.yml` to use unified
  config format in test fixtures and CI steps. Any CI steps that reference
  `.anvil/gate-config.json` or `.anvil/architecture.yaml` must be updated.
  Test fixture configs in `crates/*/tests/` and `crates/*/test-fixtures/`
  must be converted to unified TOML
- **Expected Outcome:** CI passes with unified config. No references to legacy
  config paths in workflow files
- **Validation:** CI green on all platforms (Linux, macOS, Windows)
- **Files:** `.github/workflows/rust.yml`, `.github/workflows/ci.yml`,
  test fixture directories
- **Confidence:** High
- **Priority:** Medium
- **Dependencies:** UCFG-012

---

## Parallel Execution

```
Phase 1 (foundation):
  UCFG-001 → UCFG-002 → UCFG-003 → UCFG-004
                  └──→ UCFG-005 (parallel with 003)

Phase 2 (integration — all depend on UCFG-004, can run in parallel):
  UCFG-006 ─┐
  UCFG-007 ─┤
  UCFG-008 ─┤
  UCFG-009 ─┼──→ UCFG-012 (cleanup, after all phase 2)
  UCFG-010 ─┤
  UCFG-011 ─┘

Phase 3 (docs + integrations — after phase 2):
  UCFG-013 ─┐
  UCFG-014 ─┼──→ done
  UCFG-015 ─┘
  UCFG-018 ─┘

Phase 4 (hardening — after UCFG-004, can start early):
  UCFG-016 (after UCFG-004)
  UCFG-017 (after UCFG-002)
```
