# Save-Time Trust: Anvil v0.1.0 Pre-Release Plan

> Ship AI-generated code with confidence.

## Executive Summary

**Goal**: Ship a CLI-first pre-release that gives experienced developers
immediate value when using AI-generated code, without requiring plans, policies,
or process adoption.

**Target audience**: Experienced engineers who:

- Use AI coding tools (Cursor, Copilot, Claude)
- Care about architectural integrity
- Want guardrails, not gates

**Core value proposition**: Anvil catches architectural boundary violations and
dangerous anti-patterns at file-save time, with actionable guidance and
human-owned exceptions.

---

## Current State Summary

| Component                        | State       | Reuse for v0.1                       |
| -------------------------------- | ----------- | ------------------------------------ |
| dependency-cruiser integration   | ✅ Complete | Extend for baseline storage          |
| `anvil init` with env detection  | ✅ Complete | Extend with architecture exploration |
| Pattern detection (secret check) | ✅ Complete | Template for anti-pattern library    |
| Watch mode                       | ✅ Complete | Extend to watch source files         |
| Provenance system                | ✅ Complete | Extend for suppression tracking      |
| Gate runner + caching            | ✅ Complete | Direct reuse                         |
| JSON output schema               | ✅ Complete | Extend for warnings                  |

---

## Gap Analysis

| New Capability Needed                                      | Effort      |
| ---------------------------------------------------------- | ----------- |
| Architecture baseline storage (`.anvil/architecture.json`) | Medium      |
| Layer heuristic detection (controllers/services)           | Medium      |
| NEW edge detection (baseline diff)                         | Medium      |
| Anti-pattern library (detectors + explanations)            | Medium-High |
| Inline suppression parsing (JSDoc style)                   | Low         |
| `anvil check` command (source file analysis)               | Low         |
| Watch source files (not just plans)                        | Low         |
| Warning schema v1                                          | Low         |

---

## v0.1.0 Scope

### Phase 0: Foundation

#### 0.1 Warning Schema v1

The contract for all warnings:

```typescript
interface Warning {
  id: string; // 'AP-001', 'ARCH-001'
  category: 'anti-pattern' | 'boundary' | 'architecture';
  severity: 'error' | 'warning' | 'info';
  confidence: 'high' | 'medium' | 'low';

  // Display
  title: string; // "Broad eslint-disable added"
  message: string; // What happened
  explanation: string; // Why it matters
  suggestion: string; // What to do instead

  // Location
  location: {
    file: string;
    line: number;
    column?: number;
    endLine?: number;
    endColumn?: number;
  };

  // Context
  pattern?: string; // Named pattern/rule
  drift?: {
    isNew: boolean;
    existingCount?: number;
  };

  // Suppression
  suppressed?: {
    reason: string;
    author?: string;
    timestamp?: string;
  };
}
```

#### 0.2 Narrative Reset

- Rewrite README with trust broker narrative
- Update CLAUDE.md/AGENTS.md
- De-emphasise plans/APS in primary docs

---

### Phase 1: `anvil init` v2

Extend current init with architecture exploration.

#### 1.1 Dependency Graph Building

- Use existing dependency-cruiser integration
- Build full module graph on init
- Store in `.anvil/architecture.json`

#### 1.2 Entry Point Detection

Heuristic detection for TS/JS/Node:

| Pattern                              | Entry Point Type  |
| ------------------------------------ | ----------------- |
| `src/index.ts`, `index.ts`           | Package entry     |
| `src/main.ts`, `main.ts`             | Application entry |
| `**/routes/**`, `**/controllers/**`  | HTTP handlers     |
| `**/handlers/**`, `**/api/**`        | API handlers      |
| `**/jobs/**`, `**/workers/**`        | Background jobs   |
| `**/commands/**`                     | CLI commands      |
| Files with `export default` function | Potential entry   |

#### 1.3 Layer Heuristic Detection

Common patterns to detect:

