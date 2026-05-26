# `anvil bom` — Surface Triage

**Date:** 2026-05-26
**Status:** Brainstorm — triage outcome. **Decision: do not file an `anvil
bom` APS module now. Of five candidate slices, three survive the scope-guard
(agents, policy refs, witness/protection summary) and only as a read-only
_view_ over existing collectors; two are rejected (MCP servers, credential
refs); one defers to AGOV-007 (controlled actions). The shape that earns its
place is a view + `--diff` drift gate, slotted under AGOV when that module
leaves the launch parking lot. CIB-015 closes with this recorded defer.**
**Source:** [CIB-015](../modules/continuous-improvement-backlog.aps.md#cib-015-triage-anvil-bom-surface-before-filing-as-aps),
authorised by the [Drako borrow assessment](./2026-05-24-drako-borrow-assessment.md)
§4 Borrow B + §8 open questions.

---

## 1. The question

The Drako assessment took `anvil bom` as a _framing_ borrow needing its own
scope-guard pass before any APS module is filed (assessment §7.2 Borrow B). The
proposed surface joins five slices into one command:

```
anvil bom            # human-readable summary
anvil bom --json     # machine-readable BOM for downstream tools
anvil bom --diff     # change since last BOM (drift)
```

Slices: **agents / MCP servers / policy refs / credential refs / controlled
actions.** This triage runs each through the scope-guard, names a slot for the
survivors, and answers §8's open question: is the BOM a _view over the witness
chain_ or a _separate collector_?

## 2. Method — the scope-guard test

Per `docs/vision/anvil-scope-guard.md` and the assessment §4 caveat, a BOM slice
earns its place in Anvil **only if it feeds enforcement** (policy assertions
against the BOM, witness-chain enrichment, or drift signals). _"If it's only a
pretty inventory, it belongs in a separate tool."_ Pure informational
aggregation is the failure mode §6 flags as scope creep into generic
asset-management.

## 3. Surface reality today

What each slice already has, with the production wire-up verdict (traced from
command run-paths, not spec text):

| Slice | Surface today | Evidence | Wired? |
| --- | --- | --- | --- |
| Agents | `AgentInventory` (5-tool detect), cached to `.anvil/cache/detected-agents.json`; rendered by `anvil start` / `anvil status` | `crates/anvil-cli/src/activation/detect_agents.rs:113` (ADOPT-003 Merged) | **Yes** |
| Policy refs | `anvil policy list` → `PolicyEntry{id,name,category,enabled,…}`; declared bundles via `list_bundles()` over `.anvil/bundles/` | `crates/anvil-cli/src/commands/policy/mod.rs`; `crates/anvil-policy/src/bundle.rs:8` | **Yes** |
| Witness / protection | `chain.ndjson` summary + `ProtectionClaim{worktree_state, surfaces[]}`; `anvil audit-chain` | `crates/anvil-witness/src/line.rs:18`; `crates/anvil-kernel-types/src/protection_claim.rs:237` | **Yes** |
| MCP servers | only config **generation** for Claude Code + Cursor — no discovery / inventory | `crates/anvil-cli/src/commands/mcp_config.rs` | Config-gen only |
| Credential refs | none — secret _scanning_ exists in the gate; a credential _reference registry_ does not | (AGOV-005 PR-metadata cred/PII scan is Draft, different scope) | **No surface** |
| Controlled actions | git hooks are live, but the declarative "what may this agent touch" model is AGOV-007 capability declaration | `plans/modules/agent-governance-patterns.aps.md:248` (Draft, Tier C) | **No (planned)** |

## 4. Scope-guard pass, per slice

| Slice | Feeds enforcement / enrichment? | Verdict |
| --- | --- | --- |
| **Agents** | Enrichment. The attribution machinery exists (`anvil-attribution` crate, `AgentTag` minting, env propagation, IPC carrying `agent_tag`), but the **witness line's `agent_tag` is not reliably persisted** — `audit_chain` constructs it as `None` (`audit_chain.rs:625,800`). So today the agent slice is an _inventory_, with enrichment latent. | **Survive** — as a view-slice reading the detected-agents cache, not the chain. |
| **Policy refs** | Enforcement — policies _are_ the enforcement surface, already first-class via `anvil policy list`. | **Survive** — BOM references it; adds no collector. |
| **Witness / protection** | Enforcement / provenance — this _is_ the evidence chain. A structural summary (line count, date range, state distribution, tamper/gaps, current `worktree_state`) is enforcement-coupled. | **Survive** — as a summary view-slice. |
| **MCP servers** | Nothing to feed: only config _generation_ exists, no inventory/discovery. Surfacing it requires a **new collector** — which is "build the surface," explicitly out of scope for this triage, and the broad-inventory direction §6 warns against. | **Reject** (revisit only if MCP discovery is later driven by a concrete enforcement need). |
| **Credential refs** | No surface exists. A "which credentials/secret-stores does this agent reference" registry would be a **new, security-sensitive collector** — exactly the generic-asset-management drift §6 flags. The enforcement-relevant part (secrets in artefacts) is already covered by gate secret-scanning. | **Reject.** |
| **Controlled actions** | The meaningful version is the **AGOV-007 capability declaration model** (declare paths/operations an agent may perform; gate against it). That is enforcement-aligned but **unbuilt** (Draft). The live hooks are the control, not a BOM inventory. | **Defer to AGOV-007.** |

**Result:** 3 survive (agents, policy refs, witness/protection summary), all as
_view_ slices over collectors that already exist and are production-wired; 2
reject (MCP, credentials — both need new collectors); 1 defers to AGOV-007.

## 5. What earns it its place vs. "a pretty inventory"

A bare `anvil bom` that reprints what `anvil policy list` + `anvil start` +
`anvil audit-chain` already print is the "pretty inventory" the scope-guard
rejects. The slice that earns the command its place is **`--diff` drift**:
baseline the agent + policy + surface set, then **warn on new edges** (a new
agent appeared, a policy was disabled, a bundle changed) since baseline. That
maps directly onto Anvil's stated principles — _"new edges only,"_ _"warnings
over blocks,"_ _deterministic_ — and turns the BOM from a report into an
enforcement signal. **The drift gate is the justification; the inventory alone
is not.**

## 6. View vs. collector (§8 answer)

**A view/aggregator — not a new collector, and not (yet) a view over the witness
chain.** v1 reads existing collectors:

- agents ← `.anvil/cache/detected-agents.json`
- policies ← policy catalogue + `.anvil/bundles/*/manifest.json`
- witness/protection ← `chain.ndjson` structural summary + `ProtectionClaim`

It must introduce **no new detectors** (this is what keeps the borrow thin, per
assessment §4). The "view over the witness chain" option in §8 is **not
available for the agent slice today**: `ProtectionClaim` carries no agent
attribution (opaque surface identifiers only), and the witness line's
`agent_tag` is not reliably populated. Revisit chain-sourcing the agent slice
only once witness lines persist `agent_tag` end-to-end; until then the cache is
the source of truth and the chain contributes a structural summary only.

## 7. Slot

- **Not a new module.** It is a thin view + one gate; a dedicated module would
  over-weight it.
- **Slot under AGOV** (`agent-governance-patterns.aps.md`) as a new
  `AGOV-NNN` view+drift item — it is fundamentally an _agent-surface_ view.
- **Not AGOV-007.** Capability _declaration_ (what an agent may do) is a
  different, enforcement construct; the BOM _observes_ the surface. They
  coordinate but are not the same item.
- **ADOPT is Complete/archived, not a home.** The assessment's "likely
  ADOPT-007" guess points at `plans/archive/modules/adoption-friction.aps.md`
  (ADOPT-001..006, `Complete 6/6`) — the namespace exists but is closed
  (ADOPT-003 is the AI-tool auto-detect this BOM reads from), so new work
  shouldn't be filed there. AGOV is the active home.
- AGOV is currently **Tier C (post-launch parking lot, low confidence)**, so
  filing a concrete item now would be premature precision.

## 8. Triage outcome (closes CIB-015)

**Decline to file an `anvil bom` APS item now.** Record the surviving shape so a
future filing is a scoped follow-up, not an open question:

- **Shape:** read-only view over agents (cache) + policy/bundle catalogue +
  witness/protection structural summary, plus a `--diff` drift gate as the
  enforcement hook. No new detectors.
- **Rejected:** MCP-server inventory and credential-reference registry (both
  need new, scope-creeping collectors; credentials additionally sensitive).
- **Deferred:** controlled-actions → AGOV-007 capability model when it ships.
- **Slot when filed:** new `AGOV-NNN` under `agent-governance-patterns.aps.md`.
- **Trigger to file:** AGOV leaves the launch parking lot **and** there is a
  concrete consumer for `--json`/drift (e.g. a CI surface or a buyer asking to
  baseline the agent surface). Chain-sourcing the agent slice waits on reliable
  `agent_tag` persistence on the witness line.
- **Citation:** cite Drako as parallel evolution, not prior art (assessment §6).
  Rung 6 (runtime enforcement) stays declined per the Proxilion/PIC precedent.

## 9. One-line summary

> `anvil bom` survives as a thin read-only view (agents + policies +
> witness/protection summary) whose enforcement justification is a `--diff`
> drift gate, not the inventory; MCP and credential slices are rejected as
> scope-creeping new collectors, controlled-actions defers to AGOV-007, and the
> whole thing waits for a concrete `AGOV-NNN` consumer before it is filed.
