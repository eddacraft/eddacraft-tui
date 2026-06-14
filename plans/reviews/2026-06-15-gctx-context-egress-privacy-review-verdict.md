# GCTX Context-Egress Privacy Review — Council Verdict

**Date:** 2026-06-15
**Session:** `gctx-egress-privacy-20260615` (formal context-egress privacy
review council)
**Panel:** security-analyst (lead), adversarial-reviewer, operations-reviewer,
kernel-maintainer
**Artifact under review:** the assistant-facing graph-context **egress** surface
defined by the [`graph-context-delivery`](../modules/graph-context-delivery.aps.md)
(GCTX) module — the MCP tools (`anvil_search_symbols`, `anvil_find_callers`,
`anvil_find_dependents`, `anvil_impact_of_change`, `anvil_symbol_context`,
`anvil_affected_tests`), the `graph://symbols` / `graph://edges` / `graph://stats`
resources, and the context-slicing / snippet-extraction utilities — reviewed
against [ADR-083](../decisions/083-gctx-mcp-delivery-target.md) (delivery target),
the [2026-06-08 GV2 persistence privacy verdict](2026-06-08-gv2-privacy-review-verdict.md)
(PV-9), and the live graph types in `crates/anvil-kernel-types` /
`crates/anvil-graph-cache` / `crates/anvil-cli/src/mcp`.
**Gate:** the ADR-075 entry gate — *"context-egress privacy review (PV-9,
distinct from the persistence verdict)"* — a hard `v0.9` cut prerequisite for
the GCTX module and the MCP/weave-egress-adjacent GV2-020/-023 consumer surface.
GCTX is 0/13 (all Draft): this is a **design/contract** review whose conditions
fold into GCTX item text before implementation, exactly as the 2026-06-08 review
folded PV-1..PV-12 into GV2-002/GV2-030.

---

## Verdict

**APPROVE-WITH-CONDITIONS — unanimous (4/4), no BLOCKs.** The egress surface is
approvable as a `v0.9` design provided it is built from the **inverse-default**
of the persistence posture. The load-bearing framing decision, which every panel
member independently converged on:

