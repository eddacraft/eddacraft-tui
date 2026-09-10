# MCP attach: evidence and implementation reconciliation

| Type | Authority | Owner | Status | Freshness |
| --- | --- | --- | --- | --- |
| Spec | Advisory | JREL | Proposed | Reviewed 2026-09-06 against main `46e52dddecb56b14e23d4a9a520f9b539b1af14f` and PR #4416 head `849723fb7e1ce9bf3fb0167f38427c85cbf19248` |

| Upstream | Downstream |
| --- | --- |
| Operator 2026-09-05 attach note, [JREL](../archive/modules/journey-reliability.aps.md), [JOURNEY](../modules/release-user-journeys.aps.md), [ADR-036](../decisions/036-daemon-scope-discovery-and-boundaries.md), [ADR-094](../decisions/094-worktree-registration-ux.md), [ADR-140](../decisions/140-converged-command-event-contract.md), ADR-141 in PR #4416 | [JREL attach checkpoints](../execution/JREL-attach.actions.md), JREL-002, JREL-004, JREL-005, JREL-010, JREL-011, JREL-012, JOURNEY-014, JOURNEY-015 |

## Purpose and authority

Review the operator-supplied 2026-09-05 attach failure note and make its
remaining work executable through [JREL](../archive/modules/journey-reliability.aps.md).
[JOURNEY](../modules/release-user-journeys.aps.md) remains the acceptance and
release conductor. This document proposes evidence semantics; it is not an
as-built claim or a second backlog. See the bounded
[execution checkpoints](../execution/JREL-attach.actions.md).

## Findings and evidence limits

The reported 92 shims, two listening sockets and Homebrew 0.9.7-beta behaviour
are operator observations, not reproductions performed for this review.
Current-main source confirms the disconnected paths. Release/formula currency
must be checked again during the existing release handoff.

