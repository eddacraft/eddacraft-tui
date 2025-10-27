# Anvil Roadmap

## Vision

Anvil is the forge where human and AI intent becomes deterministic, auditable
plans and production‑ready features — all inside GitHub, VS Code, and the CLI.

- APS (Anvil Plan Spec) is the universal, hash‑stable planning layer for
  human‑only, AI‑assisted, or hybrid development.
- Packs provide immediate wins (feature flags, telemetry, tests, docs).
- Productioniser hardens prototypes into deployable features.
- Agentic co‑development is optional: start simple, scale to agent workflows
  later.
- Interop first: import/export with GitHub SpecKit and BMAD.

## Themes

- Developer‑first: “with the tools you already use”.
- Trust by design: provenance, one Plan Gate, rollback‑ready.
- Interop: SpecKit & BMAD adapters; engine‑agnostic (Anthropic/OpenAI/Local).
- Progressive agentic capability (Agentic‑Lite now, heavier runtime later).

## Phases & Milestones (MVP Cut)

**✅ Foundations**

- Nx monorepo, pnpm, CI lint+test, strict TypeScript, Husky.

**🚧 Phase 2 — APS Spine**

- Zod schema + JSON Schema export.
- Canonical hashing + golden tests.
- Types exported.
- SpecKit/BMAD adapters (import/export) with fixtures. **Milestone:** APS v0.1
  locked; round‑trip interop demonstrated.

**🚧 Phase 3 — CLI Foundation**

- `plan`, `validate`, `export` commands.
- Plans under `.anvil/plans/` with pretty printing. **Milestone:**
  Create/validate/export locally with friendly UX.

**🚧 Phase 4 — Agentic‑Lite Workflows**

- Linear workflow YAMLs (plan → code → test → review).
- Engine adapters (Anthropic/OpenAI/Mock).
- Assisted Apply (diff review, accept/skip) + immediate tests/lint.
  **Milestone:** Human‑in‑the‑loop flow usable without a daemon.

**🚧 Phase 5 — Plan Gate v1**

- ESLint, Vitest coverage (changed files), secrets scan, SAST placeholder.
- Evidence bundle appended to plan. **Milestone:** Reliable pass/fail on sample
  repos with actionable output.

**🚧 Phase 6 — Provenance Surface**

- Commit trailers (`Anvil-APS`, `Anvil-Engine`, `Prompt-Hash`).
- `anvil prov` report for PR body. **Milestone:** Every PR shows lineage and
  audit trail.

**🚧 Phase 7 — VS Code Extension (Slim)**

- Generate Plan, Review Diffs, Productionise.
- Provenance decorations; engine picker. **Milestone:** Natural editor
  experience; zero new platform.

**🚧 Phase 8 — GitHub PR Experience**

- PR template injection (APS + provenance + evidence).
- AI PR review (tagged) alongside Dependabot + humans; Gate re‑runs on updates.
  **Milestone:** Layered reviews visible directly in PRs.

**🚧 Phase 9 — Packs: Feature Flags**

- `@anvil/flags` OpenFeature provider + file store.
- CLI flags add/set/list/remove → APS deltas.
- Generated tests + docs; client‑side high‑risk flag policy.
- FeatureBoard adapter stub. **Milestone:** First “killer pack” works
  end‑to‑end.

**🚧 Phase 10 — Productioniser (Minimal)**

- Repo scan (tests/docs/telemetry/secrets).
- Generate remediation APS; `anvil productionise .`. **Milestone:** Prototype →
  hardened, mergeable artefacts.

**🚧 Phase 11 — OPA/Rego Policy Engine**

- Vendor OPA; policy bundle + example rules + tests; Gate invokes OPA.
  **Milestone:** Custom policies enforced reliably.

**🚧 Phase 12 — GitHub Integration**

- GitHub Action to run Gate on PR; block on fail.
- Sample app; pass/fail demos; rollback; flags pack demo. **Milestone:**
  End‑to‑end demo on GitHub.

**🚧 Phase 13 — Hardening & Docs**

- SARIF outputs, coverage targets (core/gate ≥85%).
- Dev docs, policy cookbook, contribution guide. **Milestone:** Contributors can
  self‑serve; security validated.

**🚧 Phase 14 — Release Candidate**

- Versioning + changelog; signed artefacts.
- Day‑0 runbook, incident playbook; E2E validation + walkthrough video.
  **Milestone:** RC tagged and published.

## Post‑MVP / Stretch

- Sidecar runtime (daemon, sandboxed dry‑run/apply/rollback).
- Memory layer (vector store + provenance index + citations).
- MCP façade (Cursor/Claude Projects integration).
- Packs marketplace; additional packs (observability/auth/infra).
- Enterprise features (SSO, RBAC, audit exports).
- Multi‑language packs (Python/Go/Java/Kotlin).
- Performance (Rust/Go workers).

## KPIs

- Time‑to‑first‑plan (TTFP).
- Plan→PR conversion rate.
- PR pass rate at first Gate run.
- Gate false‑positive rate (target <5%).
- Productioniser usage rate.
- AI vs human contribution mix (from provenance trailers).

## Risks & Mitigations

- Over‑engineering early → Agentic‑Lite; defer sidecar.
- Interop drift → versioned adapters; round‑trip tests.
- Gate performance → changed‑files scope, parallelism, caching.
- Adoption friction → VS Code + GitHub first; “killer pack” quick win.