| Directory Pattern                           | Suggested Layer | Priority |
| ------------------------------------------- | --------------- | -------- |
| `controllers/`, `routes/`, `api/`           | presentation    | 1        |
| `services/`, `use-cases/`, `application/`   | application     | 2        |
| `domain/`, `entities/`, `models/`           | domain          | 3        |
| `repositories/`, `data/`, `infrastructure/` | infrastructure  | 4        |
| `utils/`, `lib/`, `common/`, `shared/`      | shared          | 5        |

**Pattern Overlap Resolution**: When a file matches multiple layer patterns
(e.g., `src/services/domain/user.ts` matches both `services/` and `domain/`),
the layer with the **lowest priority number wins** (most specific first). Files
are assigned to exactly one layer. Ambiguous matches are flagged during
`anvil init` for user confirmation.

#### 1.4 Interactive Architecture Flow

```text
$ anvil init

🔨 Initialising Anvil...

Detected environment:
  TypeScript: ✓
  Package Manager: pnpm

Analysing project structure...
  Found 127 modules
  Detected 4 entry points:
    • src/index.ts (package entry)
    • src/cli/index.ts (CLI entry)
    • src/api/server.ts (HTTP handler)

Detected layer structure:
  ┌─────────────────────────────────────┐
  │ presentation (routes/, controllers/) │
  ├─────────────────────────────────────┤
  │ application (services/)              │
  ├─────────────────────────────────────┤
  │ domain (entities/, models/)          │
  ├─────────────────────────────────────┤
  │ infrastructure (repositories/)       │
  └─────────────────────────────────────┘

? Does this look correct? (Y/n/customize)

✓ Architecture baseline saved to .anvil/architecture.json
```

#### 1.5 Baseline Storage Format

`.anvil/architecture.json` (human-editable, committed to git):

```json
{
  "schema_version": "0.1.0",
  "created_at": "2025-01-15T10:30:00Z",
  "updated_at": "2025-01-15T10:30:00Z",

  "entry_points": [
    { "path": "src/index.ts", "type": "package", "confidence": "high" },
    { "path": "src/api/server.ts", "type": "http", "confidence": "high" }
  ],

  "layers": {
    "presentation": {
      "patterns": ["src/routes/**", "src/controllers/**"],
      "depends_on": ["application", "shared"]
    },
    "application": {
      "patterns": ["src/services/**"],
      "depends_on": ["domain", "infrastructure", "shared"]
    },
    "domain": {
      "patterns": ["src/entities/**", "src/models/**"],
      "depends_on": ["shared"]
    },
    "infrastructure": {
      "patterns": ["src/repositories/**"],
      "depends_on": ["domain", "shared"]
    },
    "shared": {
      "patterns": ["src/utils/**", "src/lib/**"],
      "depends_on": []
    }
  },

  "boundaries": [
    {
      "name": "no-presentation-to-infrastructure",
      "from": "presentation",
      "to": "infrastructure",
      "severity": "error",
      "message": "Presentation layer must not directly access infrastructure"
    }
  ],

  "baseline_snapshot": {
    "module_count": 127,
    "timestamp": "2025-01-15T10:30:00Z",
    "violations": [
      {
        "id": "v-001",
        "from_layer": "presentation",
        "to_layer": "infrastructure",
        "from_file": "src/controllers/legacy.ts",
        "to_file": "src/repositories/user-repo.ts",
        "import_line": 5
      },
      {
        "id": "v-002",
        "from_layer": "presentation",
        "to_layer": "infrastructure",
        "from_file": "src/routes/admin.ts",
        "to_file": "src/repositories/config-repo.ts",
        "import_line": 12
      }
    ]
  }
}
```

**Note**: The `violations` array stores normalised edge fingerprints
(layer/from/to/file) to enable reliable NEW vs existing classification. Each
violation has a stable `id` computed from
`hash(from_file + to_file + import_line)`.

````

---

### Phase 2: Anti-Pattern Library

#### 2.1 Library Structure

