# Graph Context Delivery — Architecture Spec

| Type | Authority | Owner                                                                                                                     | Status | Freshness                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| ---- | --------- | ------------------------------------------------------------------------------------------------------------------------- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Spec | Derived   | GCTX ([`plans/archive/modules/graph-context-delivery.aps.md`](../../plans/archive/modules/graph-context-delivery.aps.md)) | Live   | Last reviewed 2026-09-09 for GATT-001 / [ADR-142](../../plans/decisions/142-graph-answer-attestation.md) self-attestation: every projection carries a shared counts-and-enums `Attestation` block (tagged totals, per-edge fidelity, cap disclosure, cost self-report). No GV2 schema, no source-text egress change, no ranking field. Authored 2026-06-15 (GCTX-001 projection contract). Folds the context-egress privacy review (PV-9, CE-1..CE-12, APPROVE-WITH-CONDITIONS 4/4) onto the GV2-023 consumer query contract. Projection rules only — schemas, deltas, and hot-path admission stay owned by GV2 (ADR-061/063/064/067/069). |

| Upstream                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | Downstream                                                                                                                                                                                                                                                                      |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [`graph-v2-foundation-spec.md`](./graph-v2-foundation-spec.md) (GV2-023 consumer query contract), [context-egress privacy review (PV-9)](../../plans/reviews/2026-06-15-gctx-context-egress-privacy-review-verdict.md), [ADR-083](../../plans/decisions/083-gctx-mcp-delivery-target.md), [ADR-075](../../plans/decisions/075-v080-graph-product-scope.md), [ADR-142](../../plans/decisions/142-graph-answer-attestation.md) (self-attestation), `crates/anvil-intercept-rules`, `crates/anvil-gctx-types`, `crates/anvil-graph-cache` | GCTX-010/011/012/013 (query tools), GCTX-021/022/023 (snippet + slicing), GCTX-030 (`graph://` resources), GCTX-031 (benchmarks), GCTX-032 (user guide), GATT-001..006 ([graph-answer-attestation](../../plans/modules/graph-answer-attestation.aps.md)), `flags/manifest.json` |