| Finding | Source disposition | Existing owner |
| --- | --- | --- |
| MCP does not register a live lease on main | Main MCP module has no client-session module; open [PR #4416](https://github.com/eddacraft/anvil-001/pull/4416) adds one | JREL-002; reuse claim #4408 |
| Durable-only evidence can promote one configured client | `activation/daemon_evidence.rs` retains the unique-HSV fallback on main; #4416 removes it and introduces per-session freshness | JREL-002 |
| Heartbeats prove attachment, not completed validation | #4416 promotes using a live lease; it does not introduce session-correlated scan evidence | JREL-002 acceptance residual; JREL-005 projection |
| MCP status reports `local` / `not-wired` | Literal values remain in `mcp/tools/status.rs` | JREL-005 |
| Runtime/state-home discovery can disagree | Operator reproduction; JREL-004 already owns cross-endpoint convergence and recycle | JREL-004, JREL-011 |
| Working-directory identity differs from intended workspace | #4416 `server_worktree()` uses current directory; launch/root admission still needs integration evidence | JREL-010 |
| Durable heartbeat age incorrectly implies death in the reported release | Main already contains snapshot-clock handling from CIB-384; do not reimplement | JOURNEY-015 release inclusion |
| Save-time recovery and graph warming look like attach failure | Distinct components, already represented in JREL | JREL-003, JREL-005, JREL-012 |

The note's per-user-singleton language needs correction:
[ADR-036](../decisions/036-daemon-scope-discovery-and-boundaries.md) requires
one daemon within an execution scope and allows multiple scopes. Preserve its
unsupported-container caveat and Windows/WSL refusal. Intentional isolated
ANVIL_HOME installations also remain distinct under JREL-004. Do not flatten
these boundaries into a universal lock.

## Proposed evidence contract

Prefer completing the existing registration model over weakening `Protecting`.
Do not replace the durable activation spine: membership and live participation
have different lifetimes under
[ADR-094](../decisions/094-worktree-registration-ux.md).

The useful contract is common provenance, not equality between an `allow`
decision and a green headline. An embedded fallback can return allow without
daemon protection; a healthy daemon can return block because it found a
violation. A completed warn/block scan is still evidence of working validation.
MCP validation is also cooperative coverage of calls that reach it, not proof
that every editor write is intercepted.

| Fact | Required evidence | Does not prove |
| --- | --- | --- |
| Configured | Managed/readable integration entry | Editor loaded it |
| Server startable | Disposable transport/protocol probe succeeds | Real editor attached or validated |
| Attached | Admitted live session with its own lease and daemon generation | A write was validated |
| Validation observed | Completed daemon scan attributed to this session and worktree, with outcome/time | Other sessions or bypassed writes are covered |
| Pre-write coverage ready | Current admitted attachment, available validation capability, applicable policy/fence state and matching live daemon | Save-time watcher or graph readiness |
| Watching | Ready save-time driver for the admitted worktree | Live MCP pre-write coverage |
| Graph ready/stale/warming | Graph's own generation, completeness and reason | Pre-write health |

JREL-002 must settle the distinction between attached/ready and validation
observed before claiming its existing outcome complete. If `LiveValidation`
continues to mean an observed call, require correlated completed-scan evidence;
otherwise amend its name/documentation through the owning decision and migrate
the public contract explicitly. Do not merely rename a live lease as validation.
Historical last-validation time need not expire because an editor is idle:
current lease/capability and historical validation are separate facts.

JREL-005 should present scoped pre-write readiness as `Protecting`, with
last-validation evidence separately visible, subject to that explicit decision.
An aggregate must identify which live sessions are covered and which are
degraded; one good session must not imply all open editors are protected.
Configured-but-closed integrations remain inventory, not failing live sessions.
Optional MCP and save-time-only operation remain supported.

Reuse and, where needed, amend ADR-141 in #4416 rather than writing a competing
attach ADR. Its head currently says Accepted; that is a branch-local statement,
not evidence of merge or operator acceptance verified here. Public semantic
changes remain subject to the normal ADR process. This plan does not approve
that ADR or authorise merging #4416.

## Identity, lifecycle and boundedness

Required logical correlation is execution scope, daemon instance/generation,
canonical admitted worktree, live session, client family and observed request.
Reuse existing identity types before adding fields. Session identity must
distinguish two instances of the same editor, not only two client families.
Client-provided names are display/classification metadata, never authority.
Preserve peer checks and the OS-account trust boundary in
[ADR-140](../decisions/140-converged-command-event-contract.md).

Resolve daemon identity once for an attached generation and use it for
registration, heartbeat, validation, graph and status. Reconnect explicitly when
that generation disappears; old evidence cannot follow the new generation.
Endpoint discovery may change only after verification. If two same-scope live
daemons conflict, report the conflict and offer bounded recovery; never choose
one silently or signal a process using stale PID evidence.

Registration follows a valid protocol session and admitted root. The disposable
probe is explicitly non-participating, even if it sends a recognisable client
name. Unknown client names remain unknown; absence of a name cannot borrow a
configured client's identity. Decide generic attachment diagnostics explicitly
without expanding named-client protection claims.

Cover daemon absent at attach, startup refusal, daemon restart, lease expiry,
re-registration, EOF, cancellation and heartbeat-thread failure. Re-registration
is idempotent per session/generation, bounded, and never replays tool mutations.
On EOF unregister promptly; on abrupt death use TTL. Idle but connected sessions
are not orphans merely because they have not called a tool recently. Do not kill
all matching MCP processes as a repair strategy. Real process-start identity or
an explicitly typed unavailable value must replace invented PID-as-starttime
claims where the platform cannot supply the same evidence.

JREL-010 owns canonicalisation/admission across root, package directory, linked
worktree and symlink. Settle its root contract before finalising attachment
identity, despite its implementation dependency on JREL-002. Roots discovered
through protocol metadata still pass existing admission; never silently trust
arbitrary caller paths or nested repositories. One server serving multiple roots
must either support separately admitted bindings or clearly reject that mode.

JREL-011 owns the added scale acceptance: around 100 idle sessions plus concurrent
status and validation must remain within documented connection, byte and latency
budgets. Transient heartbeats avoid reserving one connection per shim, but do not
prove burst safety. Test synchronised reconnects, refusal/backoff and shutdown.
Measure full `QueryStatus` payload growth; implement a bounded scoped attestation
query if needed to meet the measured gate, with protocol compatibility tests.
CIB-405 remains the connection-reuse owner. No unbounded session list or graph
scan may enter a pre-write critical path.

## Integration and release gates

Use JREL-012's real binary/process harness from the start. Required fixtures:

1. Disposable probe and durable-only membership never create live-client proof.
2. One editor, two different editors, two instances of one editor, nine configured
   editors but one open, and an unknown client produce independently scoped facts.
3. An actual stdio tool call reaches daemon `scan_buffer`; allow, warn and block
   outcomes carry matching worktree/session/generation provenance across MCP
   status and CLI status. Comparing different snapshot times accounts for changes.
4. Closing one editor, daemon loss/restart, lease expiry and PID reuse cannot
   preserve or transfer its protection claim. No editor restart is required just
   because daemon reconnect is recoverable.
5. XDG set/unset in both start orders, concurrent starts, stale endpoint plus
   live sibling, duplicate same-scope daemons and intentional isolated homes.
6. Root/subdirectory/worktree/symlink, rejected nested root, outside-repo launch,
   plus Linux, macOS and Windows named-pipe legs; unsupported boundaries refuse.
7. Graph scan timeout with healthy pre-write, watcher absent/disabled/failed,
   optional MCP and save-time-only coverage produce separate truthful outcomes.
8. Around 100 sessions under heartbeats/reconnect bursts do not starve validation
   or status. End-to-end deadlines include discovery, locks and cleanup.
9. Previous public build upgrade, mixed old/new shim-daemon versions, and a
   verified package-manager install retain protocol compatibility or give precise
   reconnect/upgrade guidance. Observe beyond both 45 seconds and lease expiry.

Synthetic stdio clients prove transport integration, not real editor config
loading. JOURNEY-015 additionally records real-client evidence for supported
release claims, prioritising Claude Code, Codex, Grok and OpenCode, plus the
Cursor scenario in the operator note. Missing clients/platforms are explicit
evidence gaps, never silent successful skips.

JOURNEY-014 consumes JREL closure evidence. JOURNEY-015 checks pinned-main
rehearsal, release tag/asset/formula inclusion and an installed-binary run using
the existing release process. Shipping CIB-384 alone is partial relief, not
attach closeout. JSIMP follows reliability acceptance; no new internal release
channel or universal control-plane rewrite is necessary.

## Documentation impact

This proposed specification and its action plan are supporting planning context;
JREL remains execution authority and the existing APS index remains canonical.
Implementation closeout updates the owning activation/MCP/intercept as-built
docs, status JSON contract, recovery guidance and relevant diagrams together.
No as-built diagram or public feature claim changes in this planning intake.
