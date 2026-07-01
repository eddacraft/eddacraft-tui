# ADR-095: GCTX Read-Surface Consumers — MCP-First, CLI as Co-Equal Secondary Surface

## Status

Proposed

## Date

2026-07-01

## Context

The GCTX "dev acceleration" read verbs — `search_symbols`, `find_dependents`,
`find_callers`, `impact_of_change`, `affected_tests`, `symbol_context` — are
exposed **only** as MCP tools today (`crates/anvil-cli/src/mcp/tools/registry.rs:96`).
A recurring question is whether to also expose them via the `anvil` CLI, and if
so, how to frame the relationship between the two surfaces (in particular,
whether the CLI should be a runtime "fallback" for when MCP fails).

Two structural facts drive this decision.

**1. MCP and CLI are peers, not layers.** Both an MCP tool and any CLI command
are thin clients that call the *same* `daemon_rpc_call` (`crates/anvil-cli/src/mcp/gctx_client.rs:78`)
→ the same `anvil/gctx/*` RPC → the same daemon-side `GctxProjector`
(`crates/anvil-gctx-egress`, dispatched from `crates/anvil-intercept/src/save_time.rs`
via the `GctxDispatch` trait, per ADR-084). There is no MCP layer *beneath* the
CLI to drop down to. This means a CLI "fallback" can only rescue failures
**above** the shared daemon spine (MCP host/transport not wired, MCP process
crashed) and rescues **nothing** below it: when the daemon is down, both surfaces
return `Unavailable` over the same socket; when the graph is cold, both return
`NotReady`. There is no embedded graph fallback for GCTX (unlike `anvil hook`,
which has a local witness writer at `crates/anvil-cli/src/commands/hook.rs:1440`) — the daemon-warmed graph is
the only implementation. Daemon-down and cold-graph are exactly the *most likely*
failures in the non-agent contexts (fresh CI runner, cold hook) where one would
reach for a "fallback", so framing the CLI as a fallback promises rescue
precisely where it cannot deliver.

**2. The resource delta is client-lifecycle only.** Because the graph lives in
the daemon and projection runs daemon-side in both paths, the dominant RAM cost
(the graph) and the query CPU are **identical** regardless of caller. Measured on
the release binary (no daemon required for the client-side numbers; **directional
figures, not a benchmarked SLO** — the direction, amortised-persistent vs
per-invocation cold-start, is the load-bearing claim, not the exact values):

| | CLI (`anvil <verb>`) | MCP (`anvil mcp serve --stdio`) |
| --- | --- | --- |
| Model | new process per query | one persistent process per session |
| Resident RSS | ~9 MB transient, freed on exit | ~9.7 MB held for session (9 tokio threads) |
| Per-query overhead | ~2.0 ms spawn+init+teardown, **every call** | paid once at startup; then in-process call + 1 socket round-trip |
| Between queries | zero resident | ~10 MB idle |

The trade is therefore **call frequency**, not absolute cost: high-frequency
(agent loop, hundreds of calls/session) favours the amortised MCP process;
sparse/one-shot (a CI job running `affected_tests` once, a pre-push hook) favours
the CLI, where the ~2 ms spawn is noise and a resident MCP process would be pure
waste.

Precedent: **ADR-092** established the same "daemon-is-the-spine, MCP-is-one-
optional-client" posture at the *activation* layer (spine required; MCP an
optional L0 upgrade; corporate `--no-mcp` opt-out). This ADR applies the same
logic one layer over, at the GCTX read-surface *consumption* layer.

This decision was interrogated by a four-persona planning council
(kernel-maintainer, adversarial-reviewer, operations-reviewer, pragmatic-lead) and
subsequently reviewed by a five-persona ratification council (the same four plus a
council-reviewer, with a judge synthesis) on 2026-07-01; their findings are folded
into the Decision and Consequences below.

## Decision

1. **MCP-first.** The MCP tool surface remains the primary, high-frequency agent
   surface and is codified/completed first. It is the value-dense path and the
   resource model favours it for in-session agent use. No change to ADR-083/084.

