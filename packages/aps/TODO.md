# APS Spinout TODO

> **Full Plan**: `docs/planning/aps-spinout-v0.3.aps.md`
>
> This TODO tracks implementation progress. See the plan document for detailed
> task breakdowns, acceptance criteria, and architectural decisions.

---

## 🎯 Next Up

### Phase 1: Package Setup (19 tasks)

- [ ] Document ESM-only stance and Definition of Done
- [ ] Run Nx generator for `packages/aps`
- [ ] Post-generator adjustments (dependencies, tsconfig, eslint)
- [ ] Verification (build, test, lint, graph)

### Phase 2: Spec Definition (4 tasks)

- [ ] Write `APS-Planning-Spec-v0.1.md`
- [ ] Write `APS-Conventions.md`
- [ ] Write `APS-NonGoals.md`
- [ ] Write `APS-Anvil-Integration.md`

---

## 🚧 In Progress

(none)

---

## ⏸️ Blocked

(none)

---

## ✅ Completed

### Migration

- [x] Move spinout plan to `docs/planning/aps-spinout-v0.3.aps.md`

---

## Phase Overview

| Phase                    | Tasks | Status               |
| ------------------------ | ----- | -------------------- |
| 1. Package Setup         | 19    | ⬜ Not started       |
| 2. Spec Definition       | 4     | ⬜ Not started       |
| 3. Template Generation   | 5     | ⬜ Not started       |
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
