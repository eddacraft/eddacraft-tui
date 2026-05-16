# Deep Code Review: anvil-001

## Executive assessment

My judgment is that the repository has a **promising Rust control-plane
foundation**, but it is **not yet coherent enough to claim a single
deterministic enforcement core**. The codebase is moving in the right general
direction on implementation language and low-level hardening, but the product
authority is still split across several overlapping paths: CLI-local checks,
CLI-local gates, watcher-triggered subprocesses, MCP pre-write validation, a
daemon IPC surface, and a separate policy engine that still shells out to OPA.
That is the central architectural issue. fileciteturn29file0
fileciteturn30file0 fileciteturn31file0 fileciteturn34file0
fileciteturn43file0 fileciteturn48file0 fileciteturn53file0

The **biggest architectural risk** is that the repository currently presents the
intercept daemon as the enforcement authority, but the daemon runtime I
inspected does not actually wire live filesystem events into a running
enforcement loop. The module plan marks watcher integration complete, yet the
watcher integration module still contains a placeholder `recv_blocking` that
intentionally never receives production batches, and `run_foreground` stands up
PID handling, fence loading, IPC, registry, and status, but not a real watcher
bridge. In plain language: the daemon looks architecturally important, but the
hot path that would make it the real local authority is not convincingly live.
fileciteturn55file0 fileciteturn46file0 fileciteturn43file0

The **biggest product-readiness risk** is documentation and planning drift.
Internal materials disagree with each other. The architecture overview still
describes a TypeScript-centred runtime with Rust as a parser/grammar assist,
ADR-014 still codifies “TypeScript by default,” the decision log says ADR-040
chose Regorus over external OPA binaries, and the intercept daemon module says
INTD is fully complete, while the code still shells out to `opa` and the watcher
bridge is not properly wired. That level of drift will mislead both human
contributors and agents. fileciteturn18file0 fileciteturn16file0
fileciteturn17file0 fileciteturn53file0 fileciteturn55file0
fileciteturn46file0

The **strongest part of the codebase** is the security-minded hardening in the
pre-write and IPC paths. The MCP validate-write surface rejects deleted server
cwd scenarios, workspace escapes, `..` traversal, symlink escapes,
non-UTF-8/binary-like content, and oversized buffers. The IPC layer is also
careful about socket directory ownership/mode, socket-file permissions, symlink
refusal, and same-UID peer validation on Unix. That is real engineering
discipline, not cosmetic polish. fileciteturn49file0 fileciteturn50file0
fileciteturn45file0 fileciteturn43file0

Is the repo on track for the stated Anvil direction? **Partially.** It is
clearly becoming more Rust-heavy, and the code contains several serious,
deterministic control surfaces. But it is **not yet converged on one credible
devtools-grade foundation**. Right now it is better described as **multiple
promising Rust subsystems plus overlapping orchestration paths**, rather than
one clean kernel with thin adapters. fileciteturn36file0
fileciteturn43file0 fileciteturn48file0 fileciteturn57file0

A limitation up front: I inspected the root architecture/decision/module
materials and the main Rust execution paths, but I did not exhaustively
enumerate every file under `plans/specs/`, every fixture directory, or every
TypeScript package because the GitHub connector did not expose a full repository
tree in one pass. Where I could not verify something directly, I say so
explicitly.

## Repository map

The inspected centre of gravity is a **Rust control-plane cluster**. The main
operational surfaces I verified are:

| Area                | What it does                                                                                                   | Key files                                                                                                                                                                                                                                                                        |
| ------------------- | -------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `anvil-cli`         | Main user-facing CLI; owns command routing for check, gate, watch, intercept, MCP, start, status, hooks, etc.  | `crates/anvil-cli/src/commands/mod.rs`, `check.rs`, `gate.rs`, `watch.rs`, `intercept.rs`, `mcp.rs`, `start.rs` fileciteturn29file0 fileciteturn30file0 fileciteturn31file0 fileciteturn34file0 fileciteturn47file0 fileciteturn48file0 fileciteturn60file0 |
| `anvil-kernel`      | Parser, graph, policy, watch loop, and filesystem watcher primitives.                                          | `crates/anvil-kernel/src/lib.rs`, `watch.rs`, `watcher/mod.rs`, `watcher/filter.rs`, `watcher/pattern.rs` fileciteturn36file0 fileciteturn38file0 fileciteturn39file0 fileciteturn40file0 fileciteturn41file0                                                     |
| `anvil-intercept`   | Daemon-facing runtime: registry, fence store, IPC listener, status, enforcement, watcher integration scaffold. | `crates/anvil-intercept/src/lib.rs`, `ipc.rs`, `watcher.rs` fileciteturn43file0 fileciteturn45file0 fileciteturn46file0                                                                                                                                                 |
| `anvil-run`         | Wrapped-launch ingress for sessions, preflight, registration, process-group spawning, heartbeats, cleanup.     | `crates/anvil-run/src/lib.rs`, `run.rs`, `preflight.rs` fileciteturn57file0 fileciteturn58file0 fileciteturn59file0                                                                                                                                                     |
| `anvil-policy`      | Standalone policy loading and policy execution abstraction, currently via external OPA execution.              | `crates/anvil-policy/src/lib.rs`, `opa.rs` fileciteturn52file0 fileciteturn53file0                                                                                                                                                                                         |
| Planning/docs layer | Architecture and module contracts, but with notable drift.                                                     | `docs/architecture/overview.md`, ADR-014, decision log, `plans/archive/modules/intercept-daemon.aps.md` fileciteturn18file0 fileciteturn16file0 fileciteturn17file0 fileciteturn55file0                                                                              |

