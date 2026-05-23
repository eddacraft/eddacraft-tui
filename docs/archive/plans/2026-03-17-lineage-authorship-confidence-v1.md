# Line-Level Authorship + Confidence (V1) Implementation Plan

> **For assistant:** REQUIRED SUB-SKILL: Use executing-plans to implement this
> plan task-by-task.

**Goal:** Enable `anvil` to answer, for any file/line, who authored it
(human/AI/mixed/unknown), which model was involved (if known), and confidence
with rationale.

**Architecture:** Build a provenance-attribution pipeline with (1) canonical
attribution schema, (2) collectors from Git + session/tool metadata, (3)
confidence engine, (4) persisted line-map store, and (5) query surfaces
(`authorship blame`, PR summaries, confidence heatmap).

**Tech Stack:** TypeScript (CLI/runtime), Rust kernel integration where
latency-sensitive, existing Anvil evidence/audit primitives, signed evidence
bundles.

---

## Scope (V1)

- Repository-level and file-line attribution for changed files in current
  branch/PR.
- Actor classes: `human | ai | mixed | unknown`.
- Model identity fields: `provider`, `model`, `model_version` (nullable).
- Confidence: numeric `0.0-1.0` + reason codes.
- Query commands:
  - `anvil authorship blame <file>:<line>`
  - `anvil authorship summary --pr <id|HEAD>`

Out of scope (V1): org-wide historical backfill, non-git sources without
adapters, real-time IDE overlays.

---

## Canonical Data Model

### Task 1: Define attribution schema

**Files:**

- Create: `packages/anvil/contracts/src/provenance/attribution.schema.ts`
- Create: `packages/anvil/contracts/src/provenance/attribution.types.ts`
- Test: `packages/anvil/contracts/src/provenance/attribution.schema.test.ts`

Deliverables:

- `LineAttributionRecord` with fields:
  - subject: repo, commit, file, lineStart, lineEnd
  - attribution: actorType, actorId?, toolSurface?, sessionId?
  - model: provider?, model?, version?
  - evidence: evidenceIds[], sourceKinds[]
  - confidence: score, band, reasonCodes[]
  - integrity: recordHash, signature?

### Task 2: Add confidence reason taxonomy

**Files:**

- Create: `packages/anvil/contracts/src/provenance/confidence-reasons.ts`
- Test: `packages/anvil/contracts/src/provenance/confidence-reasons.test.ts`

Reason code set (initial):

- DIRECT_SIGNED_SESSION_MATCH
- DIRECT_GIT_NOTES_MATCH
- TEMPORAL_CORRELATION_ONLY
- MODEL_METADATA_MISSING
- CONFLICTING_EVIDENCE
- NO_EVIDENCE

---

## Collectors + Reconciliation

### Task 3: Git collector

**Files:**

- Create: `packages/anvil/core/src/provenance/git-collector.ts`
- Test: `packages/anvil/core/src/provenance/git-collector.test.ts`

Collect:

