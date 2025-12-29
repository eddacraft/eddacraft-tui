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

### Policy Engine

| Feature                  | Difficulty | Value  | Status      |
| ------------------------ | ---------- | ------ | ----------- |
| OPA/Rego integration     | High       | High   | Not started |
| Policy bundle structure  | Medium     | Medium | Not started |
| Built-in policy library  | Medium     | High   | Not started |
| Custom policy authoring  | Medium     | Medium | Not started |
| Policy testing framework | Medium     | Medium | Not started |

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

### v1.0.0 — Save-time Trust (Current Target)

**Status:** Core engine complete, onboarding TUI in progress.

Completed:

- ✅ Core analysis engine (`anvil check`)
- ✅ Architecture boundary detection with baseline
- ✅ Anti-pattern detection (7 patterns)
- ✅ Suppression system with time-boxing
- ✅ Git integration (`--changed`, `--staged`, `--since`)
- ✅ Watch mode (`anvil watch --source`)
- ✅ CI/CD integration (GitHub Action)
- ✅ Git hooks integration
- ✅ Gate profiles and skip flags
- ✅ `.anvilrc` configuration
- ✅ Validation caching + parallel execution

In Progress:

- 🔄 Onboarding TUI (Ink-based: init wizard, status, doctor)
- 🔄 Documentation polish

### v1.1.0 — Drift Visibility

- Drift snapshots and comparison
- Trend visualisation

### v1.2.0 — Advanced Experience

- VS Code extension
- Operational TUI (watch dashboard, gate explorer)
- Template library

### v2.0.0 — AI Tool Integration

- Command safety validation (CMDSAF)
- MCP server for real-time validation
- Constraint export for AI tools

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