2. **CLI is a co-equal *secondary* surface, not a runtime fallback.** A CLI query
   surface is a peer thin client over the same daemon RPC, positioned as:
   - **the primary surface for out-of-session consumers** — CI jobs, git hooks,
     humans, and non-MCP agents (for whom it is the *only* door, not a
     "fallback"); and
   - **a manual escape hatch** for the one failure it genuinely fixes: the MCP
     server not being wired into a given host (a developer whose editor lacks the
     MCP registration can still run `anvil gctx …`).

3. **No agent-triggered MCP→CLI runtime fallback (non-goal) — and an honest note
   on what that does and does not prevent.** An agent detecting an MCP failure and
   shelling out to the CLI in a loop is explicitly out of scope: it is the
   resource-worst path (per-call spawn), and the `Unavailable` outcome is a
   *structured success*, not an error (`crates/anvil-cli/src/mcp/tools/search_symbols.rs:110`), so it is not even
   a clean fallback trigger. Two caveats keep this honest: (i) this is a
   **non-goal we decline to build *for*, not something the CLI can technically
   prevent** — nothing distinguishes an agent's `Bash` invocation of `anvil gctx …`
   from a human's, so the reconstruct-via-CLI concern is an **owner-accepted
   residual**, not a closed hole; and (ii) that residual applies only to the
   **enumerable** verbs — an agent looping
   `Bash → anvil gctx {search_symbols,find_callers,find_dependents}` is functionally
   the `tools/call` back-door CIB-091d closed (the per-session egress ceiling is
   process-local, so it does not exist for that path), which is exactly why §6/§7
   gate those three behind a daemon-side egress counter. The two bounded-report v1
   verbs (`affected_tests`, `impact_of_change`) do **not** carry this reconstruct
   risk.

