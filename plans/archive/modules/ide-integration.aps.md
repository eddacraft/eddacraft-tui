<!-- APS: See https://github.com/EddaCraft/anvil-plan-spec for format reference -->
<!-- Executable when status is Ready and tasks are defined. -->

# IDE Integration (VS Code Extension)

| Scope | Owner | Priority | Status   | Release |
| ----- | ----- | -------- | -------- | ------- |
| IDE   | —     | high     | Complete | v1.2    |

## Purpose

Surface Anvil warnings directly in the developer's editor at file-save time. The
primary feedback loop — developers see architecture violations and anti-patterns
before they leave the file, with the same fidelity as CI.

**Why v1.2 (not v1.0):** CLI-first ensures core value works universally. IDE
integration amplifies adoption once the engine is proven.

## In Scope

### Phase 1: Foundation (v1.2.0)

- Hybrid architecture: embed lightweight `@eddacraft/anvil-core` for fast-path, CLI for
  heavy operations
- Anti-pattern detection on file save (< 100ms feedback)
- Diagnostics panel integration with accurate source locations
- Status bar showing validation state
- VSIX packaging and installation

### Phase 2: Architecture Gates (v1.2.1)

- Architecture gate results display (layer violations, dependency cycles)
- OPA policy failure display with policy references
- Gate results tree view in Explorer sidebar
- Click-to-navigate for violations

### Phase 3: Polish (v1.3.0)

- APS/Rego syntax highlighting
- Hover information for tasks and warnings
- Caching to avoid re-analysis of unchanged files
- Marketplace preparation (icon, description, screenshots)

## Out of Scope

