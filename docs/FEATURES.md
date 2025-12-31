# Anvil Feature Matrix

A comprehensive overview of Anvil capabilities for planning and pricing
purposes.

---

## Core Platform

| Feature                   | Status | Completed                             | Planned | Details                                     |
| ------------------------- | ------ | ------------------------------------- | ------- | ------------------------------------------- |
| **APS Schema**            | ✅     | Zod v0.1.0 schema, JSON Schema export | —       | Deterministic, hash-stable internal format  |
| **Plan Validation**       | ✅     | Schema validation, error formatting   | —       | Validates structure, required fields, types |
| **Deterministic Hashing** | ✅     | SHA-256 content hashing, plan IDs     | —       | Same input → same hash, always              |
| **Plan ID Generation**    | ✅     | `aps-XXXXXXXX` format                 | —       | Unique, reproducible identifiers            |

---

## Format Adapters

| Feature                     | Status | Completed                             | Planned                       | Details                                   |
| --------------------------- | ------ | ------------------------------------- | ----------------------------- | ----------------------------------------- |
| **Adapter Framework**       | ✅     | FormatAdapter interface, registry     | —                             | Pluggable architecture for any format     |
| **Auto-Detection**          | ✅     | Confidence-based format detection     | —                             | Automatically identifies input format     |
| **SpecKit Adapter**         | ✅     | v1 + v2 support (114 tests)           | —                             | GitHub spec-kit PRD format                |
| **BMAD Adapter**            | ✅     | PRD/architecture format (86 tests)    | —                             | BMAD methodology support                  |
| **Generic Markdown**        | ✅     | Fallback adapter (32 tests)           | —                             | Any structured markdown document          |
| **Round-Trip Verification** | ✅     | Parse → serialize → parse consistency | —                             | Ensures lossless conversion               |
| **File Discovery**          | ✅     | Auto-find planning documents          | —                             | Searches for prd.md, plan.md, etc.        |
| **Custom Adapters**         | —      | —                                     | Plugin API for custom formats | Extend with organisation-specific formats |

---

## CLI Commands

| Feature            | Status | Completed                   | Planned                   | Details                                |
| ------------------ | ------ | --------------------------- | ------------------------- | -------------------------------------- |
| **anvil init**     | ✅     | Generate `.anvilrc` config  | —                         | Initialise Anvil in a project          |
| **anvil validate** | ✅     | Validate plans (any format) | —                         | Schema + hash validation               |
| **anvil gate**     | ✅     | Run quality gates           | —                         | Lint, test, coverage, secrets          |
| **anvil export**   | ✅     | Convert between formats     | —                         | APS ↔ SpecKit ↔ BMAD                   |
| **anvil watch**    | ✅     | Real-time file monitoring   | —                         | Git-aware, debounced, configurable     |
| **anvil hooks**    | ✅     | Git hooks management        | —                         | Install/uninstall pre-commit, pre-push |
| **anvil policy**   | ✅     | OPA policy management       | —                         | validate, test, list, init             |
| **anvil plan**     | —      | —                           | Generate plan from intent | AI-assisted plan creation              |
| **anvil apply**    | —      | —                           | Execute plan changes      | Transactional file operations          |
| **anvil rollback** | —      | —                           | Revert plan changes       | Snapshot-based restoration             |
| **anvil history**  | —      | —                           | Plan audit trail          | View execution history                 |
| **anvil diff**     | —      | —                           | Compare two plans         | Side-by-side differences               |
| **anvil preview**  | —      | —                           | Visual diff report        | HTML preview of changes                |

---

## Quality Gates

| Feature                    | Status | Completed                                       | Planned                    | Details                                              |
| -------------------------- | ------ | ----------------------------------------------- | -------------------------- | ---------------------------------------------------- |
| **ESLint Check**           | ✅     | Code quality scoring                            | —                          | Configurable min score threshold                     |
| **Coverage Check**         | ✅     | Line/branch/function coverage                   | —                          | Reads coverage-summary.json, configurable thresholds |
| **Secret Scanning**        | ✅     | Regex pattern detection                         | Entropy-based, git history | Detects API keys, tokens, passwords                  |
| **Dependency Check**       | ✅     | npm/pnpm/yarn audit, CVE links, fix suggestions | —                          | Severity thresholds, auto-detect package manager     |
| **Test Runner**            | —      | —                                               | Vitest/Jest integration    | Run tests as gate check                              |
| **Policy Check (OPA)**     | ✅     | Rego policy evaluation                          | —                          | Custom business rules                                |
| **SAST (Semgrep)**         | —      | —                                               | Semgrep integration        | Static security analysis                             |
| **Architecture Check**     | ✅     | Circular deps, layer violations, orphans        | Pattern templates          | dependency-cruiser integration, configurable rules   |
| **Anti-pattern Detection** | —      | —                                               | AST analysis (ts-morph)    | God classes, tight coupling                          |

---

## Gate Configuration

