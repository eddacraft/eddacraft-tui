# GCTX-in-harness unblocker (CCTX V2/V4)

| Type | Authority | Owner | Status | Freshness |
| ---- | --------- | ----- | ------ | --------- |
| Spec | Advisory | CCTX | Draft (eval harness diagnosis) | 2026-09-08 — probed Cursor Cloud run `bc-7ec129c3-83d7-417a-bbe2-1d5ed37d2c56` against the same class of harness that blocked CCTX-003 V2/V4 |

| Upstream | Downstream |
| -------- | ---------- |
| [CCTX-003 comparison](./2026-09-08-cctx-003-baseline-comparison.md), [Context Compiler spec](../specs/2026-09-08-context-compiler.md) §12.2 / §12.7, [AI context delivery](../../docs/guides/ai-context-delivery.md), [ADR-083](../decisions/083-gctx-mcp-delivery-target.md), [ADR-084](../decisions/084-gctx-graph-handle-access.md), [ADR-095](../decisions/095-gctx-cli-secondary-surface.md), [ADR-082](../decisions/082-daemon-lifecycle-user-startup.md), [ADR-114](../decisions/114-bare-anvil-ensure-surface.md), [GCTX dogfood failure points](../../docs/archive/reviews/2026-08-16-gctx-dogfood-failure-points.md) | [CCTX module](../modules/context-compiler.aps.md) CCTX-005, [eval fixtures README](../evals/context-compiler/2026-09-08/README.md) |

Internal evaluation harness note. **No product ships.** This does not authorise
a compiler, knowledge store, crate, feature flag, GCTX DTO change, GATT fork,
or enforcement change. It does **not** unpark spec §15. A Decision Brief and
every GCTX answer remain **never** allow / warn / block.

Live V2/V4 sessions were **not** run. Spec §12.7: if GCTX is unavailable, V2
and V4 are **blocked**, not silent V1 substitutes.

## 1. What failed in the CCTX-003 cloud harness

CCTX-003 recorded GCTX unavailable and stopped. This note explains **why**,
from a fresh probe of the same Cursor Cloud class of environment (personal
snapshot, `CURSOR_AGENT=1`, grok-model session).

| Probe | Result (2026-09-08, this run) |
| ----- | ----------------------------- |
| `anvil` on `PATH` | **absent** (`command -v anvil` empty; no binary under `/usr/local`, `/home/ubuntu`, `/opt`, `/workspace`) |
| Native MCP tool catalog | **no** `anvil_*` tools; catalog is platform MCP servers (GitHub, Vercel, Gmail, …) plus Cursor-internal tools |
| `~/.cursor/mcp.json` / workspace MCP config | **absent** |
| `ANVIL_*` / `GCTX_*` env | **unset** |
| Daemon / `ANVIL_HOME` / intercept socket | **absent** |
| `anvil-developer-functions` skill | **not** in the session skill list (anvil-managed; `eddaskills.toml` vendors `using-anvil` only) |
| Snapshot install log (`bld-20260908-4ccbffdb-…`, SYSTEM recurring) | clone + exec-daemon + VNC + git/`gh` — **no anvil install** |
| Environment config | personal, db-backed; `environment.json` not exposed to the agent |
| Harness identity | `CURSOR_AGENT=1`; `CLAUDECODE`, `CODEX_THREAD_ID`, `OPENCODE`, `GROK_AGENT` all empty (same degradation CCTX-003 recorded) |
| ADR-095 CLI query verbs (`anvil gctx search-symbols`, …) | **not implemented**; `anvil gctx` today is egress consent only |

CCTX-003's machine record already stated the same binary/MCP absence:
[`plans/evals/context-compiler/2026-09-08/runs/2026-09-08-fixture-score.json`](../evals/context-compiler/2026-09-08/runs/2026-09-08-fixture-score.json).

This is **not** the 2026-08-16 Grok dogfood failure (MCP attached, graph
`not_ready` / scan-timeout). That class is CIB-341/342. Here the tools never
appear.

