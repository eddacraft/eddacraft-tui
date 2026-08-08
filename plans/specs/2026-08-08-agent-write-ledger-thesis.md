# Agent Write Ledger — Product Thesis

| Field       | Value                                                                |
| ----------- | -------------------------------------------------------------------- |
| Type        | Product thesis (pre-APS shaping)                                     |
| Status      | Draft — non-binding                                                  |
| Date        | 2026-08-08                                                           |
| Source      | anvil opportunity assessment of `cloudflare/computer` (MIT, preview) |
| Disposition | Product Note → AGOV-006 intake once the open decisions below are resolved |

> This is a **thesis**, not an APS module, ADR, or implementation spec. It exists
> to frame a candidate capability and the decisions that gate it. It does not
> change `plans/index.aps.md` and creates no work items. Promote to APS intake
> (as an AGOV-006 residual, not a new module) only after the
> [open decisions](#open-decisions) are settled — the durable-vs-local placement
> and the capture-mechanism objection recorded by ADR-052's council both need an
> ADR-level answer.

## Summary

anvil's durable evidence is **anchored at commits and merges**. The witness line
carries `seq`, `prev_line_hash`, `scope`, `kind`, `commit_sha`, `rules_sha`
(`crates/anvil-witness/src/line.rs:19-70`) — it proves *which protection layers
ran, under which rule set, at which commit*. Drift ships as snapshot/compare/
report (`anvil drift`, `crates/anvil-cli/src/commands/drift.rs`), and ADR-052
(**Proposed**, not shipped — no `anvil/drift/edges.ndjson` in tree) would replace
those snapshots with an edge-delta ledger appended on merge-to-`main`. Every one
of these is correct, and every one is commit- or merge-shaped.

The agent, meanwhile, works **between** commits. The intercept daemon already
observes every save in that window — the kernel-event watcher and the save-time
`validate_paths` verdict verb fold each path into per-worktree assurance state
(`crates/anvil-intercept/src/validate_paths.rs`,
`crates/anvil-intercept/src/save_time.rs`) — but nothing durable records *which
paths changed, in what order, to what content identity*. A file an agent wrote,
overwrote, and reverted before committing leaves no trace in any anvil artefact
and no Git object.

The opportunity is **not** a new evidence system and **not** file-content
capture. It is to record the identity of agent writes anvil already sees, as an
append-only, revision-ordered, content-**hash**-addressed ledger, so anvil can
answer "what did the agent actually touch this session, in what order" — the
question its current evidence cannot reach.

`cloudflare/computer`'s `@cloudflare/dofs` package supplies the shape:
content-addressed blobs (`vfs_blob_bytes` keyed by `hash`), a monotonic
`incrementRev()` sequencer stamped into `vfs_nodes.rev`, tombstones for deletes,
and comparable cursors (`currentRev`, `compareChangeCursors`,
`readWatermark`/`writeWatermark`). None of its code is usable here — it is
preview-only TypeScript over Durable Objects — but the four mechanisms are
textbook and directly expressible in anvil's own terms.

## The gap (why now)

- **The claim outruns the window.** anvil's north star is control "at the point
  of change creation" (`docs/vision/anvil-scope-guard.md`). Its durable evidence
  starts one step later, at the commit.
- **The data is already in hand.** The daemon sees every save. This is a
  *recording* gap, not a detection gap — no new capture surface, no new trust
  boundary.
- **Deletions are the weakest link.** A removal is the mutation class most
  likely to matter in an audit and the easiest to lose entirely. Git records a
  deletion only once it is committed; an agent that deletes a file mid-session
  and never commits leaves nothing behind.
- **AGOV-006's residual is exactly this shape.** Its 2026-07-11 delta note
  already concludes the hash chain is shipped and "the residual is at most
  extending witness coverage to the governance events AGOV cares about". Agent
  write events are such events.

## Thesis

> anvil can prove **what an AI actually wrote between commits** — every path,
> ordered, content-hash-addressed, deletions included — because its daemon
> already sees every save, and it records that identity without ever storing
> the content.

The differentiator is the **window**, not the mechanism. Git, code review, and
every post-hoc audit tool see the commit. anvil sees the twenty saves that
produced it, including the ones the agent walked back. That is the same
"too late" critique in `docs/vision/anvil-vision.md`, applied to evidence rather
than to enforcement.

## Scope

**In scope**

- Append-only ledger of agent write events: workspace-relative path, operation
  (create/modify/delete), monotonic revision, content hash, timestamp.
- Tombstone records for deletions, so removals are recorded rather than absent.
- A comparable cursor so "what changed between these two points" is answerable
  without snapshots.
- Reuse of the existing `anvil-witness` chaining primitive for tamper-evidence.

**Out of scope**

- **File content.** Hashes and relative paths only — see
  [constraints](#constraints-already-decided).
- Any new capture surface, watcher, or IPC verb. If the daemon does not already
  see it, it is not in scope.
- Rollback, restore, undo, time-travel, or branching of workspace state. That is
  a VFS product and fails the scope guard.
- Any runtime, sandbox, container, or execution surface. `cloudflare/computer`'s
  actual product is explicitly rejected; only `dofs`'s state model is borrowed.
- Enforcement. The ledger is evidence. It may later inform a gate; it does not
  become one here (ADR-002).

## How it reuses what we already ship

- **`crates/anvil-witness`** — `WitnessLine`'s `seq` is already a monotonic
  sequencer and `prev_line_hash` already gives tamper-evidence
  (`compute_line_hash`, `verify_chain_dag`, ADR-037). A write-event `kind` is an
  extension of an existing record, not a parallel chain.
- **`crates/anvil-intercept`** — the capture point exists.
  `validate_paths`/`save_time.rs` already receive the change set at the save
  boundary and already fold it per-worktree. The witness-append verb is already
  one of the daemon's 19 methods
  (`docs/architecture/intercept-as-built.md:218`).
- **ADR-052's proposed ledger shape** — append-only NDJSON event-delta ledger
  carrying `anvil_version` + `rules_sha`. Not shipped (ADR-052 is `Proposed`), so
  this is format *alignment*, not reuse: if both land, they should share one
  shape rather than inventing a second. If ADR-052 changes shape before
  acceptance, this follows it.
- **ADR-069's privacy line** — a written, tested precedent for "structural
  identity only: relative paths, hashes, span positions; never source, comments,
  docstrings, or literal values". This thesis needs no new privacy posture; it
  inherits one.

## Constraints already decided

These are settled and the design must fit inside them. None require revisiting.

1. **No content, no secrets.** ADR-072 §3 forbids secrets in durable evidence;
   ADR-069 draws the structural-identity-only line. Content bytes are therefore
   out — the ledger stores hashes. This is also why the borrowed model must be
   *rev + hash + tombstone*, **not** `dofs`'s `vfs_blob_bytes` content store.
2. **Content-addressing is already anvil's posture.** ADR-072 ratifies durable
   evidence as content-addressed and offline-verifiable **using the repo
   itself**. Do not build a bespoke CAS; Git is the content store, and the
   ledger references hashes rather than holding bytes.
3. **Durable vs local is a hard boundary.** ADR-073: tracked `anvil/` for
   durable governance evidence, gitignored `.anvil/` for local runtime state.
   Placement is an [open decision](#open-decisions), not a free choice.
4. **Three-pipe rule.** ADR-035: this is a Kindling governance fact if durable,
   never tracing/OTEL. It cannot be justified as observability — the scope guard
   allows observability only where it serves enforcement or provenance.
5. **Warnings over blocks.** ADR-002: evidence first, enforcement opt-in and
   later.

## The ADR-052 objection

ADR-052 (**Proposed**, 2026-05-27 — not accepted, not implemented) proposes an
append-only edge-delta ledger appended **on merge-to-`main` via the PR that
introduces the edges**, and its planning council (`plan-0e9c300c`) rejected the
**daemon/hook/witness-chain/orphan-branch** alternatives. This thesis proposes
daemon-side capture into the witness chain — the mechanism that council rejected.
Pretending otherwise would waste a review.

Note what that status does and does not mean. A `Proposed` ADR is **recorded
council reasoning, not ratified policy**: it does not block this thesis the way an
Accepted ADR would, and it cannot be cited as settled precedent in either
direction. But the reasoning is on the record and was reached by a council
looking at a closely adjacent problem, so it is a strong prior that has to be
answered rather than routed around. The same cuts the other way: this thesis must
not lean on ADR-052 as authority for its own ledger format either — hence
"alignment, not reuse" above.

The distinction to argue — and it may not survive scrutiny:

- **ADR-052's subject is commit-shaped by nature.** Drift edges are a property
  of merged state; merge-to-`main` is the natural boundary and a daemon adds
  nothing but a bypass path and a scheduler.
- **This subject is not.** Intra-session writes *only* exist inside the daemon's
  window. Deferring to merge-to-`main` does not make the capture more robust —
  it makes it impossible, because the evidence is gone by then.

If a council upholds that reasoning here, the honest outcome is that this
capability is not available to anvil at all, because there is no non-daemon place
to stand. That is an acceptable answer and should be recorded as one rather than
worked around.

A second, softer objection follows: a daemon-written durable artefact is
bypassable (a developer who does not run the daemon produces no ledger) and
therefore cannot be a completeness claim. The ledger must be marketed as
"everything anvil saw", never "everything that happened", and a missing ledger
must read as `degraded`, never `pass` (ADR-072).

## Open decisions

1. **Durable or local?** (ADR-073) Tracked `anvil/writes/` makes the ledger
   PR-reviewable and shareable, but puts per-developer keystroke-adjacent
   activity into the repo — a privacy and noise problem drift evidence does not
   have, and a likely merge-conflict generator. Gitignored `.anvil/` keeps it
   local but forfeits the audit-export value that motivates the whole thesis.
   **This is the gating decision and it needs an ADR.**
2. **Does daemon capture survive the objection ADR-052's council recorded?** See
   above. Needs a planning council, not a unilateral call — and ideally one that
   also resolves ADR-052's own `Proposed` status, since the two share a mechanism
   question.
3. **Retention.** A ledger of every save grows without bound. ADR-116 amended
   Kindling to append-only with one authenticated, receipted governance-prune
   exception — does that mechanism apply, or does this need its own rollover
   (`RolloverPolicy` already exists in `anvil-witness`)?
4. **Own `kind` or own ledger?** A write-event `kind` inside the existing
   witness chain is cheapest and inherits verification, but risks drowning
   layer-evidence lines in write volume. A sibling NDJSON ledger keeps them
   separable at the cost of a second chain to verify.
5. **Hash of what, exactly?** Post-save file content hash is simplest.
   Whether it should agree with the Git blob hash for the same bytes (so the
   ledger cross-references Git objects directly, per ADR-072) is a real design
   question with real ergonomic payoff.
6. **Is the customer question actually being asked?** No beta feedback cited
   here demands this. It should be validated against real users before it
   consumes roadmap space — see [risks](#risks).

## Customer framing & differentiation

The buyer-legible line: **"Show me every file the agent touched in this session,
in order, including the ones it changed its mind about."**

Post-hoc tooling — Git, review, SCA, audit platforms — starts at the commit and
cannot answer this even in principle. anvil's position at the save boundary is
the only place the question is answerable, which makes this a differentiation
argument rather than a feature-parity one.

Two honest limits to state alongside it:

- It is "everything anvil saw", not "everything that happened". The intercept
  trust boundary is same-UID local-IPC and the mediation is **cooperative**, not
  mandatory (`docs/architecture/intercept-as-built.md:67`): a process that
  simply writes a file is observed by the watcher, not prevented, and a
  developer not running the daemon produces no ledger at all.
- It records identity, never content. "What did it write" is answerable as
  paths, order, and hashes — not as a diff.

Both limits are worth saying out loud in customer material. A prospect
evaluating an agent control layer will eventually ask what stops the agent
writing the file anyway, and discovering the answer unaided is worse than being
told.

## Risks

- **Scope drift into a VFS.** `cloudflare/computer`'s framing ("give your agent
  a computer") is attractive and wrong for anvil. Workspace state management,
  rollback, and runtime swapping are excluded by the scope guard's
  general-purpose-agent-systems and CI/CD clauses. The ledger records; it never
  restores.
- **Volume and cost.** Every save on a large repo, per worktree, forever.
  Needs sizing, an ignore filter (`dofs` needed `DEFAULT_IGNORE`/`isIgnored` for
  exactly this), and a retention answer before it can be planned.
- **Secret leakage via paths.** ADR-069 accepted residual
  secret-in-identifier risk under same-uid/default-off/owner-only. A *durable,
  tracked* ledger weakens each of those three mitigations, so the accepted risk
  does not automatically carry over. Path-only exposure is narrower than
  content, but a path can still name a customer or a secret file.
- **Council-rejected mechanism.** See [above](#the-adr-052-objection). Real
  chance the answer is no.
- **Evidence-system sprawl.** anvil already has witness (ADR-037), baseline
  (ADR-039), drift snapshots (`anvil drift`), capsules (ADR-074), and Edda — with
  ADR-052's edge-delta ledger proposed on top. A further artefact needs to justify
  not being a `kind` on an existing one.
- **Unvalidated demand.** This originated from a repository the team found
  interesting, not from a user asking for it. That is the weakest possible
  provenance for roadmap space, and the reason this stays a non-binding thesis.
- **Upstream volatility.** `cloudflare/computer` is preview-only with unstable
  APIs and accepts no unsolicited PRs. Every reference here is a 2026-08-08
  snapshot and may not survive; nothing should depend on it.

## Recommended next step

1. **Validate demand first.** Before any planning, check whether beta users or
   the pilot cohort actually ask "what did the agent touch". If not, park the
   thesis — the constraint work below is not worth spending speculatively.
2. **If demand is real, take open decisions 1 and 2 to a planning council
   together.** Durable-vs-local and the ADR-052 capture objection are
   entangled; deciding either alone will be re-litigated.
3. **Only then, promote as an AGOV-006 residual** — not a new module. AGOV is
   `Draft` and gated on the POLRESET-010 / ADR-098 product decision about which
   signal producers ship; this queues behind that gate rather than jumping it.
4. **Reject the dependency explicitly** so it is not revisited: no adoption,
   vendoring, or clean-room of `cloudflare/computer` or `@cloudflare/dofs`.
   Inspiration only.

## References

**Code**

- `crates/anvil-witness/src/line.rs:19-70` — `WitnessLine` fields: `seq`,
  `prev_line_hash`, `scope`, `kind`, `commit_sha`, `rules_sha`. No path, no
  content identity.
- `crates/anvil-witness/src/lib.rs` — chain surface: `compute_line_hash`,
  `verify_chain_dag`, `RolloverPolicy`, `WitnessWriter`.
- `crates/anvil-intercept/src/validate_paths.rs`,
  `crates/anvil-intercept/src/save_time.rs` — the save-boundary change-set
  capture point that already exists.
- `docs/architecture/intercept-as-built.md:67` — same-UID local-IPC trust
  boundary; `:218` — the witness-append verb among the daemon's 19 methods.

**Decisions** (status as at 2026-08-08; all Accepted unless noted)

- ADR-002 — warnings over blocks; enforcement opt-in.
- ADR-035 — three-pipe observability rule (Kindling = governance facts).
- ADR-037 — witness chain and L4 policy framework.
- ADR-052 — **`Proposed`, not accepted, not implemented** — drift as an
  append-only edge-delta event ledger; its planning council (`plan-0e9c300c`)
  rejected daemon/hook/witness-chain capture. Recorded reasoning, not ratified
  policy — see [the objection](#the-adr-052-objection).
- ADR-069 — Graph-V2 persistence; the structural-identity-only privacy line.
- ADR-072 — git-native governance substrate; content-addressed durable
  evidence, no secrets, missing evidence is `degraded` not `pass`.
- ADR-073 — durable `anvil/` vs local `.anvil/` boundary.
- ADR-116 — Kindling append-only with a receipted governance-prune exception.

**Modules**

- `plans/modules/agent-governance-patterns.aps.md` — AGOV-006 (hash-chained
  audit trail) and its 2026-07-11 delta note identifying the residual as
  extending witness coverage to new governance events.

**External** (2026-08-08 snapshot; MIT, preview-only, unstable APIs)

- `cloudflare/computer` — `@cloudflare/dofs` state model: `vfs_blob_bytes`
  content-addressed blobs, `vfs_nodes.rev` + `incrementRev()` monotonic
  sequencer, tombstoned deletes, `currentRev`/`compareChangeCursors` cursors,
  `readWatermark`/`writeWatermark`, `DEFAULT_IGNORE`/`isIgnored`.