```text
core/src/antipattern/
├── index.ts
├── types.ts              # AntiPattern, Finding interfaces
├── registry.ts           # Pattern registry
├── detector.ts           # Detection engine
├── suppression.ts        # Suppression parsing
└── patterns/
    ├── index.ts
    ├── escape-hatches.ts # AI escape patterns
    ├── type-safety.ts    # TypeScript anti-patterns
    ├── error-handling.ts # Error handling anti-patterns
    └── code-quality.ts   # General quality patterns
````

#### 2.2 Anti-Pattern Catalogue v1

##### AI Escape Hatches (High Priority)

| ID     | Name                           | Detection                                             | Severity |
| ------ | ------------------------------ | ----------------------------------------------------- | -------- |
| AP-001 | Broad eslint-disable           | Regex: `eslint-disable(?!-next-line)` without rule    | warning  |
| AP-002 | ts-ignore/ts-nocheck           | Regex: `@ts-ignore\|@ts-nocheck`                      | warning  |
| AP-003 | ts-expect-error without reason | Regex: `@ts-expect-error` not followed by explanation | warning  |
| AP-004 | New `any` type                 | AST: `any` keyword in type position                   | warning  |
| AP-005 | Type assertion to any          | AST: `as any`, `<any>`                                | warning  |
| AP-006 | Non-null assertion overuse     | AST: `!` postfix operator (threshold: >3 per file)    | info     |
| AP-007 | Suppressed any usage           | Regex: `as any.*eslint-disable` on same/adjacent line | error    |

**AP-007 Suppressed Any Usage** is a "double escape hatch" - using `as any` AND
suppressing the lint rule that would catch it. This is particularly dangerous
because:

1. The code bypasses type safety (`as any`)
2. The lint rule that would flag this is disabled
3. Explanatory comments make it "sound legit" but don't fix the underlying issue

Detection: Look for patterns like:

- `as any); // eslint-disable-line @typescript-eslint/no-explicit-any`
- `as any; // eslint-disable-next-line` on the preceding line

This pattern is elevated to `error` severity because it represents intentional
circumvention of multiple safety layers.

##### Error Handling

| ID     | Name                 | Detection                               | Severity |
| ------ | -------------------- | --------------------------------------- | -------- |
| AP-010 | Empty catch block    | AST: catch with empty/console-only body | warning  |
| AP-011 | Catch and ignore     | AST: catch that doesn't rethrow or log  | warning  |
| AP-012 | Catch any error type | AST: catch without specific error type  | info     |

##### Code Quality

| ID     | Name                     | Detection                                          | Severity |
| ------ | ------------------------ | -------------------------------------------------- | -------- |
| AP-020 | TODO as implementation   | Regex: `TODO:?\s*(fix\|implement\|add)` in fn body | warning  |
| AP-021 | FIXME without ticket     | Regex: `FIXME` not followed by ticket reference    | info     |
| AP-022 | Console.log in prod code | AST: `console.log` outside test files              | info     |
| AP-023 | Debugger statement       | AST: `debugger` keyword                            | error    |
| AP-024 | Magic numbers            | AST: numeric literals outside const (opt-in)       | info     |

##### Type Safety

| ID     | Name                     | Detection                                  | Severity |
| ------ | ------------------------ | ------------------------------------------ | -------- |
| AP-030 | Implicit any return      | AST: function without return type (opt-in) | info     |
| AP-031 | Object/unknown as escape | AST: `as unknown`, `as object` patterns    | warning  |

#### 2.3 Pattern Definition Format

```typescript
interface AntiPattern {
  id: string;
  name: string;
  category: 'escape-hatch' | 'error-handling' | 'code-quality' | 'type-safety';
  severity: 'error' | 'warning' | 'info';
  confidence: 'high' | 'medium' | 'low';

  // Detection
  detection: {
    type: 'regex' | 'ast';
    pattern?: RegExp; // For regex detection
    astQuery?: string; // For AST detection (ts-morph query)
  };

  // Messaging
  title: string;
  explanation: string;
  suggestion: string;

  // Customisation
  allowlist?: RegExp[]; // File patterns to skip
  configurable?: {
    threshold?: number; // For count-based patterns (e.g., AP-006: >3 per file)
  };

  // Enablement
  enabled: boolean; // Default enabled state
  optIn?: boolean; // If true, disabled by default (noisy patterns)

  // Docs
  documentation?: string; // Link to detailed docs
}
```