## 2. Why the tools cannot appear from inside the session

Four independent gaps. Closing only one is not enough.

1. **Binary install.** The cloud snapshot does not ship `anvil`. Cargo and
   Node are present, so an in-session `cargo install` of `eddacraft-anvil` is
   *possible* and still **not** a V2 session: it does not inject MCP tools
   into the agent's native catalog, and it is not the pinned corpus binary.
2. **MCP wiring into Cursor Cloud.** Desktop Cursor reads
   `~/.cursor/mcp.json` (or `anvil mcp install --client cursor`). Cloud
   agents receive MCP tools from the **platform-injected** catalog at session
   start. Writing `mcp.json` during a run does not add `anvil_search_symbols`
   to `GetDynamicTools`. That wiring is an operator / Cursor Cloud
   environment action, not a CCTX product change.
3. **Daemon spine (ADR-084).** GCTX projection is daemon-side. No intercept
   process, no socket, no warm graph. Non-interactive callers default to
   no-spawn (ADR-082). A V2 session needs an explicit, consented daemon
   ensure at the **corpus worktree root**. ADR-114's bare `anvil` is the daily
   ensure only after activation already exists.
4. **Agent skill.** Graph-tool choreography lives in
   `anvil-developer-functions`, installed by `anvil skill install`. Without
   the binary, that skill never lands. `using-anvil` is vendored but cannot
   run `anvil start --verify`.

**Do not substitute:**

| Tempting stand-in | Why it is not V2 |
| ----------------- | ---------------- |
| Ordinary file search / reads (V1) | Spec §12.4: V2 must not pretend unbounded search is GCTX |
| `anvil gctx egress status` | Consent only; not a §12.2 read verb |
| Shelling out to hypothetical `anvil gctx search-symbols` | ADR-095 CLI query verbs are not shipped; spec §12.2 is the MCP tool list |
| Invented graph answers from memory | Hidden stale use; protocol failure |
| Brief fixtures (V3) | Different variant; not a GCTX baseline |

## 3. What V2/V4 sessions require

Owner of each row is residual ownership, not a promise this stub will build it.

| Requirement | Why | Owner |
| ----------- | --- | ----- |
| `anvil` on `PATH`, version compatible with the **corpus revision** | MCP serve + daemon RPC client | Cursor Cloud environment (operator snapshot / install script) |
| Platform MCP entry: `anvil mcp serve --stdio` visible as native tools | Spec §12.2 tools must be callable as tools, not reconstructed | Cursor Cloud MCP catalog + operator; **cannot** be done from inside a session |
| Daemon running against the **corpus worktree git root** | ADR-084; nested `workspaceRoot` is refused (CIB-398 / ADR-125) | Eval operator after env install; needs ADR-082-compatible non-interactive ensure |
| Graph `ready` (not `unavailable` / looping `not_ready`) | Cold or scan-timeout answers are not a V2 baseline | Eval operator; if anvil-001 still hits scan-timeout, that is **CIB-341/342**, not CCTX |
| Corpus throwaway worktree at the pinned SHA | Spec §12.7 step 1; do not score a dirty feature branch | Eval operator |
| Egress consent recorded (default **off**) | Identity-only unless a task explicitly needs snippets | Eval operator; `ANVIL_GCTX_EGRESS` unset is correct |
| Harness identity + model recorded | §12.7 step 2 | Eval operator (`CURSOR_AGENT` / `GROK_AGENT` / …) |
| `anvil-developer-functions` (or equivalent) loaded | Agents must prefer GCTX tools over blind reads | Follows binary + `anvil skill install` |
| One session per `(task, variant)`; no reused memory | §12.7 | Eval operator |
| `anvil_validate_write` still available | Enforcement stays on a separate path | Same MCP server; never treat GCTX as a gate |