In practice, **authority currently lives in several places at once**.
`anvil-kernel` owns watch/graph primitives, `anvil-intercept` owns daemon IPC
and session/fence concepts, `anvil-run` owns launcher/session ingress, and
`anvil-cli` still owns a large amount of orchestration and several direct
validation paths instead of acting as a thin shell. fileciteturn36file0
fileciteturn43file0 fileciteturn57file0 fileciteturn29file0

That means the repo’s current centre of gravity is **Rust**, but **not a single
Rust kernel**. It is a **Rust archipelago**. The main execution paths are split
between CLI-local logic (`check`, `gate`, `start`, MCP stdio), kernel-local
watch/graph logic, daemon-local IPC and registry logic, and launcher-local
session lifecycle logic. fileciteturn30file0 fileciteturn31file0
fileciteturn34file0 fileciteturn38file0 fileciteturn43file0
fileciteturn58file0

### Architecture alignment review

A core complication is that the “intended architecture” is itself inconsistent
across internal materials. The current review brief says Rust-first
deterministic control. Older repo docs still describe a TypeScript-centred
runtime. I therefore judge alignment against the **stated product thesis** while
calling out where the internal repo docs have not caught up.

| Area                            | Intended direction                                                | Current implementation                                                                                                                                        | Alignment     | Evidence                                                                                                                                                                                                                  |
| ------------------------------- | ----------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Core authority                  | One credible deterministic core                                   | Authority is split across CLI checks/gates, MCP validation, kernel watch, daemon IPC, and separate policy execution surfaces                                  | Weak          | `check.rs`, `gate.rs`, `mcp.rs`, `watch.rs`, `anvil-intercept/lib.rs`, `opa.rs` fileciteturn30file0 fileciteturn31file0 fileciteturn48file0 fileciteturn34file0 fileciteturn43file0 fileciteturn53file0 |
| Rust/TypeScript split           | Rust-first core, TS as adapters                                   | Implementation is now Rust-heavy, but docs still describe TS runtime authority and gate flows still shell out to `pnpm`                                       | Partial       | `docs/architecture/overview.md`, ADR-014, `gate.rs` fileciteturn18file0 fileciteturn16file0 fileciteturn31file0                                                                                                  |
| Daemon as enforcement authority | Daemon should be real local control point                         | Daemon runtime wires PID, fences, IPC, registry, status; watcher integration appears incomplete in runtime transport wiring                                   | Weak          | `anvil-intercept/lib.rs`, `watcher.rs`, INTD module plan fileciteturn43file0 fileciteturn46file0 fileciteturn55file0                                                                                             |
| Watcher boundary                | Kernel watcher feeds deterministic enforcement                    | Kernel watcher is solid, but CLI `watch` still optionally shells out to child `gate`/`check` subprocesses after snapshots                                     | Partial       | `watch.rs`, `anvil-kernel/watch.rs`, `watcher/mod.rs` fileciteturn34file0 fileciteturn38file0 fileciteturn39file0                                                                                                |
| MCP/RMCP                        | Thin wrapper over real core                                       | MCP server is hand-rolled in CLI and validate-write can use daemon or embedded fallback; I did not find a dedicated RMCP crate in the inspected Rust surfaces | Partial       | `mcp.rs`, `validate_write.rs`, `validation.rs` fileciteturn48file0 fileciteturn49file0 fileciteturn50file0                                                                                                       |
| Policy evaluation               | Deterministic local engine, ideally embedded                      | `anvil-policy` still shells out to external `opa`, with env/path discovery and temp files                                                                     | Contradictory | decision log ADR-040, `opa.rs` fileciteturn17file0 fileciteturn53file0                                                                                                                                              |
| Evidence and reporting          | Stable separation of validation, evidence, enforcement, reporting | Check/gate outputs are inconsistent; `check` leaves `provenance_id` unset and `gate` results are shallow pass/score/message tuples                            | Weak          | `check.rs`, `gate.rs` fileciteturn30file0 fileciteturn31file0                                                                                                                                                       |
| Docs and plans                  | Plans should guide contributors safely                            | Plans are rich, but several key docs are stale or over-claim completion                                                                                       | Weak          | `overview.md`, ADR-014, decision log, INTD plan, `start.rs` fileciteturn18file0 fileciteturn16file0 fileciteturn17file0 fileciteturn55file0 fileciteturn60file0                                            |

## Critical flows

### CLI validation flow