**Noise Control**: Patterns marked `optIn: true` (AP-024 Magic numbers, AP-030
Implicit any return, AP-006 Non-null assertions) are disabled by default to meet
the < 5% false positive target. Users can enable them in `.anvilrc`:

```json
{
  "patterns": {
    "AP-024": { "enabled": true },
    "AP-030": { "enabled": true, "threshold": 5 }
  }
}
```

#### 2.4 Detection Engine

Use a combination of:

- **Regex**: For comment-based patterns (eslint-disable, ts-ignore, TODO)
- **ts-morph**: For AST-based patterns (any, catch blocks, type assertions)

---

### Phase 3: Boundary Detection

#### 3.1 New `anvil check` Command

```bash
# Check specific files
anvil check src/services/payment.ts

# Check all changed files (git diff)
anvil check --changed

# Check with baseline comparison
anvil check --changed --baseline
```

#### 3.2 Boundary Violation Detection

1. Load `.anvil/architecture.json`
2. Parse imports in target file(s)
3. Determine which layer each import comes from
4. Check against `layers[].depends_on` rules
5. Compare against `baseline_snapshot` to identify NEW violations

#### 3.3 Output Format

```text
⚠️  Boundary violation (NEW)

   presentation → infrastructure

   src/controllers/payment.ts:15
   └─ imports from src/repositories/user-repo.ts

   Why: Presentation layer should not directly access infrastructure.
        This bypasses the application layer and creates tight coupling.

   Suggestion: Inject this dependency through a service, or move
               this logic to the application layer.

   ℹ️  This boundary is already violated in 2 other places.

   Actions:
   • View boundary map: anvil boundaries --show
   • Suppress: Add /** @anvil-ignore ARCH-001 reason */
```

---

### Phase 4: On-Save Analysis

#### 4.1 Extend Watch Mode

```bash
# Current (watches plans)
anvil watch

# New (watches source files)
anvil watch --source
anvil watch --source --patterns "src/**/*.ts"
```

#### 4.2 Watch Configuration

In `.anvilrc`:

```json
{
  "watch": {
    "source": {
      "enabled": true,
      "patterns": ["src/**/*.ts", "src/**/*.tsx"],
      "ignore": ["**/*.test.ts", "**/__tests__/**"],
      "debounce": 300
    }
  }
}
```

#### 4.3 Incremental Analysis

- On file save, only re-analyse that file for anti-patterns (file-local)
- Use cached dependency graph (invalidate on import changes)
- **Reverse-dependency invalidation**: When a file's imports change, also
  re-check files that import the changed file (one level deep) to catch boundary
  regressions
- Cache invalidation strategy:
  - Anti-patterns: No caching needed (fast regex/AST per file)
  - Boundary checks: Cache import graph, invalidate on `import`/`require` line
    changes
  - Layer assignment: Cache per-file, invalidate on file move/rename

#### 4.4 Performance Strategy

To meet the < 2 second target for `anvil check --changed`:

| Component        | Strategy                                                                           |
| ---------------- | ---------------------------------------------------------------------------------- |
| ts-morph AST     | Reuse `Project` instance across files; use `getSourceFile()` not `addSourceFile()` |
| Dependency graph | Cache in memory during watch session; persist to `.anvil/cache/graph.json`         |
| Layer resolution | Pre-compute layer assignments on init; cache in `.anvil/architecture.json`         |
| Regex patterns   | Compile once at startup; reuse across files                                        |

**Benchmark gates**: If `anvil check --changed` exceeds 2s on a 10-file
changeset, fall back to regex-only detection and log a performance warning.

