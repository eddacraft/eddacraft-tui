# Anvil Product Roadmap

## Vision

Anvil is the forge where human and AI intent becomes deterministic, auditable
plans and production-ready features — all inside GitHub, VS Code, and the CLI.

**Core Principles**:

- **Developer-first**: Meet developers where they are (IDE, terminal, CI)
- **Trust by design**: Provenance, validation, rollback-ready
- **Interop**: Work with existing formats (SpecKit, BMAD, markdown)
- **Speed**: Sub-second validation or developers disable it

---

## Roadmap by Horizon

Features rated by **Difficulty** (Low/Medium/High) and **Value**
(Low/Medium/High/Critical).

---

## Horizon 1: Developer Experience Foundation

**Theme**: Remove friction, meet developers where they are.

### Git Integration

| Feature                             | Difficulty | Value  | Status      |
| ----------------------------------- | ---------- | ------ | ----------- |
| Pre-commit hook (`anvil validate`)  | Low        | High   | Not started |
| Pre-push hook (`anvil gate`)        | Low        | High   | Not started |
| `anvil hooks install` command       | Low        | High   | Not started |
| Husky/lint-staged integration guide | Low        | Medium | Not started |

**Notes**: Husky already configured. Need Anvil-specific hooks.

### Developer Mode & Configuration

| Feature                               | Difficulty | Value  | Status      |
| ------------------------------------- | ---------- | ------ | ----------- |
| `--skip-gates` flag for development   | Low        | High   | Not started |
| `--only=lint,test` for specific gates | Low        | Medium | Not started |
| Gate profiles (dev/ci/production)     | Medium     | High   | Not started |
| Full `.anvilrc` configuration         | Medium     | High   | Partial     |

### Watch Mode

| Feature                                     | Difficulty | Value  | Status      |
| ------------------------------------------- | ---------- | ------ | ----------- |
| `anvil watch` daemon mode                   | Medium     | High   | Not started |
| Incremental validation (changed files only) | Medium     | High   | Not started |
| File system watcher                         | Low        | Medium | Not started |

---

## Horizon 2: Speed & Caching

**Theme**: Sub-second validation or developers disable it.

### Performance

| Feature                           | Difficulty | Value    | Status      |
| --------------------------------- | ---------- | -------- | ----------- |
| Validation result caching         | Medium     | Critical | Not started |
| Hash-based cache invalidation     | Low        | High     | Not started |
| Parallel gate execution           | Medium     | High     | Not started |
| Persistent daemon for warm starts | High       | Medium   | Not started |

### Incremental Processing

| Feature                 | Difficulty | Value  | Status      |
| ----------------------- | ---------- | ------ | ----------- |
| Diff-based validation   | High       | High   | Not started |
| Cached dependency graph | Medium     | Medium | Not started |

---

## Horizon 3: CI/CD Integration

**Theme**: Gate in the pipeline, block bad merges.

### GitHub Integration

| Feature                           | Difficulty | Value    | Status      |
| --------------------------------- | ---------- | -------- | ----------- |
| GitHub Action for gate            | Medium     | Critical | Not started |
| PR status checks (pass/fail)      | Medium     | High     | Not started |
| Inline PR comments on failures    | Medium     | High     | Not started |
| PR summary comment                | Low        | High     | Not started |
| Block merge on gate failure       | Low        | Critical | Not started |
| `/anvil override` comment command | Medium     | Medium   | Not started |

### GitLab Integration

| Feature            | Difficulty | Value  | Status      |
| ------------------ | ---------- | ------ | ----------- |
| GitLab CI template | Medium     | Medium | Not started |
| MR integration     | Medium     | Medium | Not started |

---

## Horizon 4: IDE Integration

**Theme**: Never leave your editor.

### VS Code Extension

| Feature                   | Difficulty | Value    | Status      |
| ------------------------- | ---------- | -------- | ----------- |
| Extension scaffold        | Medium     | Critical | Not started |
| Inline validation status  | Medium     | High     | Not started |
| Gate results panel        | Medium     | High     | Not started |
| One-click gate run        | Low        | High     | Not started |
| Problem panel integration | Medium     | High     | Not started |
| CodeLens for plan files   | Medium     | Medium   | Not started |
| Status bar indicator      | Low        | Medium   | Not started |