The direct validation path for `anvil check` enters `check::run`, validates
flags, resolves files, and then calls
`anvil_checks::antipattern::run_antipattern_check` directly. It emits either
plain or JSON output, but the JSON path still sets `provenance_id: None`, and
the declared `checksRun` payload is only `["architecture"]` despite the command
being an antipattern-based scan. Failure is signalled by an already-reported
error if blocking findings cross the severity threshold. The important concern
is that this is a **CLI-local validation flow**, not a single shared “core
validation service” used by every surface. fileciteturn30file0

That matters because `anvil check` is not the same as `anvil gate`, and neither
is the same as MCP pre-write validation. The repo currently has multiple ways to
answer “is this change allowed?” and they do not obviously converge on one
evidence model or one canonical decision API. fileciteturn30file0
fileciteturn31file0 fileciteturn49file0 fileciteturn50file0

### Watcher flow

`anvil watch` builds scope filters and exclude globs, resolves a watch root
inside the workspace, constructs a kernel `WatchConfig`, and calls
`anvil_kernel::watch::run_watch`. The kernel side canonicalizes the root,
performs an initial scan with an ignore-based walker, builds an initial graph in
parallel, then enters a watch loop that processes change batches, re-parses
changed files, updates the graph, and emits progress/snapshot/violation/error
events. The CLI layer can optionally attach an `ActionDispatcher` that reruns
`anvil gate` or `anvil check` as child processes after snapshot events.
fileciteturn34file0 fileciteturn38file0 fileciteturn39file0

This is technically thoughtful. It handles ignored folders, pattern filtering,
watch-limit diagnostics, macOS path canonicalization quirks, Windows slash
normalization, and action-child output isolation. But it is still mostly a
**detect-and-react** flow unless the user is on an MCP pre-write path. The
`start` command is honest about this: watch fallback validates after save and is
explicitly not equivalent to live pre-write validation. fileciteturn39file0
fileciteturn41file0 fileciteturn34file0 fileciteturn60file0

### Daemon flow

`anvil intercept start --foreground` enters the CLI intercept command, builds a
current-thread Tokio runtime, installs a shutdown task, and calls
`run_foreground`. The daemon path then acquires a PID guard, loads fence state,
creates a session registry, constructs a status provider, binds the IPC
listener, and services registry/status calls until shutdown. It also performs
stale-session eviction on a timer. fileciteturn47file0 fileciteturn43file0

What I did **not** see in this runtime path is a live watcher-to-enforcement
bridge. The watcher integration module exists and has sensible concepts
(`WatcherIntegration`, `AttributedBatch`, coalescing, unregistered handler), but
its async `run` loop uses a `recv_blocking` helper that intentionally never
receives production data, with a comment saying the daemon binding code is
responsible for bridging. That makes the runtime boundary incomplete.
fileciteturn46file0

### MCP flow

`anvil mcp serve --stdio` runs a hand-rolled MCP stdio server in the CLI. It
handles JSON-RPC framing, `initialize`, `tools/list`, and `tools/call`, and it
exposes at least the `anvil_validate_write` tool. That tool parses and
normalizes paths, validates workspace anchoring and symlink safety, enforces
UTF-8 and size constraints, then calls `validate_pre_write`, which prefers a
daemon-backed `scan_buffer` RPC on Unix and otherwise falls back to embedded
validation via the intercept enforcement pipeline. The tool returns a structured
payload with a control decision and diagnostics. fileciteturn48file0
fileciteturn49file0 fileciteturn50file0

This is one of the few places where the product thesis is genuinely visible in
code: it is a **pre-write control point** with fail-closed behavior on path
safety, daemon transport errors, JSON-RPC mismatches, and backend failures. The
main concerns are that it is implemented inside `anvil-cli` rather than a
narrower adapter crate, that it is auth-gated even for a local validation
surface, and that the daemon-backed client is Unix-only in the inspected code,
so Windows falls back away from daemon-backed validation. fileciteturn48file0
fileciteturn49file0 fileciteturn50file0

### Policy and gate flow

`anvil gate` is a broad composite runner. It contains local implementations for
secret scanning, antipattern scanning, architecture validation, coverage
parsing, dependency scanning, and some command execution for `pnpm lint:check`
and `pnpm test`. It also has plan-file parsing logic to scope checks.
Separately, the standalone `anvil-policy` crate shells out to an external `opa`
binary, writes temp policy/input files, and parses OPA JSON output into
violations. fileciteturn31file0 fileciteturn51file0 fileciteturn52file0
fileciteturn53file0

That means “policy/gate” is not one clean pipeline today. It is at least two: a
CLI-local gate runner with mixed local and shell-driven checks, and a separate
external-binary policy engine. Explainability exists at the message level, but
**auditable evidence and canonical decision semantics are still too ad hoc**.
fileciteturn31file0 fileciteturn30file0 fileciteturn53file0

### CI and release flow