---

### Phase 5: Suppression System

#### 5.1 Suppression Format

```typescript
/** @anvil-ignore AP-001 Legacy code, will refactor in Q2 - @jane */
/* eslint-disable */

/** @anvil-ignore ARCH-001 Temporary workaround for #1234 */
import { UserRepo } from '../repositories/user-repo';
```

#### 5.2 Parsing Rules

- Must be JSDoc comment (`/** */`)
- Pattern: `@anvil-ignore <ID> <reason>`
- Reason is required (warn if missing)
- Optional author tag at end (`- @username`)

**Suppression Scope**:

| Placement            | Scope               | Example                                                |
| -------------------- | ------------------- | ------------------------------------------------------ |
| Above statement      | Next statement only | `/** @anvil-ignore AP-001 reason */ const x = ...`     |
| Above import         | That import only    | `/** @anvil-ignore ARCH-001 reason */ import {...}`    |
| Top of file          | Entire file         | `/** @anvil-ignore-file AP-001 reason */` at line 1-5  |
| Inline (end of line) | That line only      | `const x: any = y; /** @anvil-ignore AP-004 reason */` |

For AST-based patterns, suppression applies to the **AST node** following the
comment. For regex-based patterns, suppression applies to the **next non-empty
line**.

#### 5.3 Suppression Storage

Added to provenance records:

```typescript
interface SuppressionRecord {
  id: string;
  pattern_id: string;
  file: string;
  line: number;
  reason: string;
  author?: string;
  timestamp: string;
  commit?: string;
}
```

#### 5.4 Commands

```bash
# List all suppressions
anvil suppressions list

# Show suppressions for a pattern
anvil suppressions list --pattern AP-001

# Show suppressions without proper reasons
anvil suppressions audit
```

---

## CLI Commands Summary (v0.1.0)

```bash
# Initialisation (enhanced)
anvil init                    # Explore architecture, create baseline
anvil init --no-interactive   # Use heuristics, no prompts

# Core analysis loop
anvil check [files...]        # Check files for anti-patterns + boundaries
anvil check --changed         # Check git-changed files
anvil watch --source          # Watch source files for real-time feedback

# Architecture inspection
anvil boundaries              # Show architecture map
anvil boundaries --violations # Show current violations
anvil boundaries --refresh    # Rebuild dependency graph

# Anti-pattern management
anvil patterns                # List enabled anti-patterns
anvil patterns --disable AP-024  # Disable a pattern

# Suppression management
anvil suppressions list       # List all suppressions
anvil suppressions audit      # Find suppressions without reasons

# Existing (de-emphasised in docs)
anvil validate <plan>         # Schema validation
anvil gate <plan>             # Full gate run
anvil export <plan>           # Format conversion
```

---

## Files to Create/Modify

### New Files

| Path                                        | Purpose                                |
| ------------------------------------------- | -------------------------------------- |
| `core/src/antipattern/types.ts`             | Warning, AntiPattern, Finding types    |
| `core/src/antipattern/registry.ts`          | Pattern registry                       |
| `core/src/antipattern/detector.ts`          | Detection engine                       |
| `core/src/antipattern/suppression.ts`       | Suppression parsing                    |
| `core/src/antipattern/patterns/*.ts`        | Pattern definitions                    |
| `core/src/architecture/types.ts`            | Architecture baseline types            |
| `core/src/architecture/analyzer.ts`         | Layer detection, entry point detection |
| `core/src/architecture/baseline.ts`         | Baseline storage/loading               |
| `core/src/architecture/boundary-checker.ts` | Violation detection                    |
| `cli/src/commands/check.ts`                 | New check command                      |
| `cli/src/commands/boundaries.ts`            | Architecture inspection                |
| `cli/src/commands/patterns.ts`              | Pattern management                     |
| `cli/src/commands/suppressions.ts`          | Suppression management                 |

### Modified Files