ADR-095 CLI as a co-equal secondary surface remains **Accepted** and
**unimplemented** for the read verbs. Authorising a CLI-via-shell V2 would be
a protocol change to spec §12.2, not a silent workaround. CCTX-005 does not
authorise that change.

## 4. Minimal smoke checklist

Run **before** dispatching any V2 or V4 session. If any hard row fails, V2
and V4 stay **blocked**. Do not start a 20-task pass.

**Hard (must all pass)**

1. `command -v anvil` and `anvil --version` succeed. Record the version next
   to the corpus SHA.
2. The agent's native tool list includes at least `anvil_search_symbols`,
   `anvil_find_dependents`, `anvil_find_callers`, `anvil_impact_of_change`,
   `anvil_affected_tests`, and `anvil_symbol_context` (spec §12.2). Resources
   `graph://stats` / `graph://symbols` / `graph://edges` if the harness
   exposes MCP resources.
3. `workspaceRoot` for those calls is the corpus worktree git root, not a
   nested directory.
4. One identity-only `anvil_search_symbols` call (short name, default
   egress) returns outcome `ready`. `unavailable` → daemon missing.
   Persistent `not_ready` → graph not usable for a baseline (see CIB-341).
5. Corpus `HEAD` equals the pinned eval SHA. Working tree clean for eval
   inputs.
6. Snippet egress is **off** unless the run record says otherwise
   (`anvil gctx egress status` once the binary exists; else record "binary
   absent, treat as default off").

**Record (do not skip)**

7. Harness identity signals (`CURSOR_AGENT`, `GROK_AGENT`, `CLAUDECODE`,
   `CODEX_THREAD_ID`, `OPENCODE`) and model id.
8. Whether `anvil_validate_write` is present (required on all variants; not
   a GCTX substitute).
9. A dated run record path. CCTX-003's fixture score is **not** a V2 record.

**Then, and only then**

10. Dispatch live §12.7 sessions: 20 × V2, and 20 × V4 using the existing
    fixtures' GCTX drill-down handles. Do not reuse a V1 or V3 session.

## 5. Residual ownership

| Residual | Owner | This stub |
| -------- | ----- | --------- |
| Snapshot / PATH install of `anvil` | Cursor Cloud environment (operator) | Documents the gap; does not change the snapshot |
| Inject anvil MCP into the cloud-agent tool catalog | Cursor Cloud MCP + operator | Documents that in-session `mcp.json` is insufficient |
| Non-interactive daemon ensure at corpus root | Anvil intercept / ADR-082 follow-on if the env cannot spawn | Out of CCTX product scope |
| anvil-001 full-scan timeout / stale graph | CIB-341 / CIB-342 (do not edit CIB from this PR) | Linked only |
| ADR-095 CLI read verbs | Anvil GCTX CLI (not started here) | Not a V2 stand-in until spec §12.2 says so |
| Live V1 sessions | Separate eval follow-on (CCTX-003 residual) | Not this note |
| Live V2/V4 after smoke passes | **CCTX-005** (Draft; [private issue #4469](https://github.com/eddacraft/anvil-001/issues/4469)) | Parking stub only |
| Spec §15 including §15.6 | Parked | **Still parked** |
| Product compiler / GATT fork / enforcement | Out of scope | Not started |

CCTX-005 is **Draft**. This note is the diagnosis artefact. Starting harness
enablement or a 20×V2 pass requires a later Ready pickup of CCTX-005, not
this PR.

## 6. Change-impact (docs)

| Concern | Disposition |
| ------- | ----------- |
| New documentation unit `plans/audits/2026-09-08-gctx-in-harness-unblocker.md` | Eval/harness diagnosis; advisory; linked from CCTX module, spec §12.8, CCTX-003 residual, eval README. Not a DOCRB inventory component |
| Public contracts / GCTX / GATT / CEG / enforcement | Unaffected — no product behaviour change |
| Diagrams | Unaffected — no topology or trust-boundary diagram change |
| Spec §15 | Unaffected — remains parked |