| Feature                   | Status | Completed                         | Planned | Details                        |
| ------------------------- | ------ | --------------------------------- | ------- | ------------------------------ |
| **Config File**           | ✅     | `.anvilrc` / `.anvil/config.json` | —       | JSON/JSONC configuration       |
| **Check Enable/Disable**  | ✅     | Per-check enabled flag            | —       | Fine-grained control           |
| **Threshold Overrides**   | ✅     | Per-check thresholds              | —       | coverage: 80%, lint: 90%, etc. |
| **Gate Profiles**         | ✅     | dev, ci, production presets       | —       | Quick environment switching    |
| **Skip Checks Flag**      | ✅     | `--skip-checks=coverage,lint`     | —       | CLI override                   |
| **Only Checks Flag**      | ✅     | `--only-checks=secret`            | —       | Run specific checks only       |
| **Environment Variables** | ✅     | `ANVIL_SKIP_GATES`                | —       | CI/CD friendly                 |
| **Fail Fast**             | ✅     | `--fail-fast` flag                | —       | Stop on first failure          |

---

## Performance & Caching

| Feature                 | Status | Completed                       | Planned | Details                   |
| ----------------------- | ------ | ------------------------------- | ------- | ------------------------- |
| **Result Caching**      | ✅     | Content hash-based caching      | —       | Skip unchanged checks     |
| **Cache Location**      | ✅     | `.anvil/cache/` directory       | —       | File-based persistence    |
| **Cache Invalidation**  | ✅     | Input hash comparison           | —       | Auto-invalidate on change |
| **No-Cache Flag**       | ✅     | `--no-cache` to bypass          | —       | Force fresh execution     |
| **Cache Statistics**    | ✅     | Hits, misses, time saved        | —       | In verbose output         |
| **Parallel Execution**  | ✅     | Concurrent gate checks          | —       | Faster CI runs            |
| **Parallelism Control** | ✅     | `--parallel=<n>` (0=sequential) | —       | Resource management       |
| **Progress Reporting**  | ✅     | `--progress` flag               | —       | Real-time check status    |

---

## Watch Mode

| Feature                 | Status | Completed                          | Planned | Details                      |
| ----------------------- | ------ | ---------------------------------- | ------- | ---------------------------- |
| **File Watching**       | ✅     | chokidar-based monitoring          | —       | Efficient file system events |
| **Git-Aware Filtering** | ✅     | Unstaged changes only              | —       | Focus on work in progress    |
| **Include Untracked**   | ✅     | `--include-untracked` option       | —       | Watch new files too          |
| **Debouncing**          | ✅     | Configurable delay (300ms default) | —       | Coalesce rapid saves         |
| **Pattern Matching**    | ✅     | Glob patterns in config            | —       | Watch specific file types    |
| **Action Selection**    | ✅     | `--action validate\|gate`          | —       | Choose what runs on change   |
| **Real-Time Output**    | ✅     | Pass/fail tracking                 | —       | Immediate feedback           |
| **Config Integration**  | ✅     | `.anvilrc` watch section           | —       | Persistent settings          |

---

## Output Formats

| Feature               | Status | Completed                      | Planned                  | Details                   |
| --------------------- | ------ | ------------------------------ | ------------------------ | ------------------------- |
| **Human Output**      | ✅     | Coloured, formatted CLI output | —                        | Developer-friendly        |
| **JSON Output**       | ✅     | `--output json` flag           | —                        | Machine-readable          |
| **Structured Schema** | ✅     | Version, timing, cache stats   | —                        | Consistent JSON structure |
| **HTML Reports**      | —      | —                              | Interactive diff preview | Visual reporting          |
| **Markdown Reports**  | —      | —                              | PR comment format        | GitHub integration        |

---

## Policy Engine (OPA)

| Feature                   | Status | Completed                            | Planned | Details                   |
| ------------------------- | ------ | ------------------------------------ | ------- | ------------------------- |
| **OPA Binary Management** | ✅     | Auto-download for OS/arch            | —       | Linux, macOS, Windows     |
| **Policy Directory**      | ✅     | `.anvil/policies/` structure         | —       | Organised policy storage  |
| **Policy Validation**     | ✅     | `anvil policy validate`              | —       | Check Rego syntax         |
| **Policy Testing**        | ✅     | `anvil policy test`                  | —       | Run policy unit tests     |
| **Policy Listing**        | ✅     | `anvil policy list`                  | —       | Show active policies      |
| **Policy Init**           | ✅     | `anvil policy init`                  | —       | Scaffold policy directory |
| **Example Policies**      | ✅     | coverage_min, change_scope, security | —       | Starting templates        |
| **Custom Policies**       | ✅     | Write any Rego policy                | —       | Full OPA flexibility      |

---

## Git Integration