### JetBrains Plugin

| Feature                  | Difficulty | Value  | Status      |
| ------------------------ | ---------- | ------ | ----------- |
| IntelliJ/WebStorm plugin | High       | Medium | Not started |

---

## Horizon 5: AI Tool Integration

**Theme**: Intercept before accept, not validate after.

### MCP Server

| Feature                                | Difficulty | Value    | Status      |
| -------------------------------------- | ---------- | -------- | ----------- |
| MCP server implementation              | Medium     | Critical | Not started |
| Validation as MCP tool                 | Medium     | High     | Not started |
| Gate results as MCP resource           | Medium     | High     | Not started |
| Real-time validation during generation | High       | Critical | Not started |

### AI Tool Hooks

| Feature               | Difficulty | Value    | Status      |
| --------------------- | ---------- | -------- | ----------- |
| Claude Code hooks     | Medium     | High     | Not started |
| Cursor integration    | High       | Critical | Not started |
| Pre-accept validation | High       | High     | Not started |

---

## Horizon 6: Actionable Feedback

**Theme**: Don't just fail — fix.

### Auto-fix Suggestions

| Feature                            | Difficulty | Value  | Status      |
| ---------------------------------- | ---------- | ------ | ----------- |
| Coverage gap analysis + test stubs | High       | High   | Not started |
| Lint auto-fix integration          | Low        | Medium | Not started |
| Dependency upgrade suggestions     | Medium     | High   | Partial     |
| Security fix recommendations       | Medium     | High   | Not started |

### Rich Errors

| Feature                    | Difficulty | Value  | Status      |
| -------------------------- | ---------- | ------ | ----------- |
| Contextual fix suggestions | Medium     | High   | Not started |
| Documentation links        | Low        | Medium | Not started |
| One-command fix option     | Medium     | High   | Not started |

---

## Horizon 7: Security & Architecture Gates

**Theme**: Catch what linters miss.

### Security Gates

| Feature                            | Difficulty | Value    | Status      |
| ---------------------------------- | ---------- | -------- | ----------- |
| Dependency vulnerability scanning  | Medium     | Critical | In progress |
| Enhanced secret scanning (entropy) | Medium     | High     | Not started |
| Git history secret scanning        | Medium     | High     | Not started |
| SAST integration (Semgrep)         | High       | High     | Not started |
| IaC scanning (Dockerfile, K8s)     | High       | Medium   | Not started |
| License compliance                 | Medium     | Medium   | Not started |

### Architecture Gates

| Feature                       | Difficulty | Value  | Status      |
| ----------------------------- | ---------- | ------ | ----------- |
| Circular dependency detection | Medium     | High   | Not started |
| Layer boundary validation     | High       | High   | Not started |
| Anti-pattern detection        | High       | High   | Not started |
| Custom architecture rules     | High       | Medium | Not started |

---

## Horizon 8: Visual & Reporting

**Theme**: Make validation beautiful.

### Visual Preview

| Feature                        | Difficulty | Value  | Status      |
| ------------------------------ | ---------- | ------ | ----------- |
| Interactive HTML diff report   | High       | High   | Not started |
| Blast radius visualisation     | High       | High   | Not started |
| Dependency graph view          | Medium     | Medium | Not started |
| `anvil preview --html` command | Medium     | High   | Not started |

### Metrics Dashboard

| Feature                     | Difficulty | Value  | Status      |
| --------------------------- | ---------- | ------ | ----------- |
| Validation success tracking | Medium     | Medium | Not started |
| Common failure patterns     | Medium     | Medium | Not started |
| Team metrics                | High       | Medium | Not started |

---

## Horizon 9: Execution & Safety

**Theme**: Apply with confidence, rollback without fear.

### Safe Execution

| Feature                      | Difficulty | Value    | Status      |
| ---------------------------- | ---------- | -------- | ----------- |
| `anvil apply` with snapshots | High       | Critical | Not started |
| Transactional file changes   | High       | High     | Not started |
| Dry-run mode                 | Medium     | High     | Not started |
| Pre-flight checks            | Medium     | High     | Not started |

