<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->

# LSP Graph Navigation

| ID | Owner | Status | Progress |
| -- | ----- | ------ | -------- |
| LSPNAV | @eddacraft | Proposed | 0/7 |

**Last reviewed:** 2026-07-20 — scope and architecture approved through Planning
Council `plan-33b005f5`; [ADR-111](../decisions/111-graph-backed-lsp-references.md)
is Proposed. No implementation item is Ready until the production
diagnostics-only RTAI-005 boundary lands and this module is reconciled to it.

## Purpose

Give people and agents an exact, bounded view of every statically resolved use
of a symbol before they change governed code. The first release exposes only
`textDocument/references` for one evidence-certified language/client matrix.

This is not an expansion of RTAI-005. RTAI owns production LSP diagnostics;
`anvil-graph-cache` and Intercept own occurrence state and publication; GCTX
owns protocol-neutral anchored query/egress; and LSPNAV owns editor projection.
LSPNAV owns the end-to-end delivery acceptance for those coordinated slices so
accuracy cannot fall between module boundaries.

## In scope

- A closed reference-occurrence taxonomy and certification evidence for one
  evidence-selected language tier.
- An immutable occurrence-capable graph snapshot, atomically versioned and
  persisted with the graph generation.
- One bounded, anchored, protocol-neutral exact-reference query with closed
  no-partial CE-5 outcomes and CE-11 control.
- Dynamic LSP references registration for the certified language and supported
  client matrix.
- Exact workspace-relative-to-URI and byte-to-UTF-16 projection,
  `includeDeclaration`, dirty-buffer rejection, cancellation, multi-root
  routing, and Unix/Windows parity.
- Shadow, canary, soak, rollback, and release evidence that preserves save-time
  validation budgets.

## Out of scope

- Definitions, hover, code lenses, code actions, rename, or per-keystroke graph
  rebuilds.
- Static global references advertisement or best-effort/partial references.
- Cross-root results, source snippets, general occurrence export, or LSP-side
  workspace reads.
- Compatibility with unmerged navigation wires explored in PR #3360.
- Impact-of-change and affected-test delivery in the first release. Their
  Proposed intents below define outcomes only, not v1 wire names.

## Interfaces

**Depends on:**

- RTAI-005 production diagnostics-only LSP lifecycle and daemon-client boundary.
- [ADR-111](../decisions/111-graph-backed-lsp-references.md) and the
  [approved design](../specs/2026-07-20-lsp-graph-backed-references.md).
- GCTX's released CE-5/CE-11 query and no-leak spine
  ([archived module](../archive/modules/graph-context-delivery.aps.md), ADR-084).
- Graph V2 symbol/call/dependency identities, graph persistence (ADR-069), and
  held `WorkspaceAnchor` reads.
- ADR-031 latency measurement and the daemon scheduler's save-time class.

**Exposes:**

- A versioned reference-tier certification matrix.
- A protocol-neutral, atomic exact-reference outcome below CE-5 with
  daemon-derived, explicitly tagged UTF-16 coordinates.
- A dynamically registered LSP references capability for certified clients and
  documents only.
- Privacy-safe aggregate rollout evidence and immediate CE-11 rollback.

## Cross-cutting ownership

- **`anvil-graph-cache` owns:** occurrence structures and invariants, bounded
  deltas, the composite graph snapshot, deterministic ordering, and persistence.
- **Intercept owns:** admitted anchors, parsing, rebuild orchestration,
  scheduling, cancellation, and atomic snapshot publication.
- **GCTX owns:** anchored query projection, daemon-derived UTF-16 coordinates,
  bounds, work credits, and CE-5/CE-11 semantics.
- **LSPNAV owns:** LSP lifecycle, client/language capability matrix, URI and
  protocol DTO mapping, standard error mapping, and feature unregistration.
- **RTAI owns:** mid-edit diagnostics only. Navigation does not change its
  `scan_buffer` contract or completion criteria.
- **LSPNAV delivery acceptance owns:** proving the coordinated GCTX and LSP
  slices satisfy one exact end-to-end reference contract.

## Ready checklist

- [x] Operator approved the diagnostics/GCTX/LSPNAV ownership split.
- [x] Planning Council `plan-33b005f5` Phase 4 review has no unresolved major or
      critical objections and the session is closed as converged.
- [x] Exactness, trust-boundary, capacity, abuse-control, rollout, and non-goal
      decisions are recorded in ADR-111 and the design spec.
- [ ] PR #3360 lands as production diagnostics-only and this module is rebased
      and reconciled to the final RTAI-005 surface.
- [ ] ADR-111 is accepted and indexed.
- [ ] LSPNAV-001 has selected a tier from evidence and pinned its taxonomy and
      launch-client matrix.

## Work Items

### LSPNAV-001: Certify the first exact-reference tier

