# GCALL Caller-Egress Privacy Review — Verdict

**Date:** 2026-06-17
**Session:** `gcall-caller-egress-privacy-20260617` (caller-egress privacy
review, GCALL-007)
**Panel:** security-analyst (lead); adversarial, operations, and kernel
dimensions assessed against the inherited PV-9 CE conditions.
**Artifact under review:** the assistant-facing **caller-egress** surface —
`anvil_find_callers` (GCTX-014, not yet built) projecting the GCALL-003
`callers_of` read API (`crates/anvil-graph-cache/src/call_graph.rs`,
`CallersReport { callers: Vec<CallerResult { caller: SymbolIdentity, distance }>,
truncated }`) over resident `EdgeType::Calls` edges. Reviewed against the
[ADR-086](../decisions/086-symbol-call-graph-substrate.md) §4 caller-egress
posture, the [GCTX context-egress privacy review (PV-9)](2026-06-15-gctx-context-egress-privacy-review-verdict.md)
conditions CE-1..CE-12, and the already-approved sibling surfaces
`anvil_find_dependents` / `anvil_search_symbols`.
**Gate:** the GCALL-007 dependency of **GCTX-014** — `anvil_find_callers` flips
from Blocked to Ready only once this verdict is filed and its conditions are
folded into the GCTX-014 acceptance criteria (mirroring how PV-9's CE-1..CE-12
folded into the GCTX item text). This is a **design/contract** review; conditions
are verified at GCTX-014 implementation.

---

## Verdict

**APPROVE-WITH-CONDITIONS.** Exposing symbol→symbol call edges ("who calls this")
leaks **nothing beyond what PV-9 already approved** for `find_dependents` /
`search_symbols`. The caller surface is the same machine-local-equivalent,
identity-only structural projection one level finer than the file-level
dependency graph: it returns calling-symbol identity + traversal distance, never
source text, call-site arguments, byte spans, or session-local ids. The
finer (symbol-level) granularity is an **equivalent-risk** widening of an
already-approved identity class (PV-9 ALLOW d1/d2/e1 — names, kinds, relative
paths, edge topology, distances), not a new exposure class.

The single genuinely new requirement is **honesty about the graph's heuristic
incompleteness** (CALL-1) — a *safety* obligation, not a privacy leak: the call
graph is best-effort/static (import-derived, overload fan-out over-includes,
dynamic dispatch / default exports / non-resident targets dropped — ADR-086 §1),
so an assistant must not treat a `find_callers` result as an authoritative caller
set for a security-sensitive refactor. Everything else inherits the PV-9 CE spine
verbatim.

| Dim | Topic | Verdict |
|-----|-------|---------|
| K1 | Identity-only output; same approved class as `find_dependents` | APPROVE — no new field |
| K2 | Call graph vs dependency graph (finer granularity) | APPROVE — equivalent-risk, machine-local-equivalent |
| K3 | Heuristic/over-approximation honesty (false/missing edges) | APPROVE-WITH-CONDITIONS — **CALL-1, new + hard** |
| K4 | Sealed caller DTO + structural no-leak test | APPROVE-WITH-CONDITIONS — **CALL-2 (CE-5), hard item gate** |
| K5 | Volume bounds + opaque pagination (wide fan-out) | APPROVE-WITH-CONDITIONS — CALL-3 (CE-6) |
| K6 | Degradation + telemetry + kill-switch parity | APPROVE-WITH-CONDITIONS — CALL-4 (CE-7/10/11) |
| K7 | Call-site source-text egress stays gated | APPROVE — CALL-5 (CE-1/2/3), confirmatory |

---

## Conditions

Each: title · exposure addressed · concrete requirement · GCTX-014 fold ·
NEW (caller-specific) vs INHERITED (from PV-9).

### CALL-1 — Heuristic / partial honesty markers are mandatory on the caller surface — **NEW, hard**