### Rollback

| Feature                    | Difficulty | Value    | Status      |
| -------------------------- | ---------- | -------- | ----------- |
| `anvil rollback <plan-id>` | High       | Critical | Not started |
| Partial rollback           | High       | Medium   | Not started |
| Rollback verification      | Medium     | High     | Not started |
| Rollback audit trail       | Medium     | High     | Not started |

---

## Horizon 10: Policy & Governance

**Theme**: Codify your standards.

### Policy Engine (Core OPA Integration) ✅

| Feature                  | Difficulty | Value  | Status      |
| ------------------------ | ---------- | ------ | ----------- |
| OPA/Rego integration     | High       | High   | ✅ Complete |
| OPA binary management    | Medium     | High   | ✅ Complete |
| Policy loader            | Medium     | High   | ✅ Complete |
| Policy CLI commands      | Medium     | High   | ✅ Complete |
| Example policies         | Low        | High   | ✅ Complete |
| Policy testing framework | Medium     | Medium | ✅ Complete |

### Policy Engine Enhancements (Recent)

| Feature                   | Difficulty | Value  | Status      |
| ------------------------- | ---------- | ------ | ----------- |
| Violation categories      | Low        | High   | ✅ Complete |
| Violation fingerprints    | Low        | High   | ✅ Complete |
| Documentation URL support | Low        | Medium | ✅ Complete |
| Git context in OPA input  | Medium     | High   | ✅ Complete |
| CI context detection      | Medium     | High   | ✅ Complete |
| Policy test enforcement   | Low        | High   | ✅ Complete |
| PolicyCheck unit tests    | Medium     | High   | ✅ Complete |

### OPA Review & Alert System (Planned)

| Feature                           | Difficulty | Value    | Status      |
| --------------------------------- | ---------- | -------- | ----------- |
| Enhanced alert types              | Medium     | High     | Not started |
| Alert location with line numbers  | Medium     | High     | Not started |
| Blast radius analysis             | High       | High     | Not started |
| Suggested fixes in violations     | Medium     | Critical | Not started |
| Review workflow state machine     | High       | High     | Not started |
| Alert acknowledgement CLI         | Medium     | Medium   | Not started |
| Alert persistence/history         | High       | High     | Not started |
| Alert trend analysis              | Medium     | Medium   | Not started |
| Alert grouping by category/policy | Low        | High     | Not started |
| CI annotation formatters (GitHub) | Medium     | Critical | Not started |
| SARIF output format               | Medium     | High     | Not started |
| Coverage data in OPA input        | Low        | Medium   | Not started |
| Dependency data in OPA input      | Medium     | Medium   | Not started |
| Architecture context in OPA input | High       | High     | Not started |
| Remote policy bundles             | High       | Medium   | Not started |

**Enhanced Alert Type System** (Planned):

```typescript
interface EnhancedAlert {
  id: string; // Deterministic hash
  fingerprint: string; // Stable across runs
  rule: string;
  policy: string;
  severity: 'critical' | 'error' | 'warning' | 'info';
  category: ViolationCategory;
  message: string;
  description?: string;
  documentation_url?: string;
  locations: AlertLocation[];
  blast_radius?: BlastRadius;
  fixable: boolean;
  suggested_fix?: SuggestedFix;
  required_actions?: RequiredAction[];
  review_state?: ReviewState;
}
```

**Review Workflow States** (Planned):

```
open → acknowledged → in_progress → resolved
  ↓         ↓              ↓
wont_fix  wont_fix    auto_resolved
```

**CLI Commands** (Planned):

```bash
anvil alerts list                     # View all alerts
anvil alerts list --state open        # Filter by state
anvil alerts acknowledge <id>         # Mark as acknowledged
anvil alerts resolve <id>             # Mark as resolved
anvil alerts wont-fix <id> --reason   # Accept risk
anvil alerts history                  # View alert trends
```

---

## Horizon 11: Architecture Validation

**Theme**: Enforce boundaries, prevent drift.

### Architecture Definition

