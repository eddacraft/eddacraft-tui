# ADR-097: Allowlist confinement admits only explicit operator entries

## Status

Proposed

> Amends [ADR-061](061-save-time-daemon-delta-validation.md) §7's confinement
> clause ("Authorisation, read-safety, and confinement"). This uses a fresh
> sequential number rather than an `061a` suffix because the repo's ADR
> integrity check (`scripts/docs/adr-integrity.sh`) treats a lettered variant as
> occupying the bare slot and rejects it coexisting with `061`.
> ADR-061 as a whole remains **Accepted** and in effect; only the
> implicit-primary admission rule changes. The trust boundary, authority,
> read-safety, Windows, and placement rules of §7 are unchanged.

## Date

2026-07-04

## Context

ADR-061 §7 defined `allowlist` confinement mode with this rule:

> "…the daemon refuses any non-admitted root with a structured
> `workspace-not-admitted` code and disables first-touch auto-adopt; **the
> primary check-in root is implicitly admitted, the allowlist governs additional
> roots.**"

The intent was operator convenience: even with an empty allow list, the
connection's "own" worktree would still be served so confinement never locks a
working agent out of the repo it is editing.

CIB-149 (deepsec sweep) found that this implicit-primary rule is a **same-uid
confinement bypass**. There is no daemon-verified notion of a connection's
"primary" worktree: the value was derived from a **client-declared wire field**
— first the first-named `validate_paths.workspace_root`, and (after an
intermediate fix attempt) the `RegisterSession.worktree`. Both are supplied by
the connecting peer. A same-uid client could therefore admit any unlisted root
in `allowlist` mode simply by naming it first, defeating the operator's allow
list entirely.

This is a direct consequence of §7's own trust-boundary statement: *"SO_PEERCRED
uid == daemon uid is the real and only trust boundary. Within one uid there is
no cross-workspace boundary to enforce."* Within a single uid the daemon cannot
distinguish a legitimately-owned worktree from an attacker-named one — both are
client-declared — so an "implicitly admitted primary" can never be more
trustworthy than a first-named wire root. §7 had already dropped the
`/proc/<pid>/cwd` check for the same reason ("the wrong gate… adds no security
within a uid"), which removes the only other candidate source for a
daemon-derived worktree.

The security fix (PR #3117, merged 2026-07-03) removed implicit-primary
admission entirely. This ADR ratifies that change and reconciles the ADR text so
the shipped behaviour no longer contradicts a normative Accepted ADR. The
post-merge council review (all four personas: security-analyst,
adversarial-reviewer, operations-reviewer, pragmatic-lead) confirmed the code is
correct, the bypass is closed, and fail-closed-by-removal is the correct
**permanent** design rather than a stopgap.

## Decision

In `allowlist` confinement mode the daemon admits **exactly** the
operator-configured allow entries (exact + prefix), and nothing implicit:

- **No implicit primary.** There is no "primary check-in root" that is admitted
  by virtue of a connection having named or registered it.
- **No first-touch adoption** (unchanged from ADR-061 — this was already
  disabled in `allowlist` mode).
- **No lineage/registry-derived admission.** A `RegisterSession.worktree` and
  the PID-lineage session registry do not feed the admitted set; a registered
  worktree must be added via `anvil workspace allow` like any other root.
- **Empty allow list admits nothing** (fail-closed). Config-load failure
  continues to fail closed and loud (unchanged from ADR-061 §7).
- **`open` mode is unchanged** — it still adopts any nameable root first-touch
  by design. The posture is therefore binary: `open` (adopt all within the uid)
  or `allowlist` (admit only the operator's explicit entries).

This amends **only** the confinement bullet of ADR-061 §7. As shipped
(`crates/anvil-intercept/src/{confinement.rs,save_time.rs,ipc.rs}`):
`Confinement::to_admitted_roots()` takes no path argument and builds the
`allowlist` set from the configured exact + prefix entries alone;
`SaveTimeConn::verified_primary`, `Confinement::is_allowlist`, and
`ipc::seed_save_time_verified_primary` are removed;
`SaveTimeConn::set_originating_session` retains the wire worktree for **telemetry
correlation only** and never touches the admitted set.

Because this constrains authority within the same-uid boundary that ADR-061 §7
already declares to be the hard boundary, it is a **guardrail tightening**, not a
new security guarantee: the hard boundary for untrusted code remains running the
agent under a separate OS user.

## Rationale

A client-declared worktree can never be "verified" under the same-uid trust
model, so any rule that admits a root *because a connection named/registered it*
is exploitable. Requiring an explicit operator allow entry is the only rule that
holds, and the operator allow list already lives in owner-only config the
confined agent cannot edit (ADR-061 §7 placement).

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| **Explicit entries only (chosen)** | Closes the same-uid bypass with a simple, deterministic contract; allow list is the single source of admission | Removes the zero-config convenience of auto-admitting a connection's own worktree; operators must enumerate worktrees (or use a prefix entry) |
| Restore implicit primary from a "daemon-verified" source | Preserves zero-config convenience | Infeasible under same-uid: no worktree is daemon-attestable. The only attestable variant is a narrow "daemon itself launched the process into this worktree" table, which does not cover editor/MCP clients; §7 already rejected the `/proc/<pid>/cwd` gate. A general attested peer→worktree binding is real infrastructure work, not a quick follow-up |
| Revert PR #3117 | Restores prior behaviour | Reopens the exact CIB-149 same-uid bypass; strictly worse than accepting the UX regression |

## Consequences

- **Positive:** the CIB-149 same-uid bypass is closed on every admission path
  (`validate_paths`/GCTX `workspace_root`, `RegisterSession.worktree`, the
  lineage registry); the `allowlist` contract is now a single deterministic rule
  (admit ⟺ matches an operator allow entry) with no client-supplied input.
- **Negative:** operators who ran `allowlist` mode relying on their own worktree
  being auto-admitted will now see it **refused** until they add it via
  `anvil workspace allow <root>`. ADR-079/ACTMO-019 `register_on_start`
  worktrees likewise get no implicit admission in `allowlist` mode.
- **Risks:** the "enumerate every worktree" burden can push operators toward a
  broad prefix entry (e.g. all of `$HOME`), which is far more permissive than
  the single-worktree admission this removes. The filesystem-root prefix guard
  only rejects `/`, not a broad `/home/<user>` prefix.
- **Mitigations:** operator-facing docs and `anvil workspace` output are
  corrected to describe the fail-closed contract and to prefer exact per-worktree
  entries over broad prefixes (CIB-161, PR #3120); the empty-allowlist refusal
  now emits a server-side diagnostic naming the refused root and the
  `anvil workspace allow` remediation. A genuinely daemon-attested peer→worktree
  binding, if ever pursued to restore zero-config admission, is scoped as
  separate infrastructure work under a future ADR, not a blocking gap.

## References

- Related ADRs: [ADR-061](061-save-time-daemon-delta-validation.md) §7 (amended
  clause), [ADR-063](063-gv2-hot-path-boundary.md) (hot-read boundary over the
  same wire)
- APS items: CIB-149 (the bypass + fail-closed fix), CIB-161 (doc/CLI/diagnostic
  reconciliation), CIB-150/CIB-151 (sibling deepsec same-uid fixes)
- PRs: #3117 (CIB-149 fail-closed fix, merged), #3120 (CIB-161 reconciliation)
- Review: `plans/reviews/post-merge/cib-149-post-merge-council-review.md`
