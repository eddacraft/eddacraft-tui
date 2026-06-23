# ADR-091: GCTX pagination cursor fingerprint integrity

## Status

Proposed

## Date

2026-06-24

## Context

The GCTX Phase-1 egress tools (`anvil_search_symbols`, `anvil_find_dependents`,
and siblings) paginate with a **server-minted opaque keyset cursor**. The cursor
(`crates/anvil-gctx-egress/src/lib.rs`, `CursorPayload`) is the hex encoding of a
small JSON document:

```jsonc
{ "q": <u64 FNV-1a fingerprint of the query filters>,
  "k": <last SymbolIdentity returned on the previous page> }
```

A follow-up call echoes the cursor back; the server decodes it, checks that `q`
equals the fingerprint of the *current* call's filters, and resumes the keyset
seek strictly after `k`. The fingerprint (`query_fingerprint` /
`dependents_fingerprint`) is FNV-1a over the canonical, lowercased filter set —
deliberately reproducible across daemon restarts (privacy verdict PV-2), unlike
the randomly seeded `std` hasher, so a minted cursor survives a restart.

The cursor is **opaque but not secret**: it is plaintext hex with no MAC and no
encryption. A client can trivially decode it, set `k` to any `SymbolIdentity`,
recompute `q = fnv1a(filters)` (the algorithm is public and the filter inputs
are the client's own query), re-encode, and present a **forged** cursor. The
GCTX-011 council flagged this — *"the current FNV fingerprint only reseeks
identity-only pages (no data leak, but forgeable)"* — and the question of
whether to harden it (a keyed HMAC fingerprint) was deferred at merge and split
out of CIB-099 into **CIB-103** so CIB-099 could land as cleanly mechanical
work. This ADR settles that question.

The decision must answer one thing: **does cursor forgeability enable any
threat, given the current identity-only egress?**

## Decision

**Keep the reproducible FNV-1a fingerprint. Do not add an HMAC or other keyed
construction to the GCTX pagination cursor at this time.** The cursor is
documented as a **non-security-bearing convenience token**, and the property is
pinned by a test.

Concretely (CIB-103 execution):

- Add a module-level threat-model note in `anvil-gctx-egress` stating that the
  cursor is a server-minted keyset *seek position*, not an authorisation or
  capability token: it carries no grant, and a follow-up page is re-authorised
  by the **echoed query filters** and the CE-5 identity-only projection choke
  point — not by the cursor.
- Add a test pinning the property: a malformed, garbage, or **forged** cursor
  (one with an attacker-chosen `k` and a recomputed matching `q`) yields either a
  valid page **within the caller's own authorised result set** or an empty page —
  never a leak of anything the query itself would not return, never a panic, and
  never an out-of-bounds read. The existing `MAX_CURSOR_BYTES` length cap and
  serde-bounded decode remain the only parsing defences needed.
- Record the **explicit revisit trigger** below in the same note so the next
  author who changes what the cursor encodes is forced to re-open this decision.

### Revisit trigger (binding condition)

This decision **flips to a keyed MAC (or server-held opaque state, ADR
Alternative C)** the moment a cursor encodes anything the echoed query does not
independently re-authorise. Specifically, re-open this ADR if **any** of:

1. A cursor ever carries **source/snippet payload** or a position into it (the
   GCTX Phase-2 CE-1 snippet escalation) — the cursor would then resume into data
   the choke point no longer re-gates per page.
2. A cursor ever encodes **cross-workspace, cross-tenant, or trust-scope** state
   (e.g. a scope token, a privileged graph handle, a `WorkspaceAssurance`
   bypass).
3. A cursor ever resumes into a result set **not** fully determined by the
   client-supplied, re-fingerprinted filters — i.e. the cursor becomes load-
   bearing for *what* is returned, not just *where the stream resumes*.

At that point the cursor becomes a capability and MUST be unforgeable.

## Rationale

Cursor forgeability has **no exploitable consequence** in the current
identity-only egress, so an HMAC would add key-management cost and a wire-
contract change to defend a non-threat.

- **The cursor is not an authorisation token.** Access is governed by (a) the
  query filters the client supplies on *every* call and (b) the daemon-side CE-5
  `GctxProjector` choke point, which returns identity-only data and no source.
  The cursor only selects *where in an already-authorised result stream* to
  resume.
- **Forgery reaches nothing new.** A forged cursor reseeks to an arbitrary `k`
  *within the same filter set the caller already provides*. Every page it could
  reach is reachable by paging legitimately from the start of that same query. No
  identity is returned that the query would not return; no source content (CE-5);
  no cross-query leak (the fingerprint binds `q` to the filters, and even a
  forged matching `q` only reseeks within the caller's own query).
- **No availability impact.** The cursor length is bounded by
  `MAX_CURSOR_BYTES`; decode is serde-bounded; the seek is a bounded keyset
  lookup. A garbage `k` produces a valid-or-empty page, not a crash or unbounded
  work.
- **HMAC has real, recurring cost for zero gain here.** It needs per-daemon key
  material (generation, at-rest storage under `.anvil/`, rotation), and it must
  cohere with multi-daemon / `ANVIL_HOME` side-by-side coexistence (CIB-101). Key
  rotation invalidates in-flight cursors. All of this would sit on the warm read
  path to make a non-capability token unforgeable.
- **The FNV fingerprint already does its one real job** — binding a cursor to
  the filter set so a cursor minted for query A cannot silently resume query B
  (pagination overlap/gap, a *correctness* property, not a security one). PV-2
  reproducibility across restarts is a feature the random `std` hasher would
  break; a keyed MAC would similarly need stable key material to preserve it.

Keeping FNV is therefore the smallest correct scope that closes CIB-103: it
documents the boundary honestly and pins it with a test, while reserving the
unforgeable construction for the exact future change that would make it
necessary.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| **Keep FNV + document + pin with a test (chosen)** | Matches the actual threat model (cursor is a seek position, not a capability); zero new key material or wire change; preserves PV-2 restart-stable cursors; documents the boundary and the revisit trigger | Cursor stays client-decodable and forgeable — acceptable only while egress is identity-only and query-re-authorised; relies on a future author honouring the revisit trigger |
| **Keyed HMAC fingerprint now** | Cursor becomes unforgeable; future-proofs against the Phase-2 snippet case pre-emptively | Per-daemon key generation/storage/rotation; coheres with CIB-101 side-by-side daemons; rotation invalidates live cursors; added hot-path cost — all to defend a non-threat today; over-engineering ahead of the trigger |
| **Server-held cursor state keyed by a random opaque handle** | Nothing client-decodable or forgeable; payload never leaves the server | Server-side memory + eviction policy (an evicted entry expires a paused client's cursor mid-walk); more moving parts than identity-only paging warrants; the natural choice **if** the revisit trigger fires (privileged state stays server-side keyed by a random token) |

## Consequences

- **Positive:** CIB-103 closes with a documented, test-pinned boundary and no new
  key-management surface. The cursor stays PV-2 restart-stable. The revisit
  trigger gives the Phase-2 snippet work (and any cross-tenant scoping) a hard,
  reviewable gate that forces the unforgeable construction exactly when it is
  needed.
- **Negative:** The cursor remains forgeable. This is safe **only** while the
  three revisit conditions hold; the safety is a property of the egress shape,
  not of the cursor itself, so it must be re-verified whenever the cursor's
  contents change.
- **Risks:** A future author adds privileged content to the cursor (snippets,
  scope) without re-opening this ADR, turning a forgeable token into a capability
  leak.
- **Mitigations:** The threat-model note and the binding revisit trigger live
  next to the cursor code; the pinning test asserts the current identity-only
  property so a change that violates it surfaces as a test to reconcile, not a
  silent regression. CE-5 (`anvil-gctx-types` no-leak test) independently guards
  the projection boundary.

## References

- APS: CIB-103 (this decision), CIB-099 (GCTX cross-surface hardening — parent
  split), GCTX-010/-011 (search/dependents tool surface), GCALL-007 (PV-9 egress
  ratification)
- ADR-084 (GCTX graph-handle access; daemon-side CE-5 projection choke point),
  ADR-086 (call-graph substrate; identity-only egress, snippets as a gated CE-1
  escalation)
- Code: `crates/anvil-gctx-egress/src/lib.rs` (`CursorPayload`, `encode_cursor` /
  `decode_cursor`, `query_fingerprint` / `dependents_fingerprint`, `fnv1a`,
  `MAX_CURSOR_BYTES`), `crates/anvil-gctx-types` (CE-5 no-leak test)
- Council: GCTX-011 review
  (`plans/reviews/post-merge/feat-gctx-011-find-dependents.md`)