- **Exposure.** The call graph over-includes (overload fan-out attaches an edge to
  every same-`(file, name)` candidate) and under-includes (dynamic dispatch,
  default-export callees, barrel/re-export callees, and non-resident targets
  produce no edge — ADR-086 §1). An assistant that trusts `find_callers` as the
  *complete, exact* caller set ("I've found every caller of `checkPermission`,
  safe to change its signature") can act on a false caller edge or miss a real
  one — concluding a security-sensitive change is safe when it is not.
- **Requirement.** The egress DTO MUST carry `heuristic: bool` (set when any
  returned caller derives from an overload fan-out) and `partial: bool` (set when
  the target — or any traversed file — had `Unresolved` call sites or hit the
  `MAX_OVERLOAD_FANOUT` / node-budget cap), per ADR-086 §1. The tool description
  MUST state the result is a **best-effort static import-derived
  over-approximation, not an authoritative caller set** — mirroring the
  `affected_tests` `heuristic: true` framing.
- **Implementation split (resolved post-verdict).** The two markers split by
  where their data lives. **`heuristic`** is carried by the substrate read API
  (`CallerResult.heuristic`, GCALL-003 follow-up) — fan-out is recoverable from
  the graph **conservatively** (a caller with `Calls` edges to ≥2 symbols sharing
  the called symbol's `(file, kind, name)`), which over-flags but never
  under-reports fan-out. **`partial`** is *not* graph-computable (an unresolved
  call produces no edge, invisible to the walk) and is therefore added at the
  **GCTX-014** egress layer from the daemon `all_calls` accumulator (which records
  every call site, resolved or not). So the substrate carries identity + distance
  + `heuristic` + `truncated`; GCTX-014 adds `partial`. (ADR-086 §1 records this.)
- **Folds into:** the GCALL-003 substrate follow-up (`heuristic`, landed) +
  GCTX-014 acceptance criteria (`partial` from the accumulator, the tool
  description, surfacing `heuristic`).

### CALL-2 — Sealed caller DTO + extend the structural no-leak test — **INHERITED (CE-5), hard item gate**

- **Exposure.** A new response type (`FindCallersProjection` / `CallerSummary`)
  could `serde(flatten)` a `SymbolNode` field (`trust_level`, the session-local
  `u64` id) or a future span.
- **Requirement.** The caller result is a sealed `anvil-gctx-types` DTO carrying
  only `SymbolIdentity` + `distance` (+ the CALL-1 markers + a counts-only
  `RedactionSummary`) — no `PathBuf`, no `u64` id, no source text. Extend the
  existing structural no-leak test battery (exact-key assertion, forbidden-concept
  walk `span/byte/text/body/snippet/trust/content/id`, absolute-path-value
  backstop) to the new caller response type. The MCP crate links only
  `anvil-gctx-types`.
- **Folds into:** GCTX-014 acceptance criteria (CE-5 bullet).

### CALL-3 — Caller-traversal volume bounds + opaque pagination — **INHERITED (CE-6)**

- **Exposure.** Symbol-level call graphs fan out wider than file-level imports; a
  hot utility (`log`, `assert`) can have thousands of transitive callers, enabling
  whole-graph enumeration via deep / paged calls.
- **Requirement.** Keep the substrate node budget (`MAX_CALLERS_WALK = 10_000`)
  and GV2-026 depth clamp (already enforced in `callers_of`); at the projection
  layer apply the inherited server-minted **opaque** pagination cursor +
  `MAX_PAGE_LIMIT` + query-param validation, reusing the GCTX-010/011 cursor
  machinery. Truncation is surfaced (`truncated`), never silent.
- **Folds into:** GCTX-014 acceptance criteria (CE-6 bullet).

### CALL-4 — Degradation + telemetry + kill-switch parity — **INHERITED (CE-7 / CE-10 / CE-11)**

- **Exposure.** A divergent caller outcome could leak via a non-enum telemetry
  label or fall back to source on a warming graph.
- **Requirement.** A `FindCallersOutcome` enum identical in shape to
  `FindDependentsOutcome` (`Ready` / `NotReady` / `Unavailable` / `Disabled` /
  `InvalidQuery`), a `telemetry_outcome() -> GctxOutcome` mapping (PII-free
  labels only — caller names never enter a label), and `ANVIL_GCTX_EGRESS=0`
  re-read per call. No source-file fallback on warming/stale; a cold call graph
  reports `not_ready` / `partial` (ADR-086 §3). CALL-1's `heuristic` is a
  **response field**, never a telemetry label.
- **Folds into:** GCTX-014 acceptance criteria (CE-7 bullet, extended to
  telemetry + kill-switch parity).

### CALL-5 — Call-site source-text egress stays gated and out of scope — **INHERITED (CE-1 / CE-2 / CE-3), confirmatory**

- **Exposure.** Future demand to return the call-site line, the
  argument/receiver expressions, or the caller body.
- **Requirement.** Any call-site or caller-body **source-text** egress is a
  Phase-2 escalation behind the PV-9 CE-1 `gctx.egress` opt-in (default-off) +
  CE-2 secret scan + CE-3 path filter, exactly like GCTX-021/022/023 snippets.
  Out of scope for GCTX-014; the caller DTO remains structurally text-incapable
  (`CallSite` carries only `from` / `callee` / `line` — no text — and the
  `anvil-gctx-types` crate is structurally incapable of naming source text).
- **Folds into:** GCTX-014 acceptance criteria (note: identity-only; call-site
  text is a CE-1-gated Phase-2 escalation).

---

## Findings (no action gate)

- **K1 — identity-only, no new field.** `callers_of` output is `CallerResult {
  caller: SymbolIdentity, distance: u32 }` + `truncated`. `SymbolIdentity` is the
  PV-9-approved d1/d2 class (`{ file (workspace-relative), kind, name, ordinal }`)
  whose ordinal is structural-only (PV-1) and which carries no session/worktree
  or provenance identity (PV-3). The session-local `u64` node id is used only in
  the internal BFS and converted to `SymbolIdentity` before any result is built.
  `distance` is the same field already approved on `DependentSummary`.
- **K2 — equivalent-risk granularity.** A symbol-level call edge adds the
  *relationship*, not new content: the calling symbol's identity is already
  egressible via `search_symbols`, and edge topology is a PV-9 ALLOW class. The
  "who-calls-the-auth-check" structural-intelligence concern is real but is the
  same machine-local-equivalent projection PV-9 blessed for `find_dependents`, and
  is information a reader of the (already-egressible) source obtains. PV-9 notes
  N-2 (content-hash correlation) and N-3 (version strings) do not apply —
  `callers_of` emits neither.
- The caller surface inherits the GCTX-010/011 spine verbatim (the
  `anvil-gctx-types` sealed-DTO crate, the `GctxProjector` choke point, the
  daemon-RPC path, the per-tool degradation/telemetry/kill-switch enums); no
  inherited CE condition is weakened by the granularity change.

---

## Gate disposition

GCALL-007's validation requirement — *"Verdict filed under `plans/reviews/`;
conditions reflected in GCTX-014"* — is satisfied by this verdict plus the
GCTX-014 acceptance-criteria edits it folds into. **CALL-1 and CALL-2 are hard
gates** that must be written into GCTX-014 item text before it flips from Blocked
to Ready. **CALL-1 additionally requires a GCALL-003 substrate follow-up** — add
`heuristic` / `partial` to `CallersReport` and thread the lift's fan-out /
unresolved provenance — since the merged read API does not yet implement the
ADR-086 §1 honesty markers it mandates. CALL-3/4/5 fold into GCTX-014 acceptance
criteria and are verified at implementation, reusing the GCTX-011 spine.

**Bottom line:** the caller surface is approvable on the same
machine-local-equivalent footing as `find_dependents`; the only genuinely new
requirement is honesty about the graph's heuristic incompleteness (CALL-1).

## Follow-up work

- **GCALL-003 substrate addition (CALL-1 prerequisite):** add `heuristic: bool`
  (per `CallerResult`, from overload fan-out) and `partial: bool` (per
  `CallersReport`, from unresolved/over-cap call sites) to the `callers_of` read
  API, threading the lift's fan-out/unresolved provenance. File as a GCALL-003
  follow-up; required before GCTX-014.
- **GCTX-014:** consumes `callers_of`, projects the sealed caller DTO with the
  CALL-1..CALL-5 conditions folded into its acceptance criteria (below).
