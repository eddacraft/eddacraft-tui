# Monorepo Cleanup Impact Assessment

> **Assessment Date:** 2026-01-12 **Context:** Pre go-live restructuring
> evaluation **Target:** Nx monorepo with apps/packages separation and layered
> architecture

---

## Executive Summary

| Aspect               | Impact                           | Risk   | Recommendation             |
| -------------------- | -------------------------------- | ------ | -------------------------- |
| **Scope**            | Major restructuring              | High   | Defer to post-launch       |
| **Timeline**         | 2-4 weeks full-time              | -      | -                          |
| **Breaking changes** | All import paths                 | High   | Requires coordination      |
| **Testing impact**   | 1,473+ tests need path updates   | Medium | Automate migration         |
| **Go-live risk**     | Significant regression potential | High   | Not recommended pre-launch |

**Verdict:** The restructuring is valuable but should be planned as a **v1.1
milestone**, not a go-live blocker.

---

## 1. Current State Analysis

### 1.1 Current Structure (Flat)

```
anvil-001/
├── core/                    # @anvil/core (schema, validation, gates)
├── cli/                     # @anvil/cli (Commander.js CLI + TUI)
├── ui/                      # (referenced in workspace but minimal)
├── packs/                   # (referenced in workspace)
├── packages/
│   ├── adapters/           # @anvil/adapters
│   ├── aps/                # @anvil/aps
│   ├── eslint-plugin-anvil/
│   └── vscode-extension/
├── e2e/                    # Playwright tests
├── docs/                   # Internal documentation
├── plans/                  # APS planning specs
└── scripts/                # Build utilities
```

### 1.2 Current Package Count

| Location                        | Package             | Status     |
| ------------------------------- | ------------------- | ---------- |
| `core/`                         | @anvil/core         | Production |
| `cli/`                          | @anvil/cli          | Production |
| `packages/adapters/`            | @anvil/adapters     | Production |
| `packages/aps/`                 | @anvil/aps          | Production |
| `packages/eslint-plugin-anvil/` | eslint-plugin-anvil | Production |
| `packages/vscode-extension/`    | anvil-vscode        | Complete   |
| `ui/`                           | @anvil/ui           | Minimal    |
| `packs/`                        | @anvil/packs        | Minimal    |

**Total: 8 workspace packages**

### 1.3 Cross-Package Dependencies

- **146 import statements** using `@anvil/*` across 68 files
- **Primary dependency flow:** adapters → aps → core ← cli
- **No circular dependencies** detected

---

## 2. Target State Analysis

### 2.1 Target Structure (Layered)

```
anvil/
├─ apps/
│  ├─ anvil-cli/           # CLI entrypoint (moved from cli/)
│  ├─ anvil-api/           # NEW: API/gateway
│  ├─ anvil-ui/            # NEW: UI for plans/runs/audits
│  ├─ website/             # NEW: Marketing site
│  ├─ docs-site/           # NEW: Docusaurus public docs
│  └─ e2e/                 # Reorganised E2E suites
│     ├─ cli-e2e/
│     ├─ api-e2e/
│     ├─ ui-e2e/
│     ├─ website-e2e/
│     ├─ docs-e2e/
│     └─ oss-compat-e2e/
│
├─ packages/
│  ├─ anvil/               # SPLIT from core/
│  │  ├─ contracts/        # schemas/events/types
│  │  ├─ ports/            # interfaces
│  │  ├─ core/             # pure domain logic
│  │  ├─ runtime/          # orchestration
│  │  ├─ policy/           # OPA/Rego wrappers
│  │  └─ sdk/              # client SDK
│  │
│  ├─ edda-stack/          # NEW: Memory/proposal system
│  │  ├─ contracts/
│  │  ├─ ports/
│  │  ├─ ember/
│  │  ├─ edda/
│  │  └─ testing/
│  │
│  ├─ adapters/            # Reorganised (one per integration)
│  │  ├─ adapter-github/
│  │  ├─ adapter-opencode/
│  │  ├─ adapter-claude-code/
│  │  └─ adapter-<system>/
│  │
│  ├─ platform/            # NEW: Cross-cutting concerns
│  │  ├─ config/
│  │  ├─ storage/
│  │  ├─ telemetry/
│  │  ├─ auth/
│  │  ├─ crypto/
│  │  └─ http/
│  │
│  ├─ shared/              # NEW: Shared utilities
│  │  ├─ util/
│  │  ├─ testing/
│  │  └─ brand/
│  │
│  └─ tooling/             # NEW: Build tooling
│     ├─ eslint-config/
│     ├─ tsconfig/
│     └─ release/
│
├─ tools/                  # Nx generators + scripts
│  ├─ generators/
│  ├─ scripts/
│  └─ docker/
│
└─ docs/                   # Internal engineering docs
   ├─ architecture/
   ├─ decisions/
   ├─ runbooks/
   └─ security/
```