The repository clearly has a serious release intent: there are distinct CI,
Rust, release, and security workflows, and the code/plan surface repeatedly
references Windows coverage and release packaging discipline. That is good. But
the more important issue than raw build minutes is that the repo still validates
itself through **different compositions of checks depending on surface**: CLI
check, CLI gate, MCP validate-write, embedded intercept validation, and
external-policy execution are not obviously one canonical gate contract. Until
that converges, CI cannot fully stand in for the product’s runtime control
promise. fileciteturn22file0 fileciteturn23file0 fileciteturn24file0
fileciteturn25file0 fileciteturn30file0 fileciteturn31file0
fileciteturn50file0

## Findings

### [Critical] The daemon is presented as the enforcement centre, but the watcher-to-daemon runtime path is not convincingly wired

**What I found**  
The intercept daemon module plan marks watcher integration complete, but the
actual watcher integration module still includes an async receive helper that
intentionally never receives production batches and states that binding code is
responsible for the transport bridge. In the daemon runtime path I inspected,
`run_foreground` stands up PID handling, fences, IPC, status, and stale-session
eviction, but I did not see a live watcher bridge being started there.
fileciteturn55file0 fileciteturn46file0 fileciteturn43file0

**Why it matters**  
This is the difference between “daemon as architecture” and “daemon as real
local authority.” If filesystem events are not flowing into the daemon’s
enforcement path, then session attribution, unknown-agent fencing, and interrupt
decisions are not actually sitting in the hot path the product thesis requires.
That weakens the core claim that Anvil prevents unsafe changes from entering the
system rather than merely observing them later. fileciteturn46file0
fileciteturn55file0

**Evidence**  
The plan claims INTD-004 is complete and describes end-to-end watcher routing,
but `recv_blocking` in `crates/anvil-intercept/src/watcher.rs` explicitly says
production wiring is deferred to binding code and currently pends forever.
`run_foreground` in `crates/anvil-intercept/src/lib.rs` does not show that
bridge being activated in the runtime I inspected. fileciteturn55file0
fileciteturn46file0 fileciteturn43file0

**Recommended fix**  
Make this binary and explicit. Either wire the watcher bridge in the daemon
immediately and add one end-to-end integration test that proves “kernel change
batch → session attribution → enforcement decision,” or mark INTD-004 as not yet
production-wired and stop claiming the daemon is the event authority. Do not
leave it ambiguous. fileciteturn46file0 fileciteturn55file0

**Suggested owner area**  
Intercept daemon runtime.

**Confidence**  
High.

### [High] Validation authority is fragmented across several codepaths instead of one canonical control API

**What I found**  
`anvil check` directly calls antipattern scanning. `anvil gate` is a local
composite runner that mixes Rust checks with `pnpm` subprocesses. `anvil watch`
optionally triggers child `gate` or `check` runs after snapshots. MCP
validate-write uses either daemon `scan_buffer` or embedded intercept
validation. `anvil-policy` separately shells out to OPA. These are materially
different execution paths for related policy decisions. fileciteturn30file0
fileciteturn31file0 fileciteturn34file0 fileciteturn49file0
fileciteturn50file0 fileciteturn53file0

**Why it matters**  
When a product promises deterministic control, the user needs to believe that
“the same proposed change” gets the same decision regardless of whether it came
from CLI, editor, watcher, CI, or daemon. Right now, the repo has too many
semantically adjacent validators for that claim to be comfortable. This also
makes evidence and audit trails harder to stabilize. fileciteturn30file0
fileciteturn31file0 fileciteturn49file0

**Evidence**  
The command registry itself exposes broad overlapping surfaces, and the
implementation files show distinct local logic rather than a single shared call
boundary. `check.rs` and `gate.rs` are especially revealing here.
fileciteturn29file0 fileciteturn30file0 fileciteturn31file0

**Recommended fix**  
Pick one canonical Rust API for “proposed change / changed files → diagnostics →
control decision → evidence.” Then make CLI `check`, CLI `gate`, MCP
validate-write, daemon `scan_buffer`, and CI/reporting compose that API instead
of each owning its own semantics.

**Suggested owner area**  
Core validation architecture across CLI, intercept, and kernel.

**Confidence**  
High.

### [High] The policy engine still depends on an external OPA binary, which contradicts the deterministic local-first direction and internal decisions

**What I found**  
The decision log says ADR-040 accepted Regorus over external OPA binaries, but
the current `anvil-policy` implementation still discovers `opa` on the host or
via `ANVIL_OPA_PATH`, writes temp policy/input files, spawns `opa eval`, and
parses its JSON output. fileciteturn17file0 fileciteturn53file0

**Why it matters**  
External binary execution adds install friction, version drift, platform
variance, and a second trust boundary. It also weakens the “Rust-first core”
story and makes local operation less predictable. For a governance product whose
thesis is determinism, this is not a small detail. fileciteturn53file0

**Evidence**  
`opa.rs` is explicit about binary path resolution, `opa version`, temp
directories, subprocess execution, timeouts, and output-shape parsing. The
contradiction with ADR-040 is direct. fileciteturn53file0
fileciteturn17file0