| Path                           | Changes                           |
| ------------------------------ | --------------------------------- |
| `cli/src/commands/init.ts`     | Add architecture exploration flow |
| `cli/src/commands/watch.ts`    | Add `--source` mode               |
| `core/src/provenance/types.ts` | Add suppression records           |
| `README.md`                    | New narrative                     |
| `CLAUDE.md`                    | Updated guidance                  |
| `AGENTS.md`                    | Updated guidance                  |

---

## Dependencies

| Package    | Purpose                                 |
| ---------- | --------------------------------------- |
| `ts-morph` | AST analysis for anti-pattern detection |

Note: `dependency-cruiser` is already an optional peer dependency.

---

## Explicit Exclusions (NOT in v0.1.0)

| Feature                  | Reason                |
| ------------------------ | --------------------- |
| VS Code extension        | CLI-first             |
| PR/CI comments           | Nice-to-have for v0.2 |
| Drift reports/dashboards | Phase 2               |
| Auto-fix                 | Phase 2               |
| Plans/APS as requirement | Planless-first        |
| OPA policy authoring UI  | Internal only         |
| LLM-assisted inference   | Future enhancement    |

---

## Success Criteria

v0.1.0 is successful if:

1. `anvil init` produces a usable architecture baseline in < 30 seconds
2. `anvil check --changed` runs in < 2 seconds for typical changes
3. Anti-pattern detection has < 5% false positive rate
4. Boundary detection correctly identifies NEW vs existing violations
5. Experienced engineers find immediate value without reading docs

---

## Related Documents

- [PRODUCT_NARRATIVE.md](./feature-alignment/PRODUCT_NARRATIVE.md) - Core
  messaging
- [V1_SCOPE.md](./feature-alignment/V1_SCOPE.md) - Scope definition
- [FEATURE_RATIONALISATION.md](./feature-alignment/FEATURE_RATIONALISATION.md) -
  Keep/Reshape/Defer
- [ANTI_PATTERNS_LIBRARY.md](./feature-alignment/ANTI_PATTERNS_LIBRARY.md) -
  Pattern catalogue seed
- [UX_LOCAL_WARNINGS.md](./feature-alignment/UX_LOCAL_WARNINGS.md) - Warning UX
  design

---

## Review Findings Addressed

This plan was reviewed by multiple agents and Codex. Key findings addressed:

### High Severity (Fixed)

| Finding                                                                           | Resolution                                                                        |
| --------------------------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| Baseline filename inconsistent (`.anvil/arch.json` vs `.anvil/architecture.json`) | Standardised to `.anvil/architecture.json` throughout                             |
| NEW vs existing violations can't be computed with count-only baseline             | Added `violations[]` array with normalised edge fingerprints (layer/from/to/file) |

### Medium Severity (Fixed)

| Finding                                                | Resolution                                                                     |
| ------------------------------------------------------ | ------------------------------------------------------------------------------ |
| On-save misses regressions in dependent files          | Added reverse-dependency invalidation (one level deep) in Phase 4.3            |
| 2s performance target at risk with ts-morph            | Added Phase 4.4 Performance Strategy with caching, project reuse, and fallback |
| Noisy patterns conflict with <5% false positive target | Marked AP-006, AP-024, AP-030 as `optIn: true` (disabled by default)           |
| Layer pattern overlaps unresolved                      | Added priority-based resolution with user confirmation for ambiguous matches   |
| Suppression scope undefined                            | Added explicit scope table (statement, import, file, inline)                   |

### From Agent Reviews (Incorporated)

| Finding                                  | Resolution                                                   |
| ---------------------------------------- | ------------------------------------------------------------ |
| Warning schema conflicts with GateResult | Noted for Phase 0 implementation - will create adapter       |
| ts-morph should be optional peer dep     | Noted for implementation - follow dependency-cruiser pattern |
| Suppression parsing needed earlier       | Noted - will move to Phase 0.4 during execution              |
| Missing test tasks                       | Noted - will add during execution                            |

---

_Last updated: December 2025_