> **Status (2026-09-09): Phase 1–3 all Merged; GATT-001 amends the frozen
> contract.** The GCTX module is **15/15 Merged** — GCTX-010 (#2657) through
> GCTX-024 (#2980), the full tool/resource surface plus the consented
> snippet-egress opt-in — see the GCTX row in
> [`plans/index.aps.md`](../../plans/index.aps.md). The graph-handle and
> executor decisions this contract sits on are settled:
> [ADR-084](../../plans/decisions/084-gctx-graph-handle-access.md),
> [ADR-085](../../plans/decisions/085-daemon-full-scan-executor.md), and
> [ADR-086](../../plans/decisions/086-symbol-call-graph-substrate.md) are all
> **Accepted**. This document remains the **frozen delivery contract** the
> implementation tracks, not a forward-looking proposal.
>
> [ADR-142](../../plans/decisions/142-graph-answer-attestation.md) (**Accepted**
> 2026-09-07) adds the self-attestation contract below: every projection states
> its own bounds in-band. GATT-002..005 implement that contract in
> `anvil-gctx-types` and `anvil-graph-cache`; this amendment freezes the shape
> they must ship.
>
> [ADR-095](../../plans/decisions/095-gctx-cli-secondary-surface.md)
> (**Accepted** 2026-07-02) adds a co-equal **CLI secondary** read surface over
> the same `anvil/gctx/*` RPC spine — MCP-first, the CLI a thin client for
> out-of-session consumers, not a runtime fallback. Attestation is defined on
> the projection, so both surfaces render the same block.

## Purpose and scope

This is the **delivery contract** for the assistant-facing graph-context surface
(GCTX). It answers one question the
[GV2-023 consumer query contract](./graph-v2-foundation-spec.md#the-consumer-query-contract-gv2-023)
deliberately left open: _given the trusted joined graph, which projections are
safe and useful to hand to an untrusted AI coding assistant, and through what
mechanism?_

GV2 fixes the substrate; GV2-023 fixes that GCTX reads it as a **context
projection** through a single `GctxProjector` choke point. This spec fixes the
projection rules themselves — default posture, redaction, the sealed egress DTO,
volume bounds, self-attestation of those bounds (ADR-142), degradation, and the
transport boundary — by absorbing the twelve conditions (CE-1..CE-12) of the
[context-egress privacy review (PV-9)](../../plans/reviews/2026-06-15-gctx-context-egress-privacy-review-verdict.md).

**In scope:** the egress trust boundary and default posture; the sealed egress
DTO and single projection choke point; the egress field allowlist and residual
table; snippet-egress opt-in, secret scanning, and path filtering; volume
bounds; **self-attestation of those bounds** (ADR-142 — what the answer could
not resolve or could not fit, as a shared counts-and-enums block on every
projection); stale-graph and degradation behaviour; the transport and
workspace-root boundary; the flag, telemetry, kill-switch, and consent surface;
and the per-tool / per-resource projection map that the Phase 1–3 items
implement.

**Out of scope (owned elsewhere, by design):**

- Graph schemas, stable identity, deltas, hot-path indexes, and persistence —
  owned by GV2; this spec **defines no schema and no hot-path enforcement
  contract** (GCTX-001 Validation).
- The registry and trait implementation — GV2-020.
- Launch-critical pre-write validation — RMCP.
- Full MCP parity / TS-server retirement — RMCPF.
- Token-budget strategy detail (GCTX-020/022), benchmark fixtures (GCTX-031),
  and user-guide prose (GCTX-032) — named here, detailed in their items.

**Framing rule (from the module).** Graph v2 is Anvil-first. If assistant
context delivery and enforcement/provenance requirements conflict, **GV2 wins
and this projection adapts.**

## The egress trust boundary — the load-bearing inversion

GCTX is **not** a relaxation of the GV2 persistence posture; it is its inverse,
and must be built from the inverse default. The
[2026-06-08 persistence verdict](../../plans/reviews/2026-06-08-gv2-privacy-review-verdict.md)
was safe for two structural reasons, and GCTX breaks both:

1. **The persistence DTO never holds source text.** Spans are no-text
   `ByteRange`s; `PolicyEvidence::resolve()` re-reads source on demand _inside_
   the same-uid `0700`/`0600` boundary. GCTX-021/022/023 deliberately re-hydrate
   spans into **source text**.
2. **The persisted state never leaves the machine.** GCTX carries its projection
   **across the machine boundary** to a remote assistant, which forwards it to a
   model provider.

The real trust boundary is therefore **the remote provider, not the MCP
transport** (PV-9 N-4). Stdio is local, but the client forwards context onward;
the conditions below treat the client as untrusted egress regardless of
transport security. GCTX cannot silently inherit ADR-069's same-uid
residual-risk acceptance — that acceptance was conditioned on machine-locality,
which no longer holds.

The corrective, which every PV-9 panel member converged on independently:

> The default GCTX egress surface is an **identity-only structural projection**
> — symbol names, kinds, workspace-root-relative paths, edges, distances, and
> structural summaries (the persistence ALLOW classes d1/d2/e1). Returning
> actual source **text** is a deliberate, gated escalation: opt-in, default-off
> behind a flag, secret-scanned, path-filtered, and emitted only through a
> single sealed-DTO redaction choke point.

## Default posture: identity-only (CE-1)

Every GCTX tool and `graph://` resource defaults to **identity-only**. With the
snippet-egress opt-in off, GCTX-021/022/023 return spans **as locations** — a
workspace-root-relative `file` path, start/end line, and a no-text `ByteRange` —
never source text. The default surface thereby inherits the persistence
verdict's machine-local-equivalent safety class (d1/d2/e1).

Source-text egress requires **both**:

- **(a)** an explicit named opt-in flag — `gctx.egress`,
  `defaultVariant: disabled` (CE-9) — and
- **(b)** a per-request capability the client must assert.

With the opt-in unset, the fallback is identity-only, **not** text. No GCTX tool
auto-enables snippets on first use (CE-12). This is a hard Ready gate: CE-1 text
carries into GCTX-021, GCTX-022, and GCTX-023 item text before those items flip
to Ready.

## The sealed egress DTO and single choke point (CE-5)

The module's prior privacy posture was a single prose sentence — a
redaction-as-convention promise. This contract converts it into mechanism: a
**sealed egress projection DTO**, distinct from every internal graph type, built
by a **single** projection boundary.

Required properties of the egress DTO:

- **No `serde(flatten)`, no internal-type passthrough.** Internal types
  (`SymbolNode`, `GraphDelta`, …) must not serde-flatten into a response.
- **No `PathBuf`-typed field.** Every path-bearing string is
  workspace-root-relative — no leading `/`, no drive prefix.
- **No session-local `u64` node/edge id.** Symbols are identified by
  `SymbolIdentity` or `relative-path + name`, never the session-local `id: u64`.
- **One source-text carrier.** A `SnippetResult` (relative `file` +
  `ByteRange` + language + `text` + `truncated` + `omitted_bytes`) is the _only_
  type that can carry source text. Every other egress type is structurally
  incapable of holding source text or a free-form string. `SnippetResult.text`
  is populated only when the CE-1 opt-in is asserted.
- **Errors are a named enum.**
  `GctxError { NotReady, SymbolNotFound, BudgetExceeded, GraphUnavailable, StaleGraph, … }`
  — never `message: String` (free-form strings can echo source/identity).
- **Stable ordering.** Collections are sorted by a stable key (`SymbolIdentity`
  natural order) before serialisation, so responses are deterministic for
  identical graph state and query input.

**Single choke point.** A `GctxProjector` constructor is the only path that
builds egress DTOs; tool/resource handlers never construct them directly. One
projection boundary means one redaction/allowlist site (PV-9 N-5 — the ADR-083
single Rust surface makes this one choke point, not two).

**Structural no-leak test (mirrors persistence PV-7).** A compile-/test-time
assertion in the egress DTO crate proves: no `PathBuf` field exists; every
path-bearing string is workspace-root-relative; `GraphDelta` does not compile
into the egress module; and the only banned-name text field
(`text`/`body`/`content`/`source`/`snippet`/`raw`) is `SnippetResult.text`
behind the CE-1 opt-in. This test **gates GCTX-010** and every downstream
tool/resource item's Validation line. CE-5 is a hard item gate: without it,
every downstream item re-litigates redaction independently.

## Egress field allowlist and residual table (CE-4)

GCTX projects **only** the persistence ALLOW set. The projection boundary
**denies** — until their own privacy ADRs land — session/worktree identity,
`claimed_agent_id`, usernames, hostnames, absolute paths, `pid`/`pgid`/liveness,
raw `GraphDelta.errors`, and any APS/commit/Edda provenance ref (the GV2-013 /
GV2-014 field families sit outside the same-uid acceptance). GCTX must not
become the back door that egresses those fields ahead of their gating.

The named egress residual table (cross-boundary case) — analogous to the GV2-030
per-field-class table. Classes: **ALLOW** (egresses by default), **ALLOW-T**
(identity-class but carries a named residual; egresses with the residual
recorded), **DENY** (never egresses until a gating ADR lands):

| Field / class                                               | Egress  | Residual note                                                                                                                                                                                  |
| ----------------------------------------------------------- | ------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Symbol name, kind                                           | ALLOW   | identity class d1; no source text                                                                                                                                                              |
| Workspace-root-relative path                                | ALLOW   | identity class d2; never absolute, never a `PathBuf`                                                                                                                                           |
| Edge topology (caller/dependent/import edges)               | ALLOW   | structural identity (the persistence ALLOW narrative's edge class); no source text                                                                                                             |
| Structural / content hash                                   | ALLOW-T | unsalted SHA-256 is cross-machine-correlatable (git-blob class, PV-12 row e1) — now genuinely crosses the boundary (N-2); acceptable (same class as commit SHAs already in git) but named here |
| Import specifier incl. version string                       | ALLOW-T | `ImportEdge.to_source` (e.g. `lodash@4.17.21/fp`) reveals exact dependency versions to the assistant (N-3); included knowingly, never silently                                                 |
| `SymbolNode.trust_level`                                    | DENY    | default **exclude**; if ever surfaced, map to a documented client-facing alias — never raw `TrustLevel` passthrough                                                                            |
| Session/worktree identity, `claimed_agent_id`               | DENY    | GV2-013 control/session family; needs its own privacy ADR                                                                                                                                      |
| Usernames, hostnames, absolute paths, `pid`/`pgid`/liveness | DENY    | machine/operator identity; outside same-uid acceptance                                                                                                                                         |
| Raw `GraphDelta.errors`, `previously_*` sets                | DENY    | free-form strings embed `file::kind::name`                                                                                                                                                     |
| APS id / commit SHA / Edda provenance ref                   | DENY    | GV2-014 plan/provenance family; needs its own privacy ADR                                                                                                                                      |

The deny-list is a named, reviewable default, widened only by an explicit allow,
never silently.

## Sensitive-path egress filter (CE-3)

The named path deny-list — `.env*`, `*.pem` / `*.key` / `id_rsa*` / `*.p12`,
`.git/**`, and common secret directories (`secrets/`, `.aws/`, `.ssh/`,
`.gnupg/`) — applies to **every** egress surface, **identity-only enumeration
included**, not only snippet egress. A symbol whose file matches the deny-list
is **omitted from results entirely** (not merely snippet-redacted) — from
`anvil_search_symbols`, `graph://symbols`, and `graph://edges` as much as from
snippet responses — with an `omitted_sensitive_paths` count in metadata. The
filter is independent of the substrate filters: only the welcome scan honours
`.gitignore`; all other scans use `standard_filters(false)` by design, so GCTX
is the first surface where that content could egress. **Gitignored files** are
additionally filtered for **snippet egress specifically** (the substrate still
indexes them, so the gitignore cut is an egress-side guard, not a substrate
property). This folds into GCTX-010 and GCTX-030 (enumeration) as well as
GCTX-021 (snippets), so an identity-only tool cannot ship without the deny-list.

## Snippet egress: opt-in, secret-scanned, path-filtered (CE-2, CE-3, CE-12)

When (and only when) the CE-1 opt-in is asserted, snippet text crosses the
boundary through the following deny-by-default pipeline, in order:

1. **Sensitive-path filter (CE-3).** Apply the named path deny-list defined in
   "Sensitive-path egress filter (CE-3)" above (which already binds the
   identity-only enumeration surfaces); for snippet egress it **additionally**
   filters **gitignored files**. A denied path is **omitted from results
   entirely** (not merely snippet-redacted), with an `omitted_sensitive_paths`
   count in metadata.
2. **Deny-by-default secret scan (CE-2).** Before any snippet text crosses the
   boundary it passes a secret detector (reuse `anvil-intercept-rules`
   `SecretDetectionRule`) over the **emitted text** — not the source file, so
   partial slices are covered. On a hit the span is redacted with a placeholder
   marker and `redacted: true` + a count is set in response metadata.
   **Fail-closed:** a detector error redacts the span, never emits it. Redaction
   is **deterministic** (same input → same output) so GCTX-022 determinism
   property tests do not flake.
3. **Budget accounting (CE-2 ordering).** Redaction runs **before** budgeting so
   a redacted span still counts honestly toward the token budget (redact-before-
   budget). The budget keys on `(file, ByteRange)` position identity, never on
   text content.

**Consent (CE-12).** Enabling snippet egress is an explicit operator action with
a one-line consequence statement: _"source text from matched symbols,
secret-scanned and path-filtered, will be sent to the connected assistant/LLM
provider."_ The persistence boundary was implicit-and-safe (machine-local); this
boundary is the opposite and is surfaced, not silent (house rule: operator
config has no silent defaults). GCTX-032 documents exactly what crosses the
boundary, the identity-only default, the redaction/filter behaviour, and how to
disable.

## Volume bounds (CE-6)

Without bounds an assistant can page resources to dump the whole graph, chain
traversals to the transitive closure, reassemble whole files from overlapping
snippet calls, or abuse query params. The contract requires:

- **Resource caps.** A hard per-session byte cap on `graph://` resource reads
  plus a max page size. Pagination cursors are **server-minted opaque tokens**,
  never client-supplied offsets.
- **Traversal caps.** Per-call result-set size caps on traversal tools, plus a
  per-session **traversal credit** so call-by-call chaining cannot route around
  the per-call cap. An input file-count cap on `anvil_impact_of_change` (≈ ≤
  200).
- **Snippet byte ceiling.** A per-session snippet **byte** ceiling independent
  of the per-call token budget — defeats overlapping-span reassembly. The budget
  keys on `(file, ByteRange)` position identity, never text content.
- **Input validation.** Query params are exact-match or a named length-bounded
  glob (no arbitrary regex), ≤ 512 bytes/param. Reject NUL, path-separator
  traversal, and scheme-prefixed (`npm:`, `https:`, `data:`) inputs with a
  structured error **before** the graph is queried.
- **Diff is paths-only.** `anvil_impact_of_change` uses diff input for **changed
  file paths only** — it never reads or forwards diff content; content goes
  through the graph → snippet → redaction pipeline like any other source text.

Caps without disclosure are an unauditable answer. The next section is how every
projection reports which of these bounds fired, and what that cost.

## Self-attestation (ADR-142)

Every GCTX projection — the six tools and the `graph://` resources, and every
future projection that ships through `GctxProjector` — carries a single shared
**`Attestation`** block stating what bound the answer and what it costs. The
block is additive, counts-and-flags only, and identical in shape across every
tool and both delivery surfaces (MCP and the ADR-095 CLI secondary).

This is the third disclosure class. Two already exist:

- **Privacy** — `RedactionSummary` (CE-11): what the egress boundary removed.
- **Assurance** — `workspace_assurance.state` plus a `reason` (CE-7, ADR-084):
  whether the graph was warm.

The third class is **epistemic**: what the answer itself could not resolve, or
could not fit. It is already good on `anvil_search_symbols` (`matched` /
`returned` / `truncated`) and thin or absent on the reports with the highest
stakes (`ImpactSummary` / `AffectedTestsSummary` carry a single `truncated` bool
covering two budgets). This section makes the third class a contract, not a
per-tool courtesy.

Implementation lives in `crates/anvil-gctx-types` (the shared type held by every
projection DTO) and `crates/anvil-graph-cache` (per-edge call-resolution
fidelity; the existing `estimate_gctx_tokens` / `GCTX_TOKEN_ESTIMATOR_VERSION`
cost self-report). This spec freezes the contract; GATT-002 writes the
observability harness; GATT-003..005 land the fields.

Ranking confidence is **not** a field. `SearchSymbolsQuery` is conjunctive
filters over a deterministic keyset walk; there is no ranker, so there is no
ranking confidence to report. Inventing one would be the defect this contract
exists to prevent: an estimate presented with the confidence of a measurement.

### A shared type, not per-tool fields

`Attestation` is one struct, held by every projection DTO. It is **not** a set
of per-tool ad-hoc fields, and it is **not** folded into `RedactionSummary`.

Two separate structs, because they answer two different questions and are
audited by different people: `RedactionSummary` answers _what did the privacy
boundary remove_ (CE-11, a security artefact); `Attestation` answers _what did
the query fail to resolve or fit_ (an epistemic artefact). Merging them would
make the CE-11 counts harder to review in isolation, which is the property that
made those counts acceptable to PV-9.

Because the type is required by every projection DTO, a new tool cannot ship
without disclosure. That is the enforcement mechanism, and it is the point:
partial adoption across six tools is worse than none, since a caller then cannot
distinguish a tool with no bounds from one that does not report them.

### Bounds are named and separately counted

For every section an answer can cap, `Attestation` carries one entry naming:

- the **section** bounded,
- the **budget** that bound it (a closed enum — page limit, node budget,
  traversal depth, byte ceiling, session ceiling), one entry per budget that
  actually fired,
- **`returned`**, and
- **`total`**.

A section that was not bounded contributes no entry. Existing DTO
`truncated: bool` fields are retained for one release and become _derived_
(`true` iff at least one bound entry exists), documented as a compatibility
shim, not removed in this change.

### `total` is a tagged count, never a bare number

`total` is `Exact(n)` or `AtLeast(n)`. It is never a bare integer.

On a filtered scan (`anvil_search_symbols`) the pre-cap total is cheap and
honest — `Exact`. On a **bounded traversal** (`anvil_impact_of_change`'s
dependent closure, `anvil_affected_tests`' reverse walk) the true total is
unknowable without completing the walk the budget exists to prevent: reporting
one would require defeating the node budget on every call. `AtLeast` states the
floor and says it is a floor, in the type, so no consumer can read it as a
total.

A number that reads like a total but is a floor is the defect this contract is
about. Encoding the distinction in the type is the only way it cannot recur by
omission.

### Resolution fidelity moves to the edge

Call-resolution ambiguity is reported per call edge. Any caller-level
`heuristic` flag is retained as a **derived summary of its edge set** and
documented as such. Marking the symbol when one of its edges is ambiguous is a
false statement about its other edges, and blast-radius answers are read edge by
edge.

`crates/anvil-graph-cache` already computes per-edge ambiguity and then OR-s it
across all of a caller's edges into one caller-level `heuristic` flag. This
contract requires the per-edge decision to be retained on the projection; the
caller-level flag is a summary, not a property of the caller.

### The answer reports its own cost

`Attestation` carries `est_tokens` and the `estimator_version` that produced it,
from the existing `estimate_gctx_tokens` / `GCTX_TOKEN_ESTIMATOR_VERSION`
(`crates/anvil-graph-cache`). No second estimator is introduced. The version
travels with the number so it stays comparable across releases and reads as an
estimate rather than a measurement.

This reports cost; it does not enforce it. The snippet budget keeps its current
behaviour.

### Egress posture: counts and closed enums only, and `total` is post-filter

Every field is a count, a closed enum variant, or a version string. No name, no
path, no span, no content. The block therefore travels in the identity-only
default (CE-1) and inherits the counts-only posture PV-9 accepted for
`RedactionSummary` (CE-11); the `gctx.egress` opt-in is untouched.

**Binding constraint:** every count is taken **after** the CE-3 sensitive-path
deny-list is applied. Counting before it would turn `total` into an oracle for
the existence of denied paths — a query whose `total` exceeds its post-filter
result set would confirm that matching `.env` / `id_rsa` / `.aws` content
exists. Denied paths stay counted only in
`RedactionSummary.omitted_sensitive_paths`, which is where PV-9 put them.

### Gates are written before the fields they guard

No attestation field merges before a test that forces its condition and proves
the disclosure observable — a cap that actually fires, an ambiguous edge that
actually exists, an estimate actually attached. Each such gate must be shown to
fail when its subject is removed. A gate that cannot observe what it asserts is
worse than no gate, because it reports confidence. GATT-002 is that harness;
GATT-003..005 are the fields.

### Both surfaces, one shape

The block is defined on the projection, above the transport, so the MCP tools
and the ADR-095 CLI secondary render the same attestation from the same
`GctxProjector` output. Neither surface may carry a field the other lacks.

### Per-tool meaning

| Surface                                        | What `Attestation` must disclose                                                                                                                                                               |
| ---------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `anvil_search_symbols`                         | Page-limit bound with `Exact` pre-cap total (today's `RedactionSummary.matched` / `returned` / `truncated`; the shared block is the canonical form)                                            |
| `anvil_find_callers` / `anvil_find_dependents` | Per-call result-set cap and per-session traversal credit, as separate bound entries when they fire; per-edge resolution fidelity                                                               |
| `anvil_impact_of_change`                       | Affected-symbol cap and dependent-closure node budget as **separate** bound entries (never one bool covering both); tagged totals (`Exact` for a known seed set, `AtLeast` for a bounded walk) |
| `anvil_affected_tests`                         | Reverse-walk node budget as its own bound entry; heuristic / incomplete-coverage remains a derived summary of evidence edges                                                                   |
| `anvil_symbol_context`                         | Union of the above plus snippet byte-ceiling / token-budget bounds                                                                                                                             |
| `graph://stats`                                | Cost self-report; no section-bound entry unless a cap actually fired                                                                                                                           |
| `graph://symbols` / `graph://edges`            | Page / byte-cap bounds with tagged totals, post CE-3 filter                                                                                                                                    |

A future tool inherits this table by construction: it holds `Attestation`, so
shipping without disclosure is a compile error rather than a review catch.

## Stale-graph guard and degradation (CE-7)

The graph cache may lag source by a rebuild cycle. A credential-shaped symbol
scrubbed from source can persist in the cache; a snippet query in the stale
window would egress it. A naive warming fallback to raw file reads is worse than
the graph being off.

- **Snippet revalidation.** Snippet-returning responses re-validate the file's
  current content hash against the graph's recorded hash before emitting, or
  return a structured `StaleGraph` error (the verdict's `stale-graph` shorthand;
  `StaleGraph` is the egress `GctxError` variant) when the graph exceeds a
  stated staleness bound. Identity-only responses **may continue** under the
  existing warming/stale degradation — this carve-out is granted explicitly by
  PV-9 CE-7 (_"identity-only responses may continue under the existing
  warming/stale degradation"_), because identity-only data carries no
  source-text egress risk; the no-whole-file-fallback guarantee below still
  binds them.
- **No whole-file fallback.** When the graph is `warming` / `stale` /
  `disabled`, every tool and resource returns a structured degraded response
  (status + reason enum) with empty/omitted results — **never** a fallback to
  `std::fs::read_to_string` or any path returning file content outside the graph
  → redaction → budget pipeline.
- **Fixture.** A fixture exercises the warming path and asserts the snippet
  field is absent/empty.

## Transport and workspace-root boundary (CE-8)

- **Session-pinned root.** Every GCTX call is scoped to a single, session-pinned
  workspace root, validated at session init against the MCP server's launch root
  (reuse the `shared.rs` `validate_workspace_root` pattern). Graph tools
  additionally require that root to be the server cwd or a registered worktree
  root **exactly** — a nested directory is refused (CIB-398, ADR-125 amendment;
  `shared::validate_gctx_workspace_root`), because the daemon keys its graph on
  the root it is handed and a nested root would rebase root-relative file
  identities past the CE-3 deny-list. Cross-worktree queries are rejected. Until
  GV2-020 provides per-graph isolation, GCTX **fails closed** if the registry is
  absent or returns multiple candidates for a root (defeats a shared
  `ANVIL_HOME` / multi-graph daemon crossing worktree boundaries — Project A's
  assistant learning Project B's structure).
- **Stdio-only.** GCTX tools/resources are authorised over the **local stdio
  transport only** (`AnvilEntry::Stdio`, `anvil mcp serve --stdio`). Any RMCPF
  transport that crosses a network or cross-uid boundary (`RemoteSse`,
  `RemoteHttp`, TCP, cross-uid pipe) requires a **new** context-egress privacy
  review before GCTX tools register there. Tracked as a GCTX-002 acceptance
  criterion.

## Flag, telemetry, kill-switch (CE-9, CE-10, CE-11)

- **Flag (CE-9).** A `gctx.egress` entry in `flags/manifest.json` —
  `class: rollout`, `valueType: boolean`, `defaultVariant: disabled`,
  `createdFor: GCTX-001`, operator opt-in via `ANVIL_GCTX_EGRESS=1`. To avoid a
  FLAGCAT **orphan-flag drift** failure (the gate requires a TS+Rust consumer),
  the entry lands in the **same PR that first touches a GCTX implementation
  file**, not in this plan/contract change.
- **Telemetry (CE-10).** Every counter, span attribute, and notification binds
  to an exhaustive **outcome enum** (`hit`, `miss`, `warming`, `graph_disabled`,
  `budget_exceeded`, `redacted`, `error`, …). No label carries symbol names,
  paths (relative or absolute), query text, snippet text, or per-symbol token
  counts; only response-aggregate token totals are permitted, in spans. Applies
  equally to the ADR-035 tracing pipe and any future Kindling observation.
- **Kill-switch + redaction observability (CE-11).** `ANVIL_GCTX_EGRESS=0`
  disables egress on the **next call** — the flag is re-read per invocation,
  never cached at start-up (RMCPF per-tool capability gate). Each response
  carries a top-level `redaction_summary` of **counts + outcome enum only**
  (`fields_suppressed`, `snippets_truncated`, `fully_suppressed_symbols`,
  `outcome`) — no field names, symbol names, or path fragments. The tracing pipe
  may carry the same counts, never the suppressed content. `graph://stats` is
  the lowest-risk egress (aggregate counts, no identity) and the recommended
  default-on warm-up target for exercising this redaction-summary machinery
  before the snippet-bearing tools land (PV-9 N-1).

## Tool and resource projection map (the contract proper)

Each surface is a projection over the GV2-023 background tier, built only by
`GctxProjector`. All are identity-only by default; only the snippet-bearing
surfaces escalate under the CE-1 opt-in. Every surface carries the shared
`Attestation` block (ADR-142); see
[Self-attestation (ADR-142)](#self-attestation-adr-142) for per-tool meaning.

- **`anvil_search_symbols`** — paginated, deterministic symbol summaries
  (`SymbolIdentity` + kind + relative path + visibility), opaque cursors (CE-6),
  CE-3/CE-4 filtering. Identity-only; never carries text. Page-limit bound
  attests `Exact` pre-cap total. _(GCTX-010)_
- **`anvil_find_callers` / `anvil_find_dependents`** — bounded traversal results
  with distance, relative source file, symbol summary, and truncation metadata;
  per-call cap + per-session traversal credit (CE-6), disclosed as separate
  bound entries when they fire; per-edge resolution fidelity, with any
  caller-level `heuristic` flag derived from the edge set. Identity-only.
  _(GCTX-011)_
- **`anvil_impact_of_change`** — given changed **file paths** (never diff
  content), a deterministic `ImpactReport`: affected symbols, dependent files,
  known tests; input file-count cap ≈ ≤ 200 (CE-6). Affected-symbol cap and
  dependent-closure node budget are **separate** attestation entries; a bounded
  walk's `total` is `AtLeast`, never a bare integer. The existing
  `truncated: bool` is a one-release derived shim. Identity-only. _(GCTX-012)_
- **`anvil_affected_tests`** — test files + evidence edges with explicit
  heuristic/incomplete-coverage markers (derived from the evidence-edge set).
  Reverse-walk node budget is its own bound entry. Identity-only. _(GCTX-013)_
- **`anvil_symbol_context`** — the headline tool: search + impact + snippet
  extraction + token budgeting in one response. The primary snippet-bearing
  surface — text only under the CE-1 opt-in, through the full CE-2/CE-3
  pipeline; identity-only (spans-as-locations) otherwise. Attests the union of
  the above plus snippet byte-ceiling / token-budget bounds.
  _(GCTX-021/022/023)_
- **`graph://stats`** — aggregate counts (symbol/edge totals, warming/stale
  timestamp); the lowest-risk egress and the recommended default-on warm-up
  target for the CE-11 redaction-summary machinery (PV-9 N-1). Cost self-report
  always; no section-bound entry unless a cap actually fired. _(GCTX-030)_
- **`graph://symbols` / `graph://edges`** — read-only identity-only summaries
  with pagination (opaque cursors), CE-3/CE-4 filtering, and CE-6 byte caps,
  attested with tagged totals taken after the CE-3 deny-list. _(GCTX-030)_

## Warming, stale, and degradation behaviour by class

| Graph state | Identity-only surfaces                            | Snippet-bearing surfaces                                             |
| ----------- | ------------------------------------------------- | -------------------------------------------------------------------- |
| `warm`      | full identity projection                          | snippet text if CE-1 opt-in asserted, else spans-as-locations        |
| `stale`     | identity projection, marked `stale`               | `StaleGraph` error unless content-hash revalidation passes (CE-7)    |
| `warming`   | structured degraded response (empty/omitted)      | structured degraded response; snippet field absent/empty (CE-7)      |
| `disabled`  | structured degraded response (`GraphUnavailable`) | structured degraded response (`GraphUnavailable`); never a file read |

No state path falls back to a direct file read outside the graph → redaction →
budget pipeline.

## Determinism and ordering

Context slices and projections are **deterministic for identical graph state and
query input** (module Constraint). Concretely: collections are sorted by
`SymbolIdentity` natural order before serialisation (CE-5); pagination cursors
are server-minted and stable for a fixed query; secret redaction is
deterministic (CE-2); and the budget keys on `(file, ByteRange)` position
identity, not text, so determinism survives redaction.

## CE condition fold map

Where each PV-9 condition is discharged. CE-1 and CE-5 are **hard gates**; the
rest fold into item text and are verified at implementation.

- **CE-1** (snippet opt-in; identity-only default) — this spec +
  GCTX-021/022/023 item text. `Attestation` is counts, closed enums, and a
  version string only, so it travels in the identity-only default. _Hard Ready
  gate._
- **CE-2** (deny-by-default secret scan) — GCTX-021 (detector hook), GCTX-022
  (redact-before-budget).
- **CE-3** (sensitive-path / gitignore filter) — this spec + GCTX-010/021/030.
  Attestation `total` is counted **after** this deny-list (ADR-142 §6); denied
  paths stay in `RedactionSummary.omitted_sensitive_paths` only.
- **CE-4** (egress allowlist + residual table) — this spec (table above) +
  GCTX-010/012/013/030.
- **CE-5** (sealed DTO + single choke point + no-leak test) — this spec (DTO
  module + projector + no-leak-test scaffold) + every tool/resource Validation
  line. `Attestation` is a sealed DTO field, not an internal-type passthrough.
  _Hard item gate (GCTX-001 → Phase 1)._
- **CE-6** (volume bounds) — this spec + GCTX-010/011/012/022/030. ADR-142
  requires each fired bound to be named and counted separately on `Attestation`;
  `truncated: bool` is a one-release derived shim.
- **CE-7** (stale-graph guard; no whole-file fallback) — this spec +
  GCTX-021/023. Assurance (`workspace_assurance`) remains a separate disclosure
  class from attestation.
- **CE-8** (session-pinned root; stdio-only) — this spec + GCTX-002.
- **CE-9** (`gctx.egress` flag) — this spec + `flags/manifest.json` (at first
  implementation). Untouched by attestation.
- **CE-10** (enum-only telemetry) — this spec. `est_tokens` is an aggregate
  estimate on the projection, not a per-symbol telemetry label.
- **CE-11** (kill-switch + redaction summary) — this spec + GCTX-023 + GCTX-032.
  `Attestation` is a sibling of `RedactionSummary`, not a fold into it.
- **CE-12** (surfaced consent) — this spec + GCTX-032.

## What this spec deliberately does not freeze

- **No graph schema, stable identity, delta, hot-path index, or persistence
  contract** — owned by GV2; this spec projects over them (GCTX-001 Validation).
- **No hot-path enforcement contract** — GCTX is a non-hot context projection
  over the GV2-023 background tier only.
- **The exact `GctxProjector` / `SnippetResult` / `GctxError` type signatures**
  — named here as CE-5 implementation targets; their Rust shape is fixed by the
  Phase 1 items in the ADR-083 `anvil mcp serve` surface, not frozen here.
- **Token-estimator accuracy envelope** (GCTX-020), **benchmark fixtures**
  (GCTX-031), and **user-guide outline** (GCTX-032) — named, detailed in their
  items. ADR-142 reuses the existing estimator for cost self-report; it does not
  replace the accuracy envelope or enforce a token budget.
- **The Rust field names and enum discriminants of `Attestation`** — this spec
  freezes meaning (tagged totals, named budgets, per-edge fidelity, cost
  self-report, CE-3 post-filter counting). GATT-002..005 fix the type in
  `anvil-gctx-types` / `anvil-graph-cache`.

## Related docs

- [`graph-v2-foundation-spec.md`](./graph-v2-foundation-spec.md) — the substrate
  and the GV2-023 consumer query contract this projection sits on
- [context-egress privacy review (PV-9)](../../plans/reviews/2026-06-15-gctx-context-egress-privacy-review-verdict.md)
  — the source of CE-1..CE-12
- [ADR-083](../../plans/decisions/083-gctx-mcp-delivery-target.md) — MCP
  delivery target (`anvil mcp serve`, Rust RMCPF surface)
- [ADR-075](../../plans/decisions/075-v080-graph-product-scope.md) — v0.9 GCTX
  scope and the two entry gates
- [ADR-095](../../plans/decisions/095-gctx-cli-secondary-surface.md)
  (**Accepted**) — CLI secondary read surface over the same `anvil/gctx/*`
  daemon spine (MCP-first; CLI a co-equal thin client, not a runtime fallback)
- [ADR-142](../../plans/decisions/142-graph-answer-attestation.md)
  (**Accepted**) — graph answers attest their own bounds; this spec's
  self-attestation section is that decision
- [`plans/modules/graph-answer-attestation.aps.md`](../../plans/modules/graph-answer-attestation.aps.md)
  — GATT-001..006 implement the contract (spec, gate harness, per-edge fidelity,
  cap parity, cost self-report, consumer teaching)
- [`plans/archive/modules/graph-context-delivery.aps.md`](../../plans/archive/modules/graph-context-delivery.aps.md)
  — the GCTX module and work items