**Recommended fix**  
Either implement the embedded engine path the decision log already points to, or
formally reverse the decision and treat external OPA as an intentional product
requirement. Do not keep the repo in a half-migrated state.

**Suggested owner area**  
Policy engine.

**Confidence**  
High.

### [High] Docs and plans are materially stale and now misdescribe both the language split and module completion state

**What I found**  
The architecture overview still describes TypeScript packages as the main
runtime and a lighter Rust role. ADR-014 still says “TS by default.” The
intercept daemon module claims full completion. The actual code shows a much
larger Rust footprint, continued `pnpm` dependence in gate flows, external OPA
execution, and an incomplete watcher bridge. fileciteturn18file0
fileciteturn16file0 fileciteturn55file0 fileciteturn31file0
fileciteturn53file0 fileciteturn46file0

**Why it matters**  
In this repo, plans are not decoration. They are part of the operating system
for contributors and agents. If they drift, they do active harm: agents will
optimise toward the wrong centre of gravity and contributors will trust
completion claims that the running code does not support. fileciteturn18file0
fileciteturn55file0

**Evidence**  
The contradiction between the docs and code is not subtle. It is structural.
fileciteturn18file0 fileciteturn16file0 fileciteturn43file0
fileciteturn53file0

**Recommended fix**  
Do a documentation reconciliation pass now, not later. Supersede ADR-014 if it
is no longer policy. Update or retire the architecture overview. Reopen INTD-004
if the runtime bridge is not actually in place. Make the plans safe to follow
again.

**Suggested owner area**  
Architecture / planning / release management.

**Confidence**  
High.

### [High] The product surface is still configuration-fragmented

**What I found**  
The activation/start flow talks about `.anvilrc`. The architecture gate path
reads `.anvil/architecture.yaml`. The intercept module plan describes
`.anvil.yaml` as the daemon enforcement configuration contract. These are
different names, different locations, and different mental models.
fileciteturn60file0 fileciteturn31file0 fileciteturn55file0

**Why it matters**  
This is exactly the kind of accidental complexity that hurts both beta users and
agents. A governance product cannot afford uncertainty about where the real
config lives. If activation, architecture, and daemon enforcement read different
files, users will assume the product is lying when one surface blocks and
another does not. fileciteturn60file0 fileciteturn31file0
fileciteturn55file0

**Evidence**  
The filenames and locations are embedded in the surfaced code and plans, not
inferred. fileciteturn60file0 fileciteturn31file0 fileciteturn55file0

**Recommended fix**  
Choose one canonical operator-facing config contract, then make other files
either generated, namespaced sub-configs, or deprecated aliases with explicit
diagnostics.

**Suggested owner area**  
Product configuration and activation.

**Confidence**  
High.

### [Medium] The watcher stack is technically solid, but still JS/TS-centric in important places

**What I found**  
The kernel watcher and filter code are well hardened, but the default
parseable-extension gate still allows only `ts`, `tsx`, `js`, `jsx`, `mjs`, and
`cjs`. `anvil watch` explicitly keeps that gate for `--all` because the parser
“still only handles TS/JS today.” fileciteturn40file0 fileciteturn34file0

**Why it matters**  
This is a mismatch with the product aspiration to be a serious governance layer
for AI-assisted software development more broadly, and even with the repo’s own
Rust-heavy centre of gravity. You are building a Rust-first control plane, but
one of the main runtime observation paths still defaults to a JS/TS worldview.
fileciteturn40file0 fileciteturn34file0

**Evidence**  
The comments in `watch.rs` are admirably explicit about the trade-off; the
limitation is not hidden. fileciteturn34file0 fileciteturn40file0

**Recommended fix**  
Split language-agnostic policy checks from parser-dependent graph checks, so the
watcher can still provide deterministic coverage across more file types even
before deep parser support expands.

**Suggested owner area**  
Kernel watch/parser roadmap.

**Confidence**  
High.

### [Medium] Evidence and auditability are not yet first-class enough for the product promise

**What I found**  
`anvil check` leaves `provenance_id` unset, and its `checksRun` metadata is not
a clean reflection of the real work. `anvil gate` emits a shallow
`overall / score / checks / notifications / duration` result but not a stable
evidence envelope with bypass records, provenance, or durable decision metadata.
fileciteturn30file0 fileciteturn31file0

**Why it matters**  
The product thesis distinguishes validation, evidence, enforcement, and
reporting. That separation is not yet fully present in the emitted data model.
For enterprise teams later, this is not optional; it is part of the product.
fileciteturn30file0 fileciteturn31file0

**Evidence**  
The TODO in `check.rs` and the minimal `CheckResult` in `gate.rs` are enough to
show the current maturity level. fileciteturn30file0 fileciteturn31file0

**Recommended fix**  
Define one evidence schema that every control surface emits: input identity,
policy set identity, backend used, diagnostics, decision, bypass/override
metadata, timestamps, and a stable provenance ID.

**Suggested owner area**  
Kernel types / reporting / CLI surfaces.

**Confidence**  
High.

### [Positive] The MCP pre-write path is the clearest example of real control rather than after-the-fact detection