4. **Implementation shape.** Build the CLI command directly on the proto DTOs
   (`anvil_intercept_proto::protocol::Gctx*{Request,Response}`) + the existing
   crate-visible `daemon_rpc_call` — **not** over the MCP tool `call()` functions,
   which enforce a `workspaceRoot` trust boundary that exists only because an MCP
   client can assert an arbitrary root. A CLI process already knows its own root
   (`crate::util::workspace_root()`). Emit the raw daemon DTO via `--json`; do not
   wrap it in the MCP `content`/`isError` envelope, and skip `redact_workspace_root`
   (it hides the operator's path from a remote LLM provider — irrelevant when
   output goes to the operator's own terminal). No new crate, no new dependency;
   this is a thin command in the shape of the `anvil hook` precedent. To hold the
   two surfaces at outcome parity, the request-building and outcome-classification
   logic (including the exhaustive `should_rewarm` match over outcome variants) is
   extracted into a presentation-agnostic module (e.g.
   `crates/anvil-cli/src/gctx/query.rs`) that **both** `mcp/tools/*.rs` and the CLI
   call; the CLI must not reimplement per-verb logic, which is where the surfaces
   would otherwise drift (see the parity risk in Consequences).

5. **v1 scope + CI-contract prerequisites.** Ship read-only, `--json`-first,
   identity-only by default (snippets stay consent-gated daemon-side), starting
   with the CI-shaped verbs (`affected_tests`, `impact_of_change`) **behind an
   experimental gate** — concretely a `hide = true` clap subcommand plus a required
   opt-in (`--experimental` acknowledgement or `ANVIL_GCTX_CLI_EXPERIMENTAL=1`),
   mirroring the existing `gctx egress enable --yes` consent pattern
   (`crates/anvil-cli/src/commands/gctx.rs`) — so the cold-graph false-negative below cannot be wired
   into a CI gate before the contract lands. `symbol_context` (the sixth verb) is
   **excluded from the CLI at v1**: it can carry source snippets, so it falls under
   §6's snippet-egress revisit trigger and, if ever exposed, must be
   identity-only-by-default with snippets consent-gated and sit behind the §6/§7
   egress-counter prerequisite. Promotion past experimental requires all of:
   - **Distinct exit codes** for `NotReady` / `Unavailable` / `Disabled` /
     `InvalidQuery` vs a genuine `Ready`-with-empty result, with the code mirrored
     into the `--json` body (precedent: `crates/anvil-cli/src/commands/gate.rs`). Concrete mapping: reuse
     `EXIT_DAEMON_DOWN=6` for `Unavailable`; claim `8` for `NotReady` (cold/warming,
     recoverable) and `9` for `Disabled` (operator opt-out, not an error) — both are
     currently *reserved* in `crates/anvil-cli/src/main.rs` pending an ADR amendment, and **this ADR is
     that amendment**; map `InvalidQuery` to the existing `EXIT_ERROR=1` (caller
     bug). This closes the correctness defect where a cold-graph `NotReady` payload
     (which has *no* `tests` field) is read by a naive `jq '.outcome.tests | length'`
     as "0 affected" and exits 0 — a false negative that defeats the exact gate the
     verb is for.
   - **A CI-safe daemon warm path.** The `ensure_daemon` / `StartCapability`
     primitive is *already implemented* and wired into `anvil start` (`crates/anvil-intercept/src/ensure.rs`,
     `crates/anvil-cli/src/commands/start.rs`); the gap is not the primitive but a **caller** —
     `daemon_capability_for_start` resolves any non-interactive caller to
     `NoSpawn(NonInteractive)` ("never spawns or prompts") by ADR-082 design. The
     prerequisite is therefore a *new, explicit non-interactive opt-in* entrypoint
     (e.g. `anvil intercept ensure --yes` / an `ANVIL_*` acknowledgement) that
     passes `StartCapability::MaySpawn` from CI **without** weakening the ADR-082
     default for everyone else — an added opt-in branch, not a rebuild.
   - **A bounded `--wait[=timeout]`** for cold-graph warm-up (the DSV-045
     executor, ADR-085, makes `NotReady` recoverable-by-retry but publishes no
     bound to the caller), so a flapping cold graph can never flip a gate's
     PASS/FAIL (the Deterministic principle).

6. **Egress ceiling — resolved, pending owner sign-off.** `GRAPH_EGRESS_SPENT`
   exists to bound what a *remote LLM provider* can reconstruct through an MCP
   session (CIB-091d) — the identical threat model behind skipping
   `redact_workspace_root` for CLI in §4 ("irrelevant once output goes to the
   operator's own terminal"). For v1's scope (human/CI callers of the two
   bounded-report verbs) that purpose doesn't apply: a same-uid CLI caller
   already has full filesystem read of the repo, so the query grants no new
   confidentiality capability. `crates/anvil-intercept/src/dos.rs`'s `RpsBucket` is also the wrong
   mechanism to extend regardless of scope — its own module doc states it is
   "not a global rate limiter," a per-connection DoS-exhaustion budget, not a
   cumulative confidentiality-volume budget; generalising it would defend the
   wrong threat.
   The part that *does* carry over: a **non-MCP agent** (§2's third consumer
   type) whose only tool is Bash/CLI can reconstruct the whole graph via
   `search_symbols` / `find_callers` / `find_dependents` exactly as CIB-091d
   worried about for MCP — and since each CLI invocation is a fresh process,
   there is no session for a process-local counter to charge.
   **Decision: no new bound for v1** — the two CI-shaped verbs return bounded
   reports, not enumerable graph dumps, so the reconstruct-the-whole-graph risk
   doesn't apply to them. **Before promoting the three enumerable verbs** (see
   §7, same gate), add a daemon-side, peer-uid-keyed, time-windowed egress
   counter — a generalisation of `GRAPH_EGRESS_SPENT` itself (persisted across
   connections, not process-local, and charged identically regardless of
   transport), not an extension of `crates/anvil-intercept/src/dos.rs`.

   **Confidentiality, not DoS.** This decision concerns reconstruct/confidentiality
   only; volume/abuse of a running daemon stays bounded independently by the 4 MiB
   per-response cap and the per-connection rate bucket (`crates/anvil-intercept/src/dos.rs`), which remain in
   force for CLI callers regardless.

   **Binding revisit trigger** (beyond the enumerable-verb gate above): re-open
   this decision and require the daemon-side counter for *any* verb the moment
   either (i) the CLI surface begins serving **snippets/source text** (Phase-2
   CE-1 — the reason `symbol_context` is excluded at v1, §5), or (ii) the daemon
   becomes **multi-principal / remotely shared** (no longer same-uid-local — e.g. a
   shared CI or remote-host daemon), at which point the "caller already has `cat`
   access" premise no longer holds. *(Resolved 2026-07-01 by Claude during
   concurrent review; Morgan/owner should confirm before Accept.)*

