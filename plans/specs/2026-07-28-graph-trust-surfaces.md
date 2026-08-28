# Graph Trust Surfaces — Programme Note

| Type | Authority | Owner | Status | Freshness |
| ---- | --------- | ----- | ------ | --------- |
| Spec | Derived | APS modules named below | Live | 2026-08-27 — CONF-001/ADR-134 and current release/NBI references reconciled |

| Upstream | Downstream |
| -------- | ---------- |
| `plans/index.aps.md` (NBI), named APS modules, `ROADMAP.md` Horizon 2–5 | NBI Schedule rows, Wave 0 clearance work, future release-window scoping |

**Not** the active provisional `v0.9.8-beta` field-intake authority. That
remains [`RELEASE-PLAN.md`](../../RELEASE-PLAN.md). This note schedules a
**side programme** that may become a later minor theme once Wave 1 demos prove
the story.

## 1. Product story

anvil already holds a trusted graph (GV2 / GCTX / GBASE, shipped in
`v0.9.0-beta`). Graph Trust Surfaces answers five questions agents and team
leads care about, without inventing a second product:

| # | Question | Module | Wave-1 claim |
| - | -------- | ------ | ------------ |
| 1 | Did this change match what was claimed? | [CONF](../modules/intent-conformance.aps.md) | Tier 0: per-commit claims vs pinned Git coverage + claim-appropriate bound evidence, advisory |
| 2 | Where is this symbol used, exactly? | [LSPNAV](../modules/lsp-graph-navigation.aps.md) | One language/client: exact graph-backed `textDocument/references` |
| 3 | Did review leave a durable gate trail? | [CGBDG](../modules/council-gate-bridge.aps.md) | Council verdict → anvil-shaped evidence (prefer thin witness lines) |
| 4 | What may this agent attempt? | [POLCAP](../modules/policy-capability-discovery.aps.md) | Advisory `anvil policy capabilities` signed view |
| 5 | What new deps did this change pull in? | [SCA](../modules/supply-chain-attestation.aps.md) | One ecosystem: baseline + new-edge warnings |

Dashboard surfaces may later consume evidence these tracks produce. This
programme is not a claim of the active provisional `v0.9.8-beta` window.

## 2. Waves

### Wave 0 — Unblockers (planning and discovery)

| Track | Work | Mode today | Outcome that clears Wave 1 |
| ----- | ---- | ---------- | -------------------------- |
| **CGBDG** | CGBDG-001..006 discovery | **Ready** — execute | Discovery report + follow-on implement/spec or explicit park |
| **CONF** | CONF-002..004 Tier-0 implementation | **In Progress**; CONF-001 Merged via #4174 | Implement/prove ADR-134's Git, base-tree scope-authority, and evidence-binding contract; keep CONF-005..009 Proposed |
| **POLCAP** | POLCAP-001 ADR + Planning Council | Proposed | Accepted ADR; AD-3/AD-4 reconciled with ADR-098 |
| **SCA** | SCA-001 design (one ecosystem + graph shape) | Proposed | Design doc; edge-type home decided |
| **LSPNAV** | RTAI-005 diagnostics-only + ADR-111 Accept | Proposed | RTAI-005 production boundary + ADR-111 Accepted |

### Wave 1 — First demos (implementation, after clearance)

| Track | First executable slice | Explicitly out of Wave 1 |
| ----- | ---------------------- | ------------------------ |
| CONF | CONF-002..004 (minimal contract + commit claims vs claim-appropriate evidence) | ILGOV session ledger; Tier-2 APS adapters; PR-body claims; symbol-level claims beyond bound GV2 support |
| CGBDG | Follow-on implement only if CGBDG-006 warrants it | PocketFlow re-platform of council; LLM on enforcement path |
| POLCAP | Schema + 3–5 recipes + CLI (advisory) | Daemon IPC optional stretch; asymmetric signing; dashboard |
| SCA | One ecosystem SBOM → baseline → new-edge warn | Multi-ecosystem; hosted vuln DB; SLSA release attestation |
| LSPNAV | LSPNAV-001..005 after Wave 0 gate | Hover/rename/definitions; impact/affected-tests |

### Wave 2 — Close the loops

- CONF-005..007 and CONF-009 (PR claims, correlation join, closeout, and
  minimised/privacy-reviewed capsule grading)
- POLCAP daemon IPC + witness `cap_id` binding
- SCA release-time attestation (optional)
- LSPNAV promotion evidence and soak

### Wave 3 — Demand-pulled enrichment

- CONF Tier-1/2 (ILGOV + plan adapters)
- LSPNAV-006/007 impact and affected-tests
- POLCAP recipe expansion / v2 signing
- SCA multi-ecosystem + vuln correlation

## 3. Parallelism

```text
CGBDG discovery ──┐
CONF-001 ADR ─────┼── independent (Wave 0)
POLCAP-001 council┤
SCA-001 design ───┘
RTAI-005 ──► LSPNAV (serial; only hard chain)
```

CGBDG, CONF, POLCAP, and SCA do not block each other. LSPNAV waits on RTAI-005
(diagnostics-only production boundary) and ADR-111 acceptance.

## 4. Kill / park criteria

| Track | Park if |
| ----- | ------- |
| CONF | Exact per-commit Git extraction or revision/schema/generation-bound GV2 evidence cannot be proved without heroic work |
| LSPNAV | RTAI-005 keeps navigation scope, or occurrence snapshots break ADR-031 budgets |
| CGBDG | No clean map without a second attestation product — document and stop |
| POLCAP | Council cannot keep the surface advisory without a parallel policy evaluator |
| SCA | Dependency edges need a dedicated store that balloons past "new edges only" |