- commit metadata, author/committer
- hunks/line ranges for changed files
- git notes — auto-fetch `refs/notes/ai` from remote when not present locally
  (mirrors the existing `authorship` command's fetch hint)

### Task 4: AI/session metadata collector

**Files:**

- Create: `packages/anvil/core/src/provenance/session-collector.ts`
- Test: `packages/anvil/core/src/provenance/session-collector.test.ts`

Collect:

- AI tool session IDs
- provider/model fields
- session hashes / tool receipts

### Task 5: Reconciliation engine

**Files:**

- Create: `packages/anvil/core/src/provenance/reconciler.ts`
- Test: `packages/anvil/core/src/provenance/reconciler.test.ts`

Rules:

- Merge multi-source claims per line range
- Detect conflicts
- Emit final actor type + model fields + confidence reason codes

---

## Confidence Engine

### Task 6: Implement deterministic confidence scoring

**Files:**

- Create: `packages/anvil/core/src/provenance/confidence-engine.ts`
- Test: `packages/anvil/core/src/provenance/confidence-engine.test.ts`

Initial scoring profile:

- Direct signed session + line match: 0.90-0.98
- Direct git-notes attestation only: 0.75-0.89
- Time-window correlation only: 0.40-0.60
- Conflicting signals: cap at 0.45
- No evidence: 0.05-0.20

Band mapping:

- High >= 0.80
- Medium 0.50-0.79
- Low < 0.50

---

## Storage + Query Surfaces

### Task 7: Persist line-map records

**Files:**

- Create: `packages/anvil/core/src/provenance/store.ts`
- Create: `packages/anvil/core/src/provenance/store.sqlite.ts` (or existing DB
  adapter)
- Test: `packages/anvil/core/src/provenance/store.test.ts`

Requirements:

- upsert by (repo, commit, file, line range)
- query by file+line, commit, PR scope

### Task 8: Add CLI command `authorship blame`

**Files:**

- Modify: `apps/anvil-cli/src/commands/authorship.ts` (add `blame` subcommand)
- Test: `apps/anvil-cli/src/commands/authorship.test.ts`

Output:

- actor type, model, confidence score/band, reasons, evidence refs

### Task 9: Add CLI command `authorship summary`

**Files:**

- Modify: `apps/anvil-cli/src/commands/authorship.ts` (add `summary` subcommand)
- Test: `apps/anvil-cli/src/commands/authorship.test.ts`

Output:

- % human, % ai, % mixed, % unknown
- unknown/low-confidence hotspots

---

## Integrity + Evidence

### Task 10: Sign attribution bundles (optional key)

**Files:**

- Create: `packages/anvil/core/src/provenance/signer.ts`
- Modify: `packages/platform/crypto/...` (minimal additions if needed)
- Test: `packages/anvil/core/src/provenance/signer.test.ts`

Behavior:

- if key configured, sign bundle
- else unsigned with explicit marker

### Task 11: Evidence export

**Files:**

- Create: `apps/anvil-cli/src/commands/authorship-export.ts`
- Test: `apps/anvil-cli/src/commands/authorship-export.test.ts`

Formats:

- JSON (required)
- SARIF-like mapping (optional in v1.1)

---

## Language Decision Tree (TS vs Rust kernel)

> Canonical source of truth is now extracted to:
> `plans/decisions/014-language-allocation-tree-ts-vs-rust.md`. Keep this
> section as implementation context; update ADR-014 for long-term policy
> changes.

### Decision policy

1. **Use TypeScript** when logic is:
   - schema orchestration n - adapter integration
   - command wiring/formatting
   - expected runtime < 100ms per file and no hot loops

2. **Use Rust kernel** when logic is:
   - per-line/hunk heavy reconciliation on large diffs
   - repeated hashing/signature pre-processing at scale
   - CPU-bound confidence feature extraction over large repos
   - target p95 latency needs < 50ms under heavy load

3. **Hybrid pattern (default for this feature):**
   - TS owns orchestration, policy, UX, and source adapters.
   - Rust owns optional accelerated line-map diff/reconcile primitives behind
     stable interface.

### V1 recommendation

- Implement full V1 in TypeScript first for speed of delivery.
- Add instrumentation and benchmark thresholds.
- Promote specific hot paths to Rust only when thresholds are breached.

Promotion trigger thresholds:

- `authorship summary --pr` > 2s on 1k changed lines
- memory > 512MB for reconciliation step
- p95 blame query > 200ms

---

## Two-Week Delivery Cut

### Week 1 (Core correctness)

- Day 1-2: Task 1-2 schemas + reason taxonomy
- Day 3: Task 3 git collector
- Day 4: Task 4 session collector
- Day 5: Task 5 reconciler + Task 6 confidence engine

Exit criteria:

- deterministic reconciliation tests green
- confidence reason codes emitted for all outcomes

### Week 2 (Usability + trust)

- Day 1: Task 7 storage
- Day 2-3: Task 8 blame command
- Day 4: Task 9 summary command
- Day 5: Task 10 signing (optional path) + Task 11 export

Exit criteria:

- single-line query works end-to-end
- PR-level attribution summary works
- signed/unsigned evidence status explicit

---

## Acceptance Criteria (V1)

- Given a changed line with direct AI session evidence, `blame` shows actor=ai,
  model details, confidence >= 0.80.
- Given mixed/conflicting evidence, actor=mixed or unknown with confidence <
  0.50 and reason codes.
- `summary` reports distribution + low-confidence hotspots.
- Exports contain enough fields for audit replay and explainability.

## Risks

- Missing model metadata from upstream tools.
- Conflicts between git authorship and AI session mapping.
- Overfitting confidence rules without calibration dataset.

## Mitigations

- Default unknown + explicit low confidence over false precision.
- Persist reason codes for every score.
- Add calibration pass after first 100 real PRs.