| Feature                  | Difficulty | Value  | Status      |
| ------------------------ | ---------- | ------ | ----------- |
| Architecture YAML schema | Medium     | High   | Not started |
| Layer definition system  | Medium     | High   | Not started |
| Dependency rules         | Medium     | High   | Not started |
| Architecture templates   | Medium     | Medium | Not started |
| Layered architecture     | Low        | High   | Not started |
| Hexagonal architecture   | Medium     | Medium | Not started |
| Clean architecture       | Medium     | Medium | Not started |
| DDD bounded contexts     | High       | Medium | Not started |

### Architecture Validation

| Feature                        | Difficulty | Value  | Status      |
| ------------------------------ | ---------- | ------ | ----------- |
| Dependency-cruiser integration | High       | High   | Not started |
| Layer boundary validation      | High       | High   | Not started |
| Circular dependency detection  | Medium     | High   | Not started |
| Auto-generated Rego policies   | High       | High   | Not started |
| Architecture CLI commands      | Medium     | Medium | Not started |
| Architecture visualisation     | High       | Medium | Not started |

---

## Priority Matrix

### Do First (High Value + Low/Medium Difficulty)

1. **Pre-commit/pre-push hooks** — Low difficulty, High value
2. **`--skip-gates` flag** — Low difficulty, High value
3. **`.anvilrc` configuration** — Medium difficulty, High value
4. **Validation caching** — Medium difficulty, Critical value
5. **GitHub Action** — Medium difficulty, Critical value

### Do Next (High Value + Higher Difficulty)

1. **VS Code extension** — Medium difficulty, Critical value
2. **MCP server** — Medium difficulty, Critical value
3. **Watch mode** — Medium difficulty, High value
4. **Apply/Rollback** — High difficulty, Critical value

### Strategic Differentiators

1. **AI tool intercept** — No competitor does this
2. **Architecture gates** — Unique positioning
3. **Visual blast radius** — Wow factor for demos
4. **Auto-fix suggestions** — Zero-friction adoption

---

## Version Milestones

### v0.1.0 — Core Foundation ✅

- APS schema and validation
- Format adapters (SpecKit, BMAD, Generic)
- Basic gate checks (ESLint, coverage, secrets, dependencies)
- CLI commands (validate, gate, export, init)

### v0.2.0 — Policy Engine ✅

- OPA/Rego integration
- Policy CLI commands
- Example policies with tests
- Git/CI context in OPA input
- Violation categories and fingerprints

### v0.3.0 — Developer Ergonomics

- Git hooks integration
- Skip/only gate flags
- `.anvilrc` configuration
- Basic watch mode

### v0.4.0 — Speed

- Validation caching
- Parallel gate execution
- Incremental validation

### v0.5.0 — CI/CD

- GitHub Action
- PR status checks
- CI annotation formatters
- SARIF output support

### v0.6.0 — Alert & Review System

- Enhanced alert types
- Review workflow state machine
- Alert persistence and history
- Alert CLI commands
- Trend analysis

### v0.7.0 — IDE

- VS Code extension (basic)
- Real-time validation
- Problem panel

### v0.8.0 — Architecture Validation

- Architecture YAML definition
- Layer boundary validation
- Dependency-cruiser integration
- Auto-generated Rego policies

### v0.9.0 — Execution

- Apply with snapshots
- Rollback capability
- Full audit trail

### v1.0.0 — Production Ready

- Complete feature set
- Performance optimised
- Enterprise features
- Comprehensive docs

---

## Success Metrics

| Metric                      | Target          |
| --------------------------- | --------------- |
| Validation latency (cached) | < 2 seconds     |
| Validation latency (cold)   | < 10 seconds    |
| Daily active usage          | 80% of team     |
| Gate bypass rate            | < 5% of commits |
| CI integration              | 100% of repos   |
| Rollback success            | 100%            |

---

## Risks & Mitigations

| Risk                   | Mitigation                                |
| ---------------------- | ----------------------------------------- |
| Over-engineering early | Agentic-Lite; defer sidecar               |
| Interop drift          | Versioned adapters; round-trip tests      |
| Gate too slow          | Changed-files scope, parallelism, caching |
| Adoption friction      | VS Code + GitHub first; quick wins        |
| Developer bypass       | Dev mode (`--skip`), but enforce in CI    |

---

_Last updated: December 2025_