7. **Pagination for the enumerable verbs — resolved, pending owner sign-off.**
   `anvil kindling` already establishes the house pattern for exactly this
   shape: `collect_daemon_rows` (`crates/anvil-cli/src/commands/kindling.rs:93`)
   auto-pages a keyset-cursor RPC to exhaustion inside the command, capped by a
   generous defensive `MAX_PAGES` that exists only to guard against a daemon
   that never terminates the cursor — the CLI never surfaces a raw `--cursor`
   flag to the caller. Apply the same shape to `search_symbols` /
   `find_callers` / `find_dependents`: page internally to exhaustion (or a hard
   `MAX_CURSOR_PAGES` bound) and return one assembled `--json` result, not a
   per-page terminal interaction.
   Auto-paging bounds a single *call* (the result is still shaped by the
   query's own filters) but does not substitute for the §6 counter, which
   bounds the *caller*: a non-MCP agent can still reconstruct the graph across
   many invocations with varying filters even if each individual call is
   internally bounded. §6 and §7 are the same promotion gate — resolve and
   implement together before these three verbs reach the CLI. *(Resolved
   2026-07-01 by Claude during concurrent review, building on the pagination
   gap originally flagged in this slot; Morgan/owner should confirm before
   Accept.)*

## Rationale

Framing the CLI as a *co-equal secondary surface chosen by consumer type* rather
than a *runtime fallback* is the crux. The peers-not-layers structure means a
"fallback" cannot rescue the daemon-down / cold-graph failures that dominate the
non-agent contexts, so the fallback framing over-promises and would burn trust on
first cold run. Meanwhile the real, honest value — reaching consumers MCP
structurally cannot (CI, hooks, humans, non-MCP agents) — is fully preserved, and
the resource model independently agrees the two surfaces are complementary
(amortised MCP for the agent loop; per-invocation CLI for sparse consumers).
Because both surfaces wrap the same daemon projection, "two surfaces" never means
"two implementations": the daemon stays the single source of truth (shared
`anvil-gctx-types` DTOs, daemon-side `GctxProjector`, CE-5 identity-only choke
point).

### Alternatives Considered

| Option | Pros | Cons |
| --- | --- | --- |
| **MCP-first + CLI as co-equal secondary surface (chosen)** | Honest about what each surface serves; no over-promise; single daemon implementation; matches resource model and ADR-092 posture | Two presentation/validation layers to keep at outcome/gating parity; CLI is a stable-contract commitment once promoted |
| MCP-primary + CLI runtime fallback (the initial proposal) | Intuitive "resilience" framing | Peers-not-layers means it cannot rescue daemon-down/cold-graph — the common non-agent failures; muddy trigger (`Unavailable` is structured success) |
| Agent auto-falls-back to CLI | "Just works" for the agent | Resource-worst (per-call spawn) *and* governance-worst (reopens the CIB-091d egress-ceiling hole); brittle detection |
| CLI-only (retire/deprioritise MCP) | One surface | Abandons the amortised high-frequency agent path the resource model favours; regresses the in-session product |
| Do nothing (MCP-only, status quo) | Zero new surface/contract | Leaves CI, hooks, humans, non-MCP agents with no access to the dev-acceleration verbs — where `affected_tests`/`impact_of_change` are most valuable |

## Consequences

### Positive

- Out-of-session consumers (CI, git hooks, humans, non-MCP agents) gain access to
  the dev-acceleration verbs via composable `--json` output, with near-zero new
  plumbing (reuses `daemon_rpc_call` + proto DTOs; no new crate/dependency).
- Improved debuggability: a human can run the exact query an agent ran and inspect
  the raw outcome (`Ready`/`NotReady`/`Unavailable`/`Disabled`).
- The daemon remains the single source of truth; MCP and CLI cannot diverge in
  projection/gating because both consume the sealed daemon DTO.

### Negative

- A promoted CLI verb is a pinned public contract (output shape depended on by
  scripts), carrying the same frozen-DTO discipline as `policy eval --json` v1.
- Two presentation/validation layers must be kept at outcome parity (mitigated by
  the shared daemon projection, but the envelope and exit-code mapping differ).

### Risks

- **Cold-graph false negative** (correctness): a `NotReady` payload read as "0
  affected" silently passes a CI gate. This is the sharpest risk and is treated as
  a must-fix prerequisite (§5), not a nicety.
- **Egress observability/bound gap**: session-less CLI callers bypass the
  process-local byte ceiling entirely. Resolved for v1 (no reconstruct-the-graph
  risk in the two bounded-report verbs); the gap re-opens for a non-MCP agent
  the moment `search_symbols`/`find_callers`/`find_dependents` are promoted. See
  §6/§7 (same gate).
- **Determinism**: unbounded cold→warm flapping could make a gate non-deterministic
  absent `--wait`.
- **Unbounded pagination on the enumerable verbs**: without the §7 auto-page
  shape, a naive per-page `--cursor` CLI design would reintroduce the
  "unbounded dump" failure mode this ADR otherwise avoids.

### Mitigations

- Distinct exit codes + `--wait` + a CI-safe daemon warm entrypoint gate promotion
  past experimental (§5).
- v1 ships with no new egress bound (§6 resolution: the two CI-shaped verbs don't
  carry the reconstruct-the-graph risk). A daemon-side, uid-keyed, persistent
  egress counter is a hard prerequisite before `search_symbols`/`find_callers`/
  `find_dependents` reach the CLI (§6/§7).
- `search_symbols`/`find_callers`/`find_dependents` auto-page to exhaustion
  server-side (kindling's `collect_daemon_rows`/`MAX_PAGES` shape) rather than
  exposing a raw `--cursor` flag (§7).
- CI recipes in `--help`/docs must demonstrate `jq -e '.outcome.status == "ready"'`
  before touching result fields, and require `ANVIL_TRACE_SINK` capture when a CI
  entrypoint starts the daemon.

## References

- Related ADRs: [ADR-083](083-gctx-mcp-delivery-target.md) (GCTX MCP delivery),
  [ADR-084](084-gctx-graph-handle-access.md) (`anvil/gctx/*` RPC + daemon-side
  projection), [ADR-085](085-daemon-full-scan-executor.md) (cold-graph warm-up
  executor, DSV-045), [ADR-091](091-gctx-cursor-fingerprint-integrity.md) (cursor
  is a seek position, not a capability token),
  [ADR-092](092-mcp-optional-activation-spine.md) (daemon-spine / MCP-optional
  precedent at the activation layer),
  [ADR-082](082-daemon-lifecycle-user-startup.md) (no non-interactive auto-spawn)
- Code: `crates/anvil-cli/src/mcp/tools/registry.rs`,
  `crates/anvil-cli/src/mcp/gctx_client.rs`,
  `crates/anvil-cli/src/commands/hook.rs` (CLI→daemon-RPC precedent),
  `crates/anvil-cli/src/commands/gctx.rs` (existing operator-only `gctx` command),
  `crates/anvil-intercept-proto/src/protocol.rs` (`anvil/gctx/*` + DTOs),
  `crates/anvil-intercept/src/save_time.rs` (`GctxDispatch` impl + telemetry),
  `crates/anvil-intercept/src/dos.rs` (`RpsBucket` — per-connection DoS budget,
  ruled out as the egress-ceiling mechanism in §6),
  `crates/anvil-cli/src/commands/kindling.rs:93` (`collect_daemon_rows` —
  auto-page-to-exhaustion precedent cited in §7)
- APS modules: GCTX (graph-context), DSV (daemon-save-time-validation)