**What I found**  
The validate-write tool is strict about workspace trust, path normalization,
symlink escape rejection, deleted cwd handling, binary/oversized content
rejection, and fail-closed daemon response validation. It returns structured
control output rather than just human text. fileciteturn49file0
fileciteturn50file0

**Why it matters**  
This is the closest thing in the inspected repo to the Anvil thesis implemented
faithfully: a proposed write is evaluated before the write should happen, and a
block result is explicit. That is the right shape. fileciteturn49file0

**Evidence**  
The path and content guards are concrete and extensive. The daemon fallback
logic also distinguishes `available`, `not-wired`, and `unavailable`, which is
honest and useful. fileciteturn49file0 fileciteturn50file0

**Recommended fix**  
Promote this path into the canonical validation core rather than leaving it as
one important surface among several competing ones.

**Suggested owner area**  
MCP / validation core.

**Confidence**  
High.

### [Positive] The low-level security posture around IPC, PID/fence state, and dangerous file inputs is stronger than typical for an early devtools repo

**What I found**  
The daemon refuses insecure runtime directories and symlinked socket paths,
verifies same-UID peers on Unix, manages PID-file staleness carefully, and
persists fences before accepting connections. The CLI artifact scan rejects
symlinks, FIFOs, sockets, and oversize inputs. The watch root resolution rejects
paths that escape the workspace. fileciteturn45file0 fileciteturn43file0
fileciteturn30file0 fileciteturn32file0

**Why it matters**  
This is the kind of engineering that makes a future enterprise product
believable. The repo is not sleepwalking into obvious local attack surfaces.
fileciteturn45file0 fileciteturn30file0

**Evidence**  
The code comments and tests show these paths were designed, not accidentally
inherited. fileciteturn45file0 fileciteturn49file0 fileciteturn39file0

**Recommended fix**  
Preserve this standard and move the rest of the architecture up to it.

**Suggested owner area**  
Intercept IPC, kernel watcher, CLI validation.

**Confidence**  
High.

## Cross-platform, agentic, testing, and CI review

### Cross-platform and local-surface review

On **Linux**, the coverage looks strongest. The daemon IPC path uses same-UID
credential checks, the watcher emits diagnostics for partial registration and
inotify-style watch exhaustion, and the main tests I inspected are especially
rich on Unix/Linux paths. fileciteturn45file0 fileciteturn39file0
fileciteturn50file0

On **macOS**, the code shows awareness of real platform quirks. The watch path
documents and handles canonical-root mismatches like `/tmp` versus
`/private/tmp`, and the Unix peer-credential validation has a macOS-specific
`getpeereid` path. That is a good sign. fileciteturn38file0
fileciteturn45file0

On **Windows**, the daemon/status side is more mature than the MCP validation
side. The intercept daemon code and plan clearly include named-pipe support and
owner-only security handling, but the local MCP daemon-validation client in
`validation.rs` explicitly says daemon validation requires a Unix domain socket
and returns `Unavailable` on non-Unix builds. In other words, the write-control
story is not yet symmetrical across platforms. fileciteturn43file0
fileciteturn45file0 fileciteturn50file0

For **WSL**, I did not find convincing evidence of an intentional boundary
model. Given the split between Unix sockets and Windows named pipes, and the
lack of an explicit bridge model in the inspected code, I would treat WSL as
**unclear / likely weak** until proven otherwise.

For **multiple terminals, editor terminal vs external terminal, and multiple
agents in one worktree**, the current architecture does not look ready. The
launcher and daemon materials describe a single-session-per-worktree model for
v1, which is simple and deterministic but not a good fit for “terminal +
editor + sidecar agent” concurrency in the same project tree.
fileciteturn55file0 fileciteturn57file0 fileciteturn58file0

For **multiple worktrees**, the picture is better. The design is
worktree-oriented, and the watcher ignore list explicitly excludes `.worktrees`
and common agent scratch folders such as `.claude`, `.gemini`, `.opencode`, and
`.serena`, which suggests the team is thinking about local-agent clutter and
generated surfaces. Separate worktrees should be more plausible than multiple
sessions in one worktree. fileciteturn40file0

On **generated folders and caches**, the ignore posture is good: `.anvil`,
`.claude`, `.gemini`, `.opencode`, `.serena`, `.worktrees`, `node_modules`,
`.git`, `target`, `dist`, `build`, `.next`, `.turbo`, `.nx`, and `coverage` are
excluded in the watcher filter. fileciteturn40file0

On **symlinks and path canonicalization**, the repo is better than average. The
watcher resolves roots through canonicalization and rejects workspace escapes.
The MCP pre-write path rejects `..`, absolute mismatches, and symlink-anchor
escapes. The IPC layer refuses symlinked socket dirs/files.
fileciteturn32file0 fileciteturn38file0 fileciteturn49file0
fileciteturn45file0

On **deleted current working directory scenarios**, I found an explicit and
well-designed guard in the MCP validate-write path, which is excellent. I did
not verify an equally explicit treatment in every other surface.
fileciteturn49file0