- JetBrains plugin (separate module, post-v1.3)
- Auto-fix actions (v2 — don't be too clever)
- Language Server Protocol (LSP) — direct VS Code API is simpler for v1
- OPA WASM bundling in extension (keep in CLI, too heavy)
- Team dashboards or remote sync

## Interfaces

**Depends on:**

- `save-time-trust` — analysis runner, warning schema, anti-pattern detection
- `architecture-safety` — boundary detection, edge analysis, OPA policy engine
- `antipattern-library` — pattern catalogue and scanner
- `suppressions` — suppression parsing and filtering

**Exposes:**

- VS Code extension package (VSIX)
- Extension settings (12 configuration options)
- Commands (7 registered commands)
- Tree view provider (Gate Results)
- CodeLens provider (quick actions)

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                      VS Code Extension                          │
├─────────────────────────────────────────────────────────────────┤
│  FAST PATH (Embedded @eddacraft/anvil-core)   │  HEAVY PATH (CLI)         │
│  ────────────────────────────────   │  ─────────────────────    │
│  • Schema validation                │  • Full gate execution    │
│  • Anti-pattern detection           │  • OPA policy evaluation  │
│  • Format detection                 │  • Coverage analysis      │
│  • APS parsing                      │  • Dependency audit       │
│  • Hash calculation                 │  • Secret scanning        │
│                                     │  • Export/conversion      │
│  Target: < 100ms                    │  Target: < 5s             │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                       UI Components                             │
├─────────────────────────────────────────────────────────────────┤
│  DiagnosticsManager ──▶ Problems Panel (errors, warnings)       │
│  StatusBarManager ────▶ Status Bar (validation state)           │
│  GateResultsProvider ─▶ Tree View (gate results, violations)    │
│  CodeLensProvider ────▶ Inline Actions (validate, gate, export) │
│  PlanWatcher ─────────▶ Auto-validation on save                 │
└─────────────────────────────────────────────────────────────────┘
```

## Acceptance Criteria

### Phase 1: Foundation

- [ ] Extension installs from VSIX without errors
- [ ] Anti-pattern warnings appear within 200ms of file save
- [ ] Warnings show in Problems panel with correct file/line/column
- [ ] Clicking warning navigates to exact source location
- [ ] Status bar reflects current validation state
- [ ] Works offline (no network dependency for fast-path)

### Phase 2: Architecture Gates

- [ ] `anvil.gate` command runs all enabled gates
- [ ] Gate results appear in dedicated tree view
- [ ] Layer violations show source and target with explanation
- [ ] OPA policy failures show policy name and remediation hint
- [ ] Clicking violation opens offending file at import location

### Phase 3: Polish

- [ ] `.aps.md` files have semantic highlighting
- [ ] Hover over warning shows full explanation
- [ ] Unchanged files skip re-analysis (cache hit)
- [ ] Extension published to VS Code Marketplace

## Tasks

### Phase 1: Foundation (IDE-001 to IDE-003)

| Task    | Description                                     | Status | Priority |
| ------- | ----------------------------------------------- | ------ | -------- |
| IDE-001 | Embed @eddacraft/anvil-core for fast-path operations      | Done   | high     |
| IDE-002 | Anti-pattern detection on save with diagnostics | Done   | high     |
| IDE-003 | Improve source location mapping from CLI output | Done   | medium   |

### Phase 2: Architecture Gates (IDE-004 to IDE-006)

| Task    | Description                                 | Status | Priority |
| ------- | ------------------------------------------- | ------ | -------- |
| IDE-004 | Architecture gate display in tree view      | Done   | high     |
| IDE-005 | OPA policy failure display with remediation | Done   | high     |
| IDE-006 | Click-to-navigate for all violation types   | Done   | medium   |

### Phase 3: Polish (IDE-007 to IDE-008)

| Task    | Description                                  | Status | Priority |
| ------- | -------------------------------------------- | ------ | -------- |
| IDE-007 | APS and Rego syntax highlighting             | Done   | medium   |
| IDE-008 | Analysis caching and Marketplace preparation | Done   | medium   |

## Non-Functional Requirements

| Requirement        | Target                               |
| ------------------ | ------------------------------------ |
| Fast-path latency  | < 100ms for anti-pattern detection   |
| Heavy-path latency | < 5s for full gate run               |
| Bundle size        | < 2MB (extension + embedded core)    |
| Memory overhead    | < 50MB additional VS Code memory     |
| Activation time    | < 500ms from extension load to ready |
| Offline support    | Fast-path works without network      |

## Risks

| Risk                                    | Impact | Mitigation                                   |
| --------------------------------------- | ------ | -------------------------------------------- |
| Bundle size too large with embedded lib | Medium | Tree-shake; defer heavy deps to CLI          |
| CLI not installed / wrong version       | High   | Clear error message; link to install docs    |
| OPA policies not found                  | Medium | Graceful degradation; show "no policies"     |
| Performance regression on large files   | Medium | Debounce saves; skip files > 100KB           |
| VS Code API breaking changes            | Low    | Pin minimum VS Code version; test on updates |

## Configuration (Existing)

The extension already defines these settings in `package.json`:

| Setting                         | Default  | Description                  |
| ------------------------------- | -------- | ---------------------------- |
| `anvil.autoValidate`            | `true`   | Auto-validate on save        |
| `anvil.validateOnOpen`          | `true`   | Validate when file is opened |
| `anvil.showStatusBar`           | `true`   | Show status bar item         |
| `anvil.showCodeLens`            | `true`   | Show CodeLens actions        |
| `anvil.defaultFormat`           | `"auto"` | Default format detection     |
| `anvil.gates.enabled`           | all      | Which gates to run           |
| `anvil.gates.skipInDevelopment` | `[]`     | Gates to skip in dev mode    |
| `anvil.coverage.threshold`      | `80`     | Minimum coverage %           |
| `anvil.cli.path`                | `""`     | Custom CLI path              |

## Open Questions

- [ ] Should we show suppression hints inline (gutter icon)?
- [ ] How to handle workspaces with multiple `.anvilrc` files?
- [ ] Should gate results persist across VS Code restarts?