| Feature                    | Status | Completed                       | Planned | Details                      |
| -------------------------- | ------ | ------------------------------- | ------- | ---------------------------- |
| **Pre-Commit Hook**        | ✅     | `anvil validate` on commit      | —       | Catch issues early           |
| **Pre-Push Hook**          | ✅     | `anvil gate` on push            | —       | Full validation before share |
| **Husky Integration**      | ✅     | Auto-detect, `--husky` flag     | —       | Works with existing setup    |
| **Hook Install/Uninstall** | ✅     | `anvil hooks install/uninstall` | —       | Easy management              |
| **Hook Status**            | ✅     | `anvil hooks status`            | —       | View current state           |
| **Skip Hooks Env**         | ✅     | `ANVIL_SKIP_HOOKS=1`            | —       | Bypass when needed           |

---

## CI/CD Integration

| Feature                | Status | Completed                | Planned                  | Details                    |
| ---------------------- | ------ | ------------------------ | ------------------------ | -------------------------- |
| **JSON Output**        | ✅     | Machine-readable results | —                        | Parse in pipelines         |
| **Exit Codes**         | ✅     | 0=pass, 1=fail           | —                        | Standard CI semantics      |
| **GitHub Action**      | —      | —                        | `anvil-gate` action      | Native GitHub integration  |
| **PR Comments**        | —      | —                        | Auto-post summary        | Inline validation feedback |
| **Commit Status**      | —      | —                        | Set pass/fail status     | Block merge on failure     |
| **GitLab CI Template** | —      | —                        | `.gitlab-ci.yml` example | GitLab support             |
| **Azure Pipelines**    | —      | —                        | `azure-pipelines.yml`    | Azure DevOps support       |

---

## Execution & Rollback

| Feature                | Status | Completed | Planned                 | Details                      |
| ---------------------- | ------ | --------- | ----------------------- | ---------------------------- |
| **Plan Apply**         | —      | —         | `anvil apply <plan>`    | Execute planned changes      |
| **Pre-Apply Snapshot** | —      | —         | `.anvil/snapshots/`     | Capture state before changes |
| **Transactional Ops**  | —      | —         | All-or-nothing changes  | Atomic file operations       |
| **Dry Run Mode**       | —      | —         | `--dry-run` flag        | Preview without changes      |
| **Plan Rollback**      | —      | —         | `anvil rollback <id>`   | Restore from snapshot        |
| **Partial Rollback**   | —      | —         | Select files to restore | Granular control             |
| **Rollback Audit**     | —      | —         | Track all rollbacks     | Compliance trail             |

---

## Visual & Reporting

| Feature                | Status | Completed          | Planned                  | Details                   |
| ---------------------- | ------ | ------------------ | ------------------------ | ------------------------- |
| **CLI Pretty Print**   | ✅     | Chalk, ora, tables | —                        | Beautiful terminal output |
| **HTML Diff Preview**  | —      | —                  | `anvil preview --html`   | Interactive report        |
| **Side-by-Side Diff**  | —      | —                  | File comparison view     | Visual diff               |
| **Blast Radius Graph** | —      | —                  | D3.js dependency graph   | Impact visualisation      |
| **Risk Highlighting**  | —      | —                  | Colour-coded risk levels | Prioritise review         |

---

## IDE Integration

| Feature                | Status | Completed | Planned                 | Details                  |
| ---------------------- | ------ | --------- | ----------------------- | ------------------------ |
| **VS Code Extension**  | —      | —         | Full extension          | Native IDE experience    |
| **Inline Validation**  | —      | —         | Real-time status        | See issues as you type   |
| **Gate Results Panel** | —      | —         | Dedicated view          | All results in one place |
| **Problem Panel**      | —      | —         | Diagnostics integration | Standard VS Code errors  |
| **Status Bar**         | —      | —         | Quick status indicator  | At-a-glance health       |
| **CodeLens**           | —      | —         | Inline actions          | Run gates from code      |
| **MCP Server**         | —      | —         | AI tool integration     | Claude/Copilot support   |

---

## Enterprise Features

| Feature                | Status | Completed | Planned                    | Details                 |
| ---------------------- | ------ | --------- | -------------------------- | ----------------------- |
| **Multi-Language**     | —      | —         | Python, Java, Go support   | Beyond TypeScript/JS    |
| **SSO Authentication** | —      | —         | SAML, OIDC support         | Enterprise auth         |
| **RBAC**               | —      | —         | Role-based access          | Permission management   |
| **Compliance Reports** | —      | —         | SOC2, ISO 27001 format     | Audit-ready output      |
| **License Scanning**   | —      | —         | OSS license compliance     | Legal requirements      |
| **IaC Security**       | —      | —         | Dockerfile, K8s, Terraform | Infrastructure scanning |
| **React Dashboard**    | —      | —         | Plan approval UI           | Visual management       |

---

## Legend

| Symbol            | Meaning                                 |
| ----------------- | --------------------------------------- |
| ✅                | Fully implemented and tested            |
| —                 | Not applicable / No work in this column |
| Text in Completed | What's done                             |
| Text in Planned   | What's coming                           |

---

_Last updated: December 2025_