### Agentic workflow review

The repo is **safer for agentic execution than a typical devtools repo**, but
only on some paths. The best example is the MCP server instruction block, which
explicitly tells agents to call validate-write before file writes and to treat
block decisions as authoritative. Combined with the validate-write
implementation, that gives you a real pre-write control surface.
fileciteturn48file0 fileciteturn49file0

But the same repo also makes clear that `anvil start --watch` is only a
save-time fallback and that watch-mode protection is not equivalent to pre-write
validation. That honesty is good, but it also means agent safety depends heavily
on **correct surface installation and use**, not just repo policy.
fileciteturn60file0

`anvil-run` also correctly states that environment propagation is advisory only
and should not be treated as proof of identity. That is the right trust model.
But again, because the daemon-side live watcher authority is not convincingly
wired end to end, the repo is not yet in a place where I would say “agents are
decisively constrained by Anvil at every important boundary.”
fileciteturn57file0 fileciteturn46file0

So my practical assessment is: **safe enough for guided MCP-first agent
workflows; not yet safe enough to claim broad system-wide agent control**.

### Test review

The test architecture has real strengths. Many of the most sensitive modules
carry detailed inline unit tests: watch output modes and race handling, watcher
path filtering on Windows and macOS, socket permission ladders, stale socket/PID
handling, JSON-RPC response-shape checks, and parity checks between embedded and
daemon-backed validation. These are useful behavioural tests, not just
implementation-detail assertions. fileciteturn34file0 fileciteturn39file0
fileciteturn40file0 fileciteturn41file0 fileciteturn45file0
fileciteturn50file0

The biggest missing test is a **whole-system test of the actual product
promise**: launcher registration, daemon runtime, filesystem event ingestion,
worktree/session attribution, deterministic enforcement decision, and surfaced
evidence. The absence of that test lines up with the missing runtime watcher
bridge. fileciteturn46file0 fileciteturn58file0

A second gap is **cross-platform parity testing for MCP validation**. The status
path has better platform thinking than the daemon-backed pre-write validation
path. Windows should have an equivalent daemon-backed validation test, or the
product should explicitly state that MCP pre-write uses embedded-only validation
on Windows for now. fileciteturn47file0 fileciteturn50file0

I did not find evidence of obviously flaky test design in the inspected modules.
The tests are heavily platform-gated and pragmatic. My concern is **coverage
holes**, not obvious flakiness.

### CI, build, and release review

The release posture looks more mature than the architecture posture. The
repository has dedicated CI, Rust, release, and security workflows, which is
what I would expect from a repo taking packaging and distribution seriously.
fileciteturn22file0 fileciteturn23file0 fileciteturn24file0
fileciteturn25file0

My main critique here is not build minutes; it is **semantic drift between what
different pipelines validate**. If the runtime product promise is “deterministic
governance before unsafe changes land,” then the most important CI improvement
is not shaving a few minutes. It is making sure CI, MCP, daemon, watch, and CLI
gate all sit on the same validation and evidence core. Right now they do not.
fileciteturn30file0 fileciteturn31file0 fileciteturn49file0
fileciteturn50file0

I also cannot give a precise wasted-minutes estimate because I did not
line-audit every workflow job and cache key. So I do not want to overstate that
part. The more urgent problem is **credibility of gates**, not CI throughput.

## Remediation, questions, and final recommendation

### Prioritised remediation plan

#### Immediate

| Priority | Action                                                                                                                                                       | Effort                                       | Risk reduced                                                    | Likely files/modules                                                                                                                                                                               |
| -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------- | --------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| P0       | Wire the daemon to a real watcher transport bridge, or explicitly mark watcher integration as not yet runtime-wired and stop claiming daemon event authority | 1–2 days                                     | Removes the largest truth gap in the product story              | `crates/anvil-intercept/src/lib.rs`, `crates/anvil-intercept/src/watcher.rs`, `plans/archive/modules/intercept-daemon.aps.md` fileciteturn43file0 fileciteturn46file0 fileciteturn55file0 |
| P0       | Define one canonical Rust validation API and identify which existing surfaces must call it                                                                   | 1–2 days for design, longer for full rollout | Reduces architecture fragmentation immediately                  | `check.rs`, `gate.rs`, `mcp/validation.rs`, `validate_write.rs`, intercept enforcement surface fileciteturn30file0 fileciteturn31file0 fileciteturn49file0 fileciteturn50file0         |
| P1       | Unify the config contract and stop mixing `.anvilrc`, `.anvil.yaml`, and `.anvil/architecture.yaml` without explicit mapping                                 | 1–2 days                                     | Removes a major source of operator/agent confusion              | `start.rs`, `gate.rs`, intercept config/docs/plans fileciteturn60file0 fileciteturn31file0 fileciteturn55file0                                                                            |
| P1       | Reconcile docs now: supersede stale architecture docs and reopen any over-claimed module status                                                              | 1–2 days                                     | Prevents contributors and agents from following the wrong model | `docs/architecture/overview.md`, ADR-014, decision log, INTD plan fileciteturn18file0 fileciteturn16file0 fileciteturn17file0 fileciteturn55file0                                      |