### 2.2 Target Package Count

| Category             | Packages              | Status            |
| -------------------- | --------------------- | ----------------- |
| apps/                | 6 apps + 6 e2e suites | 2 existing, 4 new |
| packages/anvil/      | 6 sub-packages        | Split from 1      |
| packages/edda-stack/ | 5 sub-packages        | All new           |
| packages/adapters/   | 4+ adapter packages   | Split from 1      |
| packages/platform/   | 6 sub-packages        | Extract from core |
| packages/shared/     | 3 sub-packages        | New               |
| packages/tooling/    | 3 sub-packages        | Consolidate       |

**Total: ~40+ workspace packages** (from current 8)

---

## 3. Impact Analysis

### 3.1 Code Changes Required

| Change Type           | Count             | Effort      |
| --------------------- | ----------------- | ----------- |
| File moves            | ~257 source files | Medium      |
| Import path updates   | 146+ statements   | Automatable |
| package.json rewrites | 8 → 40+           | High        |
| project.json creation | 3 → 40+           | High        |
| tsconfig updates      | All packages      | Medium      |
| Test file moves       | 103 test files    | Medium      |
| CI/CD updates         | Workflow files    | Medium      |
| Documentation updates | All READMEs       | Low         |

### 3.2 Risk Assessment

| Risk                          | Likelihood | Impact   | Mitigation                     |
| ----------------------------- | ---------- | -------- | ------------------------------ |
| Broken imports post-migration | High       | High     | Automated codemods             |
| Test failures                 | High       | Medium   | Run full suite after each move |
| Build order issues            | Medium     | High     | Nx graph validation            |
| CI pipeline failures          | Medium     | Medium   | Feature branch testing         |
| Incomplete migration          | Medium     | High     | Atomic commits per package     |
| Go-live delay                 | High       | Critical | **Defer to post-launch**       |

### 3.3 Dependency Graph Changes

**Current (simple):**

```
cli → core
cli → adapters → aps → core
vscode-extension → core
```

**Target (layered):**

```
apps/anvil-cli → packages/anvil/runtime → packages/anvil/core
                                        → packages/anvil/ports
                                        → packages/anvil/contracts

packages/anvil/runtime → packages/platform/*
                       → packages/adapters/*
                       → packages/edda-stack/contracts

packages/adapters/* → packages/anvil/ports
                    → packages/anvil/contracts
```

This introduces **explicit architectural boundaries** which is good long-term
but requires careful orchestration.

---

## 4. Effort Estimation

### 4.1 Phase Breakdown

| Phase                         | Tasks                                                | Duration |
| ----------------------------- | ---------------------------------------------------- | -------- |
| **Phase 1: Preparation**      | Create target structure, Nx generators               | 2-3 days |
| **Phase 2: Core Split**       | Split core/ into contracts/ports/core/runtime/policy | 3-4 days |
| **Phase 3: App Migration**    | Move cli/ to apps/anvil-cli/, set up other apps      | 2-3 days |
| **Phase 4: Adapters Split**   | Split adapters into per-integration packages         | 2-3 days |
| **Phase 5: Platform Extract** | Extract platform/\* from core                        | 2-3 days |
| **Phase 6: New Packages**     | Create edda-stack/, shared/, tooling/                | 3-4 days |
| **Phase 7: Validation**       | Full test suite, CI fixes, documentation             | 2-3 days |

**Total Estimate: 16-23 working days (3-5 weeks)**

### 4.2 Resource Requirements