## 5. APS authority

| Document | Role |
| -------- | ---- |
| This note | Programme framing, wave order, clearance list — **not** execution authority |
| Named `plans/modules/*.aps.md` | Work-item status, Files, Validation — **execution authority** |
| `plans/index.aps.md` NBI | Ranked pick-up and Schedule rows |
| `RELEASE-PLAN.md` | Active release window only (do not accrete this programme as a second window) |
| ADRs under `plans/decisions/` | Durable decisions (CONF-001, POLCAP-001, ADR-111, SCA edge home) |

### Cleared for APS Option B (execute or promote)

| Module | Disposition (2026-08-27) |
| ------ | ------------------------ |
| **CGBDG** | **Ready**; NBI rank 3. Discovery may start without further status promotion. |
| **CONF** | CONF-001 **Merged via #4174**; ADR-134 is accepted. CONF-002..004 remain **In Progress** until merge, with implementation and executor proof complete on `feat/conf-002-004-tier0`. Existing `GraphDelta.file` alone is not sufficient evidence; exact Git extraction and claim-appropriate binding are now proved by focused tests. |
| POLCAP / SCA / LSPNAV | **Not** Ready. Clearance steps in §6 still apply. |

## 6. Clearance checklist (to unlock the rest)

### CONF — Intent conformance (Tier 0)

- [x] Author and accept **CONF-001** product ADR (ADR-134: in-lane; tier model;
      naming "conformance" not "drift"; planless Tier 0)
- [x] Pin deterministic per-commit Git extraction, explicit `path:` grammar,
      fixed extraction/rename budgets with reason-coded overflow,
      replacement-disabled sanitised Git execution, option-safe revision
      resolution, base-tree scope-mapping authority, claim-appropriate
      evidence, per-member dispositions, monotonic outcome/evidence-strength
      semantics, and the capsule privacy gate in ADR-134
- [x] Record that `GraphDelta.file` is an input, not sufficient per-file
      evidence or clearance without revision/schema/generation binding
- [x] Sequence the **Tier-0 contract** so CONF-002 can land without waiting for
      full ILGOV `IntentLedgerRecord` rescope item 1 (co-design seam; no fork)
- [x] Promote CONF-002..004 on 2026-08-28 with concrete Rust ownership and
      validation commands
- [x] Implement and prove exact Git extraction plus repository/worktree,
      revision/blob, graph-schema, generation, and run binding in CONF-002..004;
      enforce base-tree mapping authority and `policy-change` dispositions
- [x] Leave CONF-005..009 Proposed until later-wave dogfood

### LSPNAV — Graph-backed references

- [ ] Land **RTAI-005** as production **diagnostics-only** (PR #3360 scope
      cleanup; no navigation in RTAI)
- [ ] Rebase/reconcile LSPNAV module to the final RTAI-005 surface
- [ ] Accept and index **ADR-111**
- [ ] Complete LSPNAV-001 tier selection evidence
- [ ] Promote LSPNAV-001..005 to **Ready** only after the above

### POLCAP — Capability discovery (advisory)

- [ ] Author **POLCAP ADR** (next free number at authoring; do not reuse 051/092)
- [ ] Convene **Planning Council** (required by module holdCondition)
- [ ] Reconcile with **ADR-098** AD-3 (`ControlDecision`) and AD-4 (no new
      tool-call interception without its own ADR)
- [ ] Confirm ACTAX-001 / AGOV-007 coordination notes (or explicit defer for v1
      CLI-only slice)
- [ ] Promote POLCAP-002..005 (schema, recipes, CLI) to **Ready** after council

### SCA — Supply chain new-edges

- [ ] Write **SCA-001** design: one ecosystem (cargo **or** npm), CycloneDX
      generator, merge/baseline cadence
- [ ] Decide graph home: existing graph edge type vs dedicated dep graph
- [ ] Confirm SEC handoff (SEC-006 stays pointer to SCA; no duplicate SBOM home)
- [ ] Confirm graph can **ingest** component + depends-on edges (gating
      prerequisite from the module)
- [ ] Promote SCA-002 + new-edge warn items to **Ready** after design accept

### CGBDG — already clear

- [x] Witness chain + schema follow-ups terminal
- [x] Discovery items Ready
- [ ] Execute CGBDG-001..006 → discovery report at
      `plans/specs/YYYY-MM-DD-council-gate-bridge-discovery.md`
- [ ] If thin path wins, file follow-on implementation work items; else park

## 7. Demo narrative (when Wave 1 lands)

1. Agent asks what it may do → **POLCAP**
2. Agent resolves exact references while editing → **LSPNAV**
3. Save/check warns on claim/delta mismatch and new dep edges → **CONF** + **SCA**
4. Council review of the PR attests into the same trail → **CGBDG**
5. Team-lead dashboard glances the same evidence → consumer, not release claim

## 8. Non-goals

- Replacing or delaying the active provisional `v0.9.8-beta` field-intake
  window
- Enterprise constellation (POLFED / ORGHIER / CEWS / TRUST)
- WEAVE / in-process agent harness as part of this programme
- Auto-fix of violations
- Treating this note as a second release window in `RELEASE-PLAN.md`

## 9. Next operator decisions (only if Wave 1 is to be a release theme)

After Wave 0 clearance and at least two Wave 1 demos:

1. Name a later minor theme (candidate: "Graph Trust Surfaces" / "Agent Trust Loop")
2. Or keep the tracks as a standing side programme under NBI Schedule rows

Until then: **execute CGBDG discovery**; CONF-001 is integrated, and later work
promotes only through the §6 checklists. Do not force all five into one cut.