- **Execution plan:** [LSPNAV-001 tier certification](../execution/LSPNAV-001.actions.md)
- **Intent:** Establish an auditable completeness claim for one language tier
  and the clients allowed to receive its references capability.
- **Expected Outcome:** A versioned closed occurrence taxonomy, adjudicated
  golden corpus, differential native-engine evidence, full/incremental/restore
  parity evidence, and real-client dynamic-registration matrix select exactly
  one launch tier. Every unsupported construct and client has an explicit
  fail-closed disposition. The evidence digest names every change class that
  invalidates certification.
- **Files:** `plans/specs/2026-07-20-lsp-graph-backed-references.md`,
  `crates/anvil-kernel/tests/lspnav_certification.rs`,
  `crates/anvil-kernel/tests/fixtures/lspnav/`,
  `crates/anvil-graph-cache/tests/lspnav_certification.rs`,
  `crates/anvil-graph-cache/tests/fixtures/lspnav/`
- **Dependencies:** RTAI-005 production diagnostics-only; ADR-111 Accepted.
- **Validation:** `cargo test -p eddacraft-anvil-kernel --test lspnav_certification && cargo test -p eddacraft-anvil-graph-cache --test lspnav_certification`
- **Confidence:** medium
- **Status:** Proposed

---

### LSPNAV-002: Publish an occurrence-capable composite graph snapshot

- **Intent:** Make declarations and all certified occurrences readable from one
  immutable, restart-safe graph generation.
- **Expected Outcome:** Full scans, deltas, restore, and restart produce the same
  deterministically ordered occurrence results inside one composite snapshot.
  Persistence is accepted or rejected as a unit; corrupt, incompatible, newer,
  root-mismatched, incomplete, or downgrade-unsupported state yields `NotReady`
  and an anchored rebuild. No query surface is dispatchable in this item.
- **Files:** `crates/anvil-graph-cache/src/`,
  `crates/anvil-graph-cache/tests/lspnav_snapshot.rs`,
  `crates/anvil-intercept/src/kernel_cache.rs`,
  `crates/anvil-intercept/src/full_scan_executor.rs`,
  `crates/anvil-intercept/tests/lspnav_snapshot.rs`
- **Dependencies:** LSPNAV-001; ADR-069; GCTX/GV2 graph identities.
- **Coordinates with:** `anvil-graph-cache` owns occurrence/snapshot/persistence
  implementation, Intercept owns rebuild/publication, and LSPNAV owns the
  certified-reference acceptance evidence.
- **Validation:** `cargo test -p eddacraft-anvil-graph-cache --test lspnav_snapshot && cargo test -p eddacraft-anvil-intercept --test lspnav_snapshot`
- **Confidence:** medium
- **Status:** Proposed

---

### LSPNAV-003: Seal the bounded anchored reference query

- **Intent:** Expose one exact reference query without allowing partial egress,
  unsafe roots, repeated-query abuse, or save-time starvation.
- **Expected Outcome:** A separately reviewed daemon RPC pins one admitted root
  and composite generation, verifies every participating file through the held
  anchor, validates a bounded authenticated complete open-document manifest,
  derives tagged UTF-16 coordinates from verified bytes, and returns only the
  closed ADR-111 outcomes, including `Busy` and `DeadlineExceeded` under the
  normative precedence order. Immutable lowerable
  ceilings, a 100 ms queue bound, one-second end-to-end deadline, cooperative
  cancellation, an exclusive save-time reservation, and bounded hierarchical
  daemon-derived work credits plus fair principal/root/session admission make
  load and reconnect behaviour deterministic. CE-11 defaults the direct
  location surface off.
- **Files:** `crates/anvil-gctx-types/src/lib.rs`,
  `crates/anvil-gctx-egress/src/`, `crates/anvil-intercept-proto/src/`,
  `crates/anvil-intercept/src/ipc.rs`, `crates/anvil-intercept/src/path_safety.rs`,
  `crates/anvil-intercept/tests/lspnav_references.rs`
- **Dependencies:** LSPNAV-002; ADR-084 CE-5/CE-11 spine.
- **Coordinates with:** the daemon scheduler and authenticated peer/root
  admission contracts; navigation may never consume reserved save-time capacity.
- **Validation:** `cargo test -p eddacraft-anvil-gctx-types --all-targets && cargo test -p eddacraft-anvil-gctx-egress --all-targets && cargo test -p eddacraft-anvil-intercept --test lspnav_references`
- **Confidence:** medium
- **Status:** Proposed

---

### LSPNAV-004: Project certified references through LSP

- **Intent:** Make exact references available only where the server can honour
  the certified language and client contract.
- **Expected Outcome:** The LSP server omits a global references provider,
  dynamically registers an exact document selector after `initialized`, maps
  only `Ready` relative locations and daemon-produced tagged UTF-16 ranges to
  encoded file URIs and LSP DTOs, honours
  `includeDeclaration`, and maps every non-ready outcome to a stable standard LSP
  error. Dirty buffers, generation changes, multi-root ambiguity, tier
  invalidation, cancellation, disablement, and races never return partial or
  null success. A bounded authenticated capability-epoch subscription
  unregisters the provider while idle and fails closed on disconnect or daemon
  restart. Unix and Windows transports have the same semantics.