- Dedicated engineer(s) for migration
- Feature freeze during active migration phases
- Extended CI time for validation
- Documentation sprint post-migration

---

## 5. Go-Live Impact Analysis

### 5.1 Arguments FOR Cleanup Before Go-Live

| Argument                                   | Weight |
| ------------------------------------------ | ------ |
| Clean architecture from day one            | Medium |
| Avoid "migrate while running" complexity   | Medium |
| External contributors see final structure  | Low    |
| Easier to explain architecture to new team | Low    |

### 5.2 Arguments AGAINST Cleanup Before Go-Live

| Argument                                     | Weight   |
| -------------------------------------------- | -------- |
| **3-5 week delay to launch**                 | Critical |
| High regression risk                         | High     |
| Current structure is functional              | High     |
| No external users yet to disrupt             | Medium   |
| Can migrate incrementally post-launch        | High     |
| Feature development blocked during migration | High     |

### 5.3 What Actually Matters for Go-Live

The current structure supports:

- ✅ CLI functionality (validate, gate, watch, export)
- ✅ VS Code extension
- ✅ All 1,473 tests passing
- ✅ CI/CD pipeline working
- ✅ Documentation complete

The target structure provides:

- Better separation of concerns
- Clearer dependency boundaries
- Room for edda-stack/website/api expansion
- Professional monorepo appearance

**None of these target benefits are required for initial launch.**

---

## 6. Recommendations

### 6.1 Primary Recommendation: Defer to v1.1

**Do not perform the full restructuring before go-live.**

Rationale:

1. Current structure is production-ready
2. 3-5 week delay unacceptable for launch timing
3. High regression risk with no safety net of production traffic
4. Can migrate incrementally with real user feedback

### 6.2 Acceptable Pre-Launch Cleanup (Low Risk)

If cleanup is desired, limit to these low-impact changes:

| Change                                                 | Risk | Time    |
| ------------------------------------------------------ | ---- | ------- |
| Create `apps/` folder, move `cli/` → `apps/anvil-cli/` | Low  | 1 day   |
| Create `tools/` folder, move `scripts/`                | Low  | 0.5 day |
| Consolidate docs structure                             | Low  | 0.5 day |
| Add `.changeset/` for versioning                       | Low  | 0.5 day |

**Total: 2-3 days max**

### 6.3 Post-Launch Migration Plan (v1.1)

| Week   | Focus                                         |
| ------ | --------------------------------------------- |
| Week 1 | Create target folder structure, Nx generators |
| Week 2 | Split core/ into layered packages             |
| Week 3 | Migrate adapters to per-integration packages  |
| Week 4 | Extract platform/ services                    |
| Week 5 | Create shared/, tooling/, validate everything |

### 6.4 Migration Tooling to Prepare

1. **Nx generators** for new package scaffolding
2. **Codemod scripts** for import path updates
3. **Validation scripts** to check dependency boundaries
4. **Rollback plan** for each migration phase

---

## 7. Decision Matrix

| Option                                  | Risk | Time         | Go-Live Impact |
| --------------------------------------- | ---- | ------------ | -------------- |
| **A: Full restructure now**             | High | 3-5 weeks    | Delays launch  |
| **B: Minimal cleanup now**              | Low  | 2-3 days     | Minimal delay  |
| **C: No changes, defer all**            | None | 0            | No delay       |
| **D: Hybrid (B then full post-launch)** | Low  | 2-3 days now | Best balance   |

**Recommended: Option D** - Minimal cleanup now, full restructure as v1.1
milestone.

---

## 8. Conclusion

The target monorepo structure is well-designed and represents good architectural
practice. However, the current structure is functional, tested, and
production-ready.

**The restructuring should be treated as a v1.1 feature, not a go-live
prerequisite.**

Attempting a major restructuring before launch introduces unnecessary risk and
delay with no immediate user benefit. The professional appearance of a clean
monorepo is less important than shipping working software.

### Next Steps

1. **Approve or modify** this assessment
2. If proceeding with minimal cleanup (Option D):
   - Create `apps/` and move CLI
   - Create `tools/` and move scripts
   - Update workspace configuration
3. Create v1.1 milestone for full restructuring
4. Proceed to go-live with current structure

---

_Assessment prepared for Anvil go-live planning_