> **The default GCTX egress surface is an identity-only structural projection —
> symbol names, kinds, workspace-root-relative paths, edges, distances, and
> structural summaries (the persistence verdict's ALLOW classes d1/d2/e1). It
> inherits the 2026-06-08 verdict's machine-local-equivalent safety. Returning
> actual source *text* (snippets) is a deliberate, gated escalation: opt-in,
> default-off behind a flag-catalogue entry, scanned for secrets, path-filtered,
> and emitted only through a single sealed-DTO redaction choke point.**

This matters because GCTX inverts both halves of what made persistence safe. The
persistence DTO is a sealed allowlist that **never holds source text** (spans are
no-text `ByteRange`s; `PolicyEvidence::resolve()` re-reads source on demand inside
the same-uid 0700/0600 boundary), and it **never leaves the machine**. GCTX-021/
022/023 deliberately re-hydrate spans into source text **and** carry it across the
machine boundary to a remote assistant/LLM provider. That is precisely the surface
PV-9 reserved, and it cannot silently inherit ADR-069's residual-risk acceptance.
The module's current privacy posture is a single prose sentence (Constraints:
*"Sensitive diagnostics, secret content, and private provenance fields must be
redacted by default before crossing MCP boundaries"*) — a redaction-as-convention
promise, not a deny-by-default mechanism. The conditions below convert it into
mechanism, mirroring the persistence verdict's central lesson: a sealed-allowlist
DTO with a structural no-leak test, **not a review convention**.

| Dim | Topic | Lead | Verdict |
|-----|-------|------|---------|
| E1 | Source-text (snippet) egress default posture | security-analyst | APPROVE-WITH-CONDITIONS — **CE-1 hard Ready gate** |
| E2 | Secret-shaped content + sensitive-path filtering | security / adversarial | APPROVE-WITH-CONDITIONS (CE-2, CE-3) |
| E3 | Egress field allowlist (no GV2-013/014 fields ahead of their ADRs) | security / kernel | APPROVE-WITH-CONDITIONS (CE-4) |
| E4 | Sealed egress DTO + single choke point + no-leak test | kernel-maintainer | APPROVE-WITH-CONDITIONS — **CE-5 hard item gate** |
| E5 | Volume bounds (quotas, pagination, budget bypass, input validation) | adversarial-reviewer | APPROVE-WITH-CONDITIONS (CE-6) |
| E6 | Stale-graph snippet hazard + degradation posture | adversarial / operations | APPROVE-WITH-CONDITIONS (CE-7) |
| E7 | Session-pinned root + stdio-only transport boundary | adversarial / operations | APPROVE-WITH-CONDITIONS (CE-8) |
| E8 | Flag catalogue, telemetry, kill-switch, observability, consent | operations / security | APPROVE-WITH-CONDITIONS (CE-9..CE-12) |

---

## Conditions

Each condition: title · exposure addressed · concrete requirement · fold target.
CE-1 and CE-5 are hard gates (see *Gate disposition*); the rest fold into GCTX
item text and are verified at implementation.

### CE-1 — Snippet egress is opt-in; identity-only is the default — **HARD Ready gate**

- **Exposure:** GCTX-021 (snippet extractor), GCTX-022 (budget slicer), GCTX-023
  (`anvil_symbol_context`) return actual **source text** to a remote assistant/LLM
  provider. The In-Scope framing treats snippets as the headline feature, implying
  default-on; the persistence DTO never carried text.
- **Requirement:** The default posture of every GCTX tool and `graph://` resource
  is **identity-only** (the persistence ALLOW classes d1/d2/e1 — names, kinds,
  relative paths, edges, distances, structural summaries). Returning source text
  requires (a) an explicit named opt-in flag (`gctx.egress`, see CE-9),
  `defaultVariant=disabled`, **and** (b) a per-request capability the client must
  assert. With the opt-in off, GCTX-021/022/023 return spans-as-locations (`file`
  relative path + start/end line + no-text `ByteRange`), never text. The default
  surface thereby inherits the persistence verdict's safety; text is a deliberate,
  gated escalation.
- **Folds into:** GCTX-001 (contract: default posture + opt-in flag) and
  GCTX-021/022/023 (each: *"text only when snippet-egress opt-in asserted;
  identity-only otherwise"*).

### CE-2 — Deny-by-default secret scanning on every emitted snippet

- **Exposure:** Snippets can contain `.env` lines, inline credentials, API keys,
  PEM private-key blocks, connection strings. The persistence DTO excludes this by
  construction (PV-1 named `foo(key = "sk-live-…")` as a literal-value leak class);
  GCTX re-introduces it by returning bodies.
- **Requirement:** Before any snippet text crosses the MCP boundary it passes a
  **deny-by-default** secret detector (reuse `anvil-intercept-rules`
  `SecretDetectionRule`) over the **emitted text** (not the source file, so partial
  slices are covered). On a hit the span is redacted (placeholder marker) and
  `redacted: true` + a count is set in response metadata. **Fail-closed**: a
  detector error redacts the span, never emits it. Redaction is **deterministic**
  (same input → same redacted output) so GCTX-022's determinism property tests do
  not flake.
- **Folds into:** GCTX-021 (detector hook), GCTX-022 (redact-before-budget so a
  redacted span still counts honestly toward the budget).

### CE-3 — Sensitive-path / gitignore-aware egress filter (deny-by-default)

- **Exposure:** The graph substrate GCTX reads from does **not** exclude
  gitignored/dotfile content — only the welcome scan honours `.gitignore`; all
  other scans use `standard_filters(false)` by design. GCTX is the first surface
  where that content egresses.
- **Requirement:** GCTX applies an egress-side path filter independent of the
  substrate filters: deny by default `.env*`, `*.pem` / `*.key` / `id_rsa*` /
  `*.p12`, `.git/**`, and common secret directories (`secrets/`, `.aws/`, `.ssh/`,
  `.gnupg/`); and for snippet egress specifically, **gitignored files**. A denied
  path is omitted from results entirely (not merely snippet-redacted), with an
  `omitted_sensitive_paths` count in metadata. The deny-list is a named, reviewable
  default, widened only by explicit allow, never silently.
- **Folds into:** GCTX-001 (the named deny-list + the *"only welcome scan honours
  .gitignore"* cross-reference) and GCTX-010/021/030 (enumeration + snippets).

### CE-4 — Egress field allowlist = persistence ALLOW set; DENY GV2-013/014 fields until their ADRs

- **Exposure:** GCTX tools/resources project graph fields. GV2-013 (control/
  session) and GV2-014 (plan/provenance) each require their **own** privacy ADR
  before persisting and sit **outside** the same-uid acceptance. GCTX must not
  become the back door that egresses those fields ahead of their gating.
- **Requirement:** GCTX projects **only** the persistence ALLOW set (symbol names/
  kinds, relative paths, edges, structural hashes, visibility/side-effect classes).
  Explicitly **DENY** at the projection boundary: session/worktree identity,
  `claimed_agent_id`, usernames, hostnames, absolute paths, `pid`/`pgid`/liveness,
  raw `GraphDelta.errors`, and any APS/commit/Edda provenance ref **until
  GV2-013/GV2-014 ship their privacy ADRs**. `SymbolNode.trust_level` requires an
  explicit include/exclude decision: default **exclude**, or map to a documented
  client-facing alias (never raw `TrustLevel` passthrough). GCTX-001 carries a
  **named egress residual table** (analogous to the GV2-030 per-field-class table)
  covering symbol names, paths, import specifiers (incl. version strings), edge
  topology, and content hashes as ALLOW / ALLOW-T / DENY for the cross-boundary
  case.
- **Folds into:** GCTX-001 (allowlist + residual table) and GCTX-010/012/013/030.

### CE-5 — Sealed egress DTO, single choke point, structural no-leak test — **HARD item gate (GCTX-001 → Phase 1)**

- **Exposure:** Constraints is prose, not mechanism. `SymbolNode` derives
  `Serialize` and carries `id: u64` (session-local) and `trust_level`;
  `GraphDelta.errors: Vec<String>` and `previously_*` sets embed `file::kind::name`.
  Any of these can serde-flatten into a response with zero friction.
- **Requirement:** A **sealed egress projection DTO** distinct from all internal
  graph types — no `serde(flatten)`, no `PathBuf`-typed field, no internal-type
  passthrough, no session-local `u64` node/edge id (identify symbols by
  `SymbolIdentity` or `relative-path + name`). A **single** projection boundary
  (`GctxProjector` constructor) is the only path that builds egress DTOs; handlers
  never construct them directly. A `SnippetResult` (file relative + `ByteRange` +
  language + `text` + `truncated` + `omitted_bytes`) is the **only** source-text
  carrier; every other type is structurally incapable of holding source text or a
  free-form error string. Error responses are a named enum (`GctxError { NotReady,
  SymbolNotFound, BudgetExceeded, GraphUnavailable, … }`), never `message: String`.
  Collections are sorted by a stable key (`SymbolIdentity` natural order) before
  serialisation. A **structural no-leak test** (mirror PV-7) asserts: no `PathBuf`
  field; every path-bearing string is workspace-root-relative (no `/` or drive
  prefix); `GraphDelta` does not compile into the egress module; the only banned-
  name text field (`text`/`body`/`content`/`source`/`snippet`/`raw`) is
  `SnippetResult.text` behind CE-1. The test lives in the egress DTO crate and
  gates GCTX-010.
- **Folds into:** GCTX-001 (defines the DTO module, projector, and no-leak test
  scaffold) and each tool/resource item's Validation line.

### CE-6 — Volume bounds: quotas, opaque pagination, budget-bypass, input validation

- **Exposure:** Without bounds, an assistant can page `graph://symbols` /
  `graph://edges` to dump the whole graph, chain traversals to the transitive
  closure, reassemble whole files from overlapping snippet calls, or abuse query
  params (glob/regex, path traversal, scheme-prefixed specifiers).
- **Requirement:** (a) hard per-session byte cap on `graph://` resource reads + a
  max page size; pagination cursors are **server-minted opaque tokens**, not
  client-supplied offsets. (b) per-call result-set size caps on traversal tools
  and a per-session traversal credit so call-by-call chaining cannot route around
  them; an input file-count cap on `anvil_impact_of_change` (≈≤200). (c) a
  per-session snippet **byte** ceiling independent of the per-call token budget
  (defeats overlapping-span reassembly); budget keys on `(file, ByteRange)`
  position identity, never on text content. (d) query-param validation: exact match
  or a named length-bounded glob (no arbitrary regex), ≤512 bytes/param, reject NUL
  / path-separator traversal / scheme-prefixed (`npm:`, `https:`, `data:`) inputs
  with a structured error before the graph is queried. (e) `anvil_impact_of_change`
  uses the diff input for **changed file paths only** — it never reads or forwards
  diff content; content goes through the graph + snippet + redaction pipeline.
- **Folds into:** GCTX-001 and GCTX-010/011/012/022/030.

### CE-7 — Stale-graph snippet guard; no whole-file fallback on warming

- **Exposure:** The graph cache may lag source by a rebuild cycle. A credential-
  shaped symbol scrubbed from source can persist in the cache; a snippet query in
  the stale window egresses it. A naive warming fallback to raw file reads is worse
  than the graph being off.
- **Requirement:** Snippet-returning responses re-validate the file's current
  content hash against the graph's recorded hash before emitting (or return a
  structured `stale-graph` error) when the graph exceeds a stated staleness bound;
  identity-only responses may continue under the existing warming/stale
  degradation. When the graph is `warming` / `stale` / `disabled`, all tools and
  resources return a structured degraded response (status + reason enum) with empty/
  omitted results — **never** a fallback to `std::fs::read_to_string` or any path
  returning file content outside the graph → redaction → budget pipeline. A fixture
  exercises the warming path and asserts the snippet field is absent/empty.
- **Folds into:** GCTX-001, GCTX-021, GCTX-023.

### CE-8 — Session-pinned workspace root + stdio-only transport boundary

- **Exposure:** A shared `ANVIL_HOME` / multi-graph daemon could cross worktree
  boundaries (Project A's assistant learns Project B's structure). A future
  networked RMCPF transport would silently widen the egress boundary.
- **Requirement:** Every GCTX call is scoped to a single, session-pinned workspace
  root validated at session init against the MCP server's launch root (reuse the
  `shared.rs` `validate_workspace_root` pattern); cross-worktree queries are
  rejected; until GV2-020 provides per-graph isolation, GCTX fails closed if the
  registry is absent or returns multiple candidates for a root. GCTX tools/
  resources are authorised over the **local stdio transport only**
  (`AnvilEntry::Stdio`, `anvil mcp serve --stdio`). Any RMCPF transport that
  crosses a network or cross-uid boundary (`RemoteSse`, `RemoteHttp`, TCP, cross-uid
  pipe) requires a **new** context-egress privacy review before GCTX tools register
  there. Tracked as a GCTX-002 acceptance criterion.
- **Folds into:** GCTX-001 and GCTX-002.

### CE-9 — Default-off flag in `flags/manifest.json` (PV-11 mirror)

- **Exposure:** The egress surface must be operator-controlled at launch, exactly
  as `daemon.persist-graph` entered the catalogue at `defaultVariant: disabled`.
- **Requirement:** Add a `gctx.egress` entry to `flags/manifest.json`
  (`class: rollout`, `valueType: boolean`, `defaultVariant: disabled`,
  `createdFor: GCTX-001`, operator opt-in via `ANVIL_GCTX_EGRESS=1`). To avoid a
  FLAGCAT **orphan-flag drift** failure (the gate requires a TS+Rust consumer), the
  entry lands in the **same PR that first touches a GCTX implementation file**, not
  ahead of it — this verdict records the required entry; it is not added in the
  plan-only change that files this review.
- **Folds into:** GCTX-001 and `flags/manifest.json` (at first implementation).

### CE-10 — Telemetry labels are enum outcomes only (PV-10 mirror)

- **Exposure:** Egress query inputs and snippet content are user-authored source;
  a metric label that echoes them is a second leak.
- **Requirement:** Every counter / span attribute / notification emitted by GCTX
  binds to an exhaustive outcome enum (`hit`, `miss`, `warming`, `graph_disabled`,
  `budget_exceeded`, `redacted`, `error`, …). No label carries symbol names, paths
  (relative or absolute), query text, snippet text, or per-symbol token counts;
  only response-aggregate token totals are permitted, in spans. Applies equally to
  the ADR-035 tracing pipe and any future Kindling observation.
- **Folds into:** GCTX-001 and `docs/architecture/graph-context-delivery-spec.md`.

### CE-11 — Kill-switch + redaction observability

- **Exposure:** Operators need a synchronous off-switch and a way to confirm
  redaction ran without that confirmation becoming its own leak.
- **Requirement:** `ANVIL_GCTX_EGRESS=0` disables egress on the **next call**
  (flag re-read per invocation, not cached at start-up; RMCPF per-tool capability
  gate). Each response carries a top-level `redaction_summary` of **counts +
  outcome enum only** (`fields_suppressed`, `snippets_truncated`,
  `fully_suppressed_symbols`, `outcome`) — no field names, symbol names, or path
  fragments. The tracing pipe may carry the same counts; never the suppressed
  content.
- **Folds into:** GCTX-001, GCTX-023, GCTX-032 (operator runbook).

### CE-12 — Consent: the boundary crossing is surfaced, not silent

- **Exposure:** The user may not realise enabling snippet egress ships source text
  to a third-party provider. The persistence boundary was implicit-and-safe
  (machine-local); the egress boundary is the opposite and must be explicit (house
  rule: operator config has no silent defaults).
- **Requirement:** Enabling snippet egress (CE-1 opt-in) is an explicit operator
  action with a one-line consequence statement (*"source text from matched symbols,
  secret-scanned and path-filtered, will be sent to the connected assistant/LLM
  provider"*). No GCTX tool auto-enables snippets on first use; the unset-flag
  fallback is identity-only, not text. GCTX-032 documents exactly what crosses the
  boundary, the identity-only default, the redaction/filter behaviour, and how to
  disable.
- **Folds into:** GCTX-032 and GCTX-001.

---

## Notes (no action gate)

- **N-1 — `graph://stats` is the lowest-risk egress; lead with it.** Aggregate
  counts (symbol/edge totals, warming/stale timestamp) carry no identity and make a
  good default-on warm-up target for the CE-11 redaction-summary machinery before
  the snippet-bearing tools land. `graph://symbols` / `graph://edges` need CE-3/CE-4
  filtering and CE-6 caps.
- **N-2 — Content-hash correlation residual now crosses the boundary for real.**
  PV-12 named unsalted SHA-256 content hashes as cross-machine-correlatable (git-
  blob class) and reserved re-evaluation "before any cross-machine surface" — that
  caveat (table row e1) is now triggered. Acceptable (same class as commit SHAs
  already in git) but named in the GCTX-001 residual note.
- **N-3 — Import-specifier version strings advertise exact dependency versions.**
  `ImportEdge.to_source` for external packages (e.g. `lodash@4.17.21/fp`) is an
  ALLOW-class identity string but reveals dependency versions to the assistant;
  name it in the GCTX-001 residual note, do not silently include.
- **N-4 — The remote provider is the real trust boundary, not the MCP transport.**
  Stdio is local, but the assistant client forwards context to its model provider.
  CE-1/CE-2/CE-3 are correct because they treat the client as untrusted egress, not
  because of transport security.
- **N-5 — The ADR-083 RMCPF single-surface decision helps.** One Rust
  `anvil mcp serve` surface means one redaction/allowlist choke point (CE-5), not
  two; reuse the existing `shared.rs` workspace-root containment, symlink rejection,
  and entry-count cap rather than reimplementing them (CE-8).
- **N-6 — No implementation exists yet.** All GCTX source paths are future Draft
  items; the scenarios above are design-time projections, not observed
  vulnerabilities. The conditions are pre-implementation guards.

---

## Gate disposition

The ADR-075 entry gate — *"context-egress privacy review (PV-9)"* — is
**satisfied** by this verdict. The completed answer: the default GCTX egress
surface is an identity-only structural projection inheriting the 2026-06-08
persistence safety, and source-text egress is a gated escalation governed by
CE-1..CE-12. These conditions fold into GCTX-001 (contract) item text and the
named per-item targets; the GCTX module's Ready-checklist line *"Redaction rules
for graph context are reviewed by security"* flips with this review.

Two conditions are **hard gates**, not merely foldable:

- **CE-1** (snippet egress opt-in, identity-only default + the flag) must be
  written into GCTX-001 + GCTX-021/022/023 item text before those items flip to
  Ready — it is the literal subject of PV-9 and determines whether GCTX inherits or
  inverts the persistence verdict's safety.
- **CE-5** (sealed egress DTO + single choke point + structural no-leak test) is a
  compile-time artefact requirement on GCTX-001 that gates the Phase 1 tool items;
  without it every downstream item re-litigates redaction independently.

With [ADR-083](../decisions/083-gctx-mcp-delivery-target.md) **Accepted**
(2026-06-15) and this review filed, both ADR-075 entry decisions are landed:
GCTX-002 becomes **Ready** and the GCTX module opens (Draft → Ready, 0/13 — the
remaining items stay Draft pending the GCTX-001 contract that folds CE-1..CE-12).
The MCP/weave-egress-adjacent **GV2-020** and **GV2-023** consumer items are
promoted to **Ready** (their substrate dependencies GV2-010..014 are Merged). The
`flags/manifest.json` `gctx.egress` entry (CE-9) lands with the first GCTX
implementation to avoid a FLAGCAT orphan-flag drift failure.
