# APS Spinout TODO

> **Full Plan**: `docs/planning/aps-spinout-v0.3.aps.md`
>
> This TODO tracks implementation progress. See the plan document for detailed
> task breakdowns, acceptance criteria, and architectural decisions.

---

## 🎯 Next Up

### Phase 3: Template Generation (5 tasks)

- [ ] Create template generator script
- [ ] Generate index template from spec
- [ ] Generate leaf spec template from spec
- [ ] Add Nx target for template regeneration
- [ ] Add tests for template generation

---

## 🚧 In Progress

### Phase 3: Template Generation (5 tasks)

Creating template generation functionality.

---

## ⏸️ Blocked

(none)

---

## ✅ Completed

### Phase 2: Spec Definition (4 tasks)

- [x] Wrote `APS-Planning-Spec-v0.1.md` - Complete specification with examples
- [x] Wrote `APS-Conventions.md` - Detailed conventions for file naming, links,
      IDs, fields
- [x] Wrote `APS-NonGoals.md` - Explicit non-goals to prevent scope creep
- [x] Wrote `APS-Anvil-Integration.md` - How planning docs become execution
      plans

### Phase 1: Package Setup (19 tasks)

- [x] Manually created package structure (Nx generator timed out)
- [x] Created `package.json` with ESM config and explicit exports
- [x] Created `project.json` with Nx configuration
- [x] Created TypeScript configs (tsconfig.json, tsconfig.lib.json,
      tsconfig.spec.json)
- [x] Created vitest.config.ts with coverage exclusions for docs/examples
- [x] Created eslint.config.mjs with docs/examples ignores
- [x] Added dependencies: @anvil/core, remark-parse, unified, unist-util-visit
- [x] Created placeholder modules: parser, loader, validator, state, types
- [x] Updated tsconfig.base.json with @anvil/aps path mapping
- [x] Installed dependencies with pnpm
- [x] Verified build succeeds (dist/ created with correct structure)
- [x] Verified typecheck passes
- [x] Created docs/, examples/, and src/ directories
- [x] Created symlink from docs/guides/aps → packages/aps/docs
- [x] Created README.md

### Migration

- [x] Move spinout plan to `docs/planning/aps-spinout-v0.3.aps.md`

---

## Phase Overview

| Phase                    | Tasks | Status               |
| ------------------------ | ----- | -------------------- |
| 1. Package Setup         | 19    | ✅ Completed         |
| 2. Spec Definition       | 4     | ✅ Completed         |
| 3. Template Generation   | 5     | 🟡 In progress       |
| 4. Examples              | 3     | ⬜ Not started       |
| 5a. Single-File Parser   | 5     | ⬜ Not started       |
| 5b. Index + Links        | 5     | ⬜ Not started       |
| 5c. Graph Features       | 3     | ⬜ Not started       |
| 5d. Filtering & Scoping  | 3     | ⬜ Not started       |
| 6. Validation            | 10    | ⬜ Not started       |
| 7. Task Locking          | 6     | ⬜ Not started       |
| 8. CLI Surface + Dogfood | 7     | ⬜ Not started       |
| Migration                | 5     | 🟡 In progress (1/5) |
| Dogfooding               | 5     | ⬜ Not started       |

**Total**: ~85 tasks

---

## Notes

- **Package doesn't exist yet** — Phase 1 creates it
- **ESM-only stance** — No CJS support
- **remark for parsing** — Not regex-based
- **Scopes vs Tags** — Scopes constrain LLM, Tags for filtering

---

_Last updated: December 2025_
