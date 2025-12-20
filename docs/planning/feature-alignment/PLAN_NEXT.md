# PLAN_NEXT — Anvil (Next)

## 0) One-sentence plan
Ship a developer-first Anvil v1 that makes AI-generated code safe to merge by catching **architecture boundary violations** and **AI escape-hatch anti-patterns** at **file-save time**, with actionable guidance and human-owned exceptions; then expand into PR/CI fail-safes, drift reporting, and AI-guided generation.

---

## 1) Product thesis (locked)
Anvil is a tool for **AI ↔ human collaboration**.

- Primary purpose: improve the trust developers have in AI-generated code so **more of it reaches production faster**.
- Secondary (high-value) benefit: it also improves quality and architecture compliance for human-written code.

### North-star outcomes (both must hold)
1. Teams merge significantly more AI-generated code **with confidence**.
2. Architecture drift **slows or reverses** over time.

### Primary beneficiary
Individual developers feel the benefit first (they get to use AI safely at the pace leadership expects).

---

## 2) Core problem (what we prevent)
The most damaging recurring failure is **second-wave feature work** drifting away from intended patterns because engineers:
- don’t know which patterns apply,
- don’t read ADRs/diagrams,
- and don’t recognise when their change crosses a boundary.

Most reliable early signal: a **new dependency edge** where a function/class reaches across contexts.

---

## 3) Strategy (layered safety, fast feedback)
### Planless-first posture
Anvil must deliver strong value **without requiring a plan**.
Baseline source of truth (when no plan exists): the **current codebase and its dependency structure**.
Plans/APS remain valuable, but as an *accelerant* and governance layer (not a prerequisite).

### Feedback loops
- **Primary loop:** IDE/CLI on **file save** (near real time).
- **Fail-safe loop:** PR/CI/web commits mirror the same warnings later.
- **Reflective loop:** drift reports show how the system evolves over time.

---

## 4) v1 objective and scope
### v1 objective
Make the safe path the easy path for developers using AI.

### v1 capabilities (must-have)
1. **On-save analysis** (local, in-flow)
2. **Architecture boundary safety** (baseline + new-edge detection; acknowledge legacy drift)
3. **AI anti-pattern safety** (high-confidence warnings for escape hatches)
4. **Actionable warnings** (explanation + deterministic suggestion)
5. **Exceptions with accountability** (explicit suppression with human note; inline + provenance)
6. **Impact framing** (directional; runtime entry-point anchor; careful language + low-confidence surfacing)

### v1 out-of-scope (explicit)
- Requiring plans/APS for value
- Heavy org dashboards
- Hard-blocking by default
- Auto-fixing as the default
- Perfect feature/journey mapping on day one

---

## 5) `anvil init` bootstrap (architecture inference)
Architecture inference should not be a mysterious assumption.

- **Exploratory by default:** propose multiple plausible models and let the user choose.
- **Fallback:** descriptive mode if user can’t choose.
- **Descriptive view:** entry points → internals (public vs internal reachability).
- Re-surface maps at violation time + in drift reports.

---

## 6) AI visibility roadmap (cascading maturity)
1. **AI-aware:** export constraints/patterns/anti-patterns as machine-readable artefacts.
2. **AI-reflective:** emit structured feedback AI tools can consume.
3. **AI-guiding:** inject authoritative constraints during generation.
4. **AI-collaborative (later):** constrained feedback loops.

Hard boundary: **humans approve exceptions / risk acceptance**.

---

## 7) Phased roadmap
### Phase 0 — Refocus and align (1–2 sprints)
- Reframe README/landing narrative around trust + AI collaboration.
- Define v1 acceptance criteria and warning schema v1.

### Phase 1 — v1 “Save-time Trust” (2–4 sprints)
- On-save runner
- Architecture baseline inference + new-edge detection
- Anti-pattern library v1 + explanation/suggestion
- Suppression + provenance (inline + structured)
- PR/CI mirroring (basic)

### Phase 2 — Drift + impact (next)
- Entry-point mapping to runtime surfaces
- Drift reports (snapshot + compare)
- Better boundary modelling via init refinements

### Phase 3 — AI-guiding (next)
- Export constraints + reflective feedback artefacts
- Optional AI-assisted suggestions behind safety wrappers

---

## 8) Success metrics
- AI-assisted throughput ↑ without longer review cycles
- Developer confidence ↑
- New cross-boundary edges/week ↓
- Meaningful suppression notes % ↑
- Save-time feedback latency ↓

---

## 9) Risks & mitigations
- Noise kills adoption → high-signal first; warn on new edges only
- Performance → incremental analysis; caching
- Over-claiming impact → careful phrasing; show low-confidence
- Legacy drift overwhelm → acknowledge existing; focus on new
- Suppression rot → require note; provenance; drift review

---

## 10) Open questions (tracked)
- First editor integration (VS Code vs JetBrains) vs CLI-only v1
- Which runtime entry points to support first
- Provenance storage format
- Optional detection of AI-authored changes