- **Files:** `crates/anvil-cli/src/commands/lsp.rs`,
  `crates/anvil-cli/src/commands/`, `crates/anvil-cli/tests/lsp_references.rs`
- **Dependencies:** production RTAI-005; LSPNAV-001; LSPNAV-003.
- **Validation:** `cargo test -p eddacraft-anvil --test lsp_references`
- **Confidence:** medium
- **Status:** Proposed

---

### LSPNAV-005: Prove and promote the reference capability

- **Intent:** Enable exact references only after correctness, privacy,
  performance, resource, client, and rollback evidence passes on supported
  platforms.
- **Expected Outcome:** Shadow comparison produces no location egress; opt-in
  canary covers the certified language/client matrix; bounded soak and abuse
  tests prove no permit, memory, credit, or fairness leak; warm accepted queries
  meet p95 <= 250 ms, p99 < 500 ms, and max < 1 s; existing save-time gates stay
  green under navigation saturation. CE-11 rollback unregisters and cancels
  navigation without affecting diagnostics, and older binaries rebuild safely
  from newer cache state. A checked evidence manifest names every required
  certification, client, platform, benchmark, boundary, soak, rollback, canary,
  and save-time result and fails if any evidence is absent, stale, zero-test, or
  from an unsupported platform.
- **Files:** `crates/anvil-intercept/benches/`, `crates/anvil-cli/tests/`,
  `.github/workflows/`, `scripts/lspnav/evidence-check.mjs`,
  `plans/reviews/lspnav-reference-release-evidence.md`, `package.json`,
  `docs/architecture/`
- **Dependencies:** LSPNAV-004.
- **Validation:** `pnpm lspnav:evidence:check && pnpm validate:full`
- **Confidence:** medium
- **Status:** Proposed

---

### LSPNAV-006: Shape graph-backed impact-of-change projection

- **Intent:** Decide whether certified occurrence evidence can improve
  deterministic change-impact guidance without widening the first references
  release or duplicating existing GCTX behaviour.
- **Expected Outcome:** A separately approved design defines the user outcome,
  accuracy boundary, ownership, privacy posture, and evidence gate, or records a
  no-go. It creates no compatibility promise for a transport or method name
  before that approval.
- **Dependencies:** LSPNAV-005 release evidence and a concrete demand signal.
- **Validation:** `pnpm aps:active-lint && pnpm adr:check`
- **Confidence:** low
- **Status:** Proposed

---

### LSPNAV-007: Shape graph-backed affected-test projection

- **Intent:** Decide whether certified occurrence evidence can improve
  deterministic affected-test discovery without coupling it to the first
  references release.
- **Expected Outcome:** A separately approved design defines evidence semantics,
  coverage gaps, ownership, privacy and validation, or records a no-go. It
  creates no compatibility promise for a transport or method name before that
  approval.
- **Dependencies:** LSPNAV-005 release evidence, LSPNAV-006 disposition, and a
  concrete demand signal.
- **Validation:** `pnpm aps:active-lint && pnpm adr:check`
- **Confidence:** low
- **Status:** Proposed

## Risks

| Risk | Impact | Likelihood | Mitigation |
| ---- | ------ | ---------- | ---------- |
| Plausible partial references are mistaken for exact results | High | Medium | Closed certified taxonomy; no partial success; runtime tier invalidation |
| Stale or substituted files corrupt locations | High | Medium | Composite generation plus response-set-wide held-anchor verification |
| Sensitive paths or identities leak | High | Low | Generic whole-request `Withheld`; CE-5 structural no-leak tests |
| Repeated queries exhaust or scrape the daemon | High | Medium | Hierarchical weighted work credits, bounded account state, coarse failures |
| Navigation delays save-time protection | High | Medium | Exclusive save-time scheduler reservation and contention release gate |
| Client/platform capability drifts | Medium | Medium | Dynamic-only launch matrix and Unix/Windows real-client certification |
| Rollback cannot consume newer persistence | Medium | Low | Additive versions, reject-and-rebuild, no reverse migration |

## Decisions

- ADR-111 is the architecture authority for exact graph-backed LSP references.
- ADR-109 remains authoritative for protocol plurality and RTAI-005's
  diagnostics-only scope.
- Planning Council `plan-33b005f5` approved the ownership, accuracy, trust,
  capacity, abuse-control, delivery, and rollout decisions recorded here.

## Open questions

- Which language wins LSPNAV-001's evidence-based first-tier selection?
- Which clients meet the dynamic-registration launch matrix?
- What checked-in ceilings and credit defaults pass the slowest supported
  reference-class calibration without regressing save-time validation?