#### Short term

| Priority | Action                                                                                                                                 | Effort    | Risk reduced                                                      | Likely files/modules                                                                                                            |
| -------- | -------------------------------------------------------------------------------------------------------------------------------------- | --------- | ----------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| P1       | Replace external OPA execution with the embedded policy path the repo already says it wants, or formally reverse that decision         | 1–2 weeks | Improves determinism, installability, and local-first credibility | `crates/anvil-policy/src/opa.rs`, policy crate design/docs fileciteturn53file0 fileciteturn17file0                        |
| P1       | Add one end-to-end “actual control loop” test: launcher/session → daemon → watcher batch → attribution → decision/evidence             | ~1 week   | Proves the product thesis in running code                         | `anvil-run`, `anvil-intercept`, kernel watcher integration fileciteturn58file0 fileciteturn46file0 fileciteturn38file0 |
| P1       | Create a stable evidence schema used by check/gate/MCP/daemon                                                                          | ~1 week   | Improves auditability and enterprise readiness                    | `check.rs`, `gate.rs`, kernel types, MCP response models fileciteturn30file0 fileciteturn31file0 fileciteturn49file0   |
| P2       | Make Windows MCP validation parity explicit: either implement daemon-backed pipe validation or document embedded-only fallback cleanly | ~1 week   | Removes a cross-platform trust gap                                | `mcp/validation.rs`, intercept IPC/client surfaces fileciteturn45file0 fileciteturn50file0                                |

#### Medium term

| Priority | Action                                                                                            | Effort     | Risk reduced                                                      | Likely files/modules                                                                                                                           |
| -------- | ------------------------------------------------------------------------------------------------- | ---------- | ----------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| P1       | Refactor `anvil-cli` toward a thinner adapter layer over shared Rust service APIs                 | 1–2 months | Reduces accidental complexity and surface divergence              | CLI commands, intercept, kernel, policy crates fileciteturn29file0 fileciteturn43file0 fileciteturn36file0                            |
| P1       | Expand language-agnostic enforcement beyond the current JS/TS watch/parser assumptions            | 1–2 months | Aligns the watch/enforcement story with the broader product scope | kernel watcher/filter/parser and intercept enforcement split fileciteturn40file0 fileciteturn34file0                                     |
| P2       | Rework the session model for multi-surface local usage: terminal + editor + multiple agents + WSL | 1–2 months | Improves local-product reality and future enterprise rollout      | `anvil-run`, registry/daemon, launcher policies fileciteturn55file0 fileciteturn57file0 fileciteturn58file0                           |
| P2       | Separate reporting/UX surfaces from enforcement authority more explicitly                         | 1–2 months | Clarifies product architecture and makes control claims auditable | CLI output, daemon status, TUI, MCP payload models fileciteturn30file0 fileciteturn31file0 fileciteturn47file0 fileciteturn48file0 |

### Questions for maintainers

These are the questions I could not answer confidently from the inspected code
and plans:

1. Is the single-session-per-worktree model still intentional for the near term,
   even though it clashes with editor + terminal + multi-agent usage in one
   repo? fileciteturn55file0
2. Is `scan_buffer` intended to become the canonical validation API for all
   surfaces, or is MCP pre-write still meant to be its own path?
   fileciteturn47file0 fileciteturn50file0
3. Is external OPA execution still strategic, or should ADR-040 now be treated
   as the source of truth and implemented? fileciteturn17file0
   fileciteturn53file0
4. Which config file is the user-facing source of truth: `.anvilrc`,
   `.anvil.yaml`, or `.anvil/architecture.yaml`? fileciteturn60file0
   fileciteturn31file0 fileciteturn55file0
5. Was the daemon watcher bridge intentionally left out of `run_foreground`, or
   is that simply unfinished despite the module plan status?
   fileciteturn43file0 fileciteturn46file0 fileciteturn55file0

### Final recommendation

**Pause and fix foundations.**

That is not a condemnation of the repo. There is a lot here worth keeping: the
Rust substrate is real, the pre-write MCP path is genuinely promising, the IPC
and path-safety work is strong, and the team is clearly thinking seriously about
determinism and local control. fileciteturn49file0 fileciteturn50file0
fileciteturn45file0

But I would not widen the trust claims of the product yet. The repo still has
too many overlapping sources of policy truth, the daemon/watcher authority is
not cleanly proven in runtime, policy execution is still externally coupled to
OPA, and the docs/plans are stale enough to mislead contributors and agents.
Until those are fixed, the codebase is **credible as a promising foundation**,
not yet **credible as the singular deterministic governance layer it wants to
be**. fileciteturn46file0 fileciteturn53file0 fileciteturn18file0
fileciteturn16file0 fileciteturn55file0

My blunt version is this: **the repo is close enough that you should not re-plan
the product, but far enough from a unified enforcement core that you should stop
adding new surfaces until the core is made singular, wired, and truthful.**
