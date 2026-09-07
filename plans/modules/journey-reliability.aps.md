# Journey Reliability — Correct Setup, Recovery and Protection Evidence

| ID | Owner | Priority | Status | Progress |
| -- | ----- | -------- | ------ | -------- |
| JREL | Josh | high | In Progress | 1/13 |

**Packages:** eddacraft-anvil, eddacraft-anvil-intercept, eddacraft-anvil-tui, @eddacraft/anvil-e2e

## Purpose

Own the bounded reliability repair work identified by the 2026-09-05 new/returning-user source review. [JOURNEY](./release-user-journeys.aps.md) coordinates acceptance and release sequencing; [JSIMP](./journey-simplification.aps.md) owns subsequent experience simplification. Planning authorised by the operator on 2026-09-05; no implementation or release is claimed by this intake.

## Evidence and Scope

The review inspected source/tests at `b7b200aecf4a3218b03c144b650269db6b7370fb`. Planning was reconciled against `96bcaf39fd436ea5d68415dedc62c90976db1236`; intervening changes were bookkeeping, not runtime fixes. Findings are source-derived hypotheses with traced control flow, not fresh runtime reproductions. Every item must establish a regression scenario against its execution baseline before changing behaviour; already-fixed cases close with evidence, not duplicate code.

Work-item order maps to review findings F1, F2, F3, F4, F5, F7, F8, F9, F10, F12, F11 and the release-test gap. Current source anchors are each item's Files; [the reviewed source](https://github.com/eddacraft/anvil-001/tree/b7b200aecf4a3218b03c144b650269db6b7370fb) preserves the baseline.

## Boundaries

- Reliability comes before simplification and any public release claim of a reliable journey.
- Use local/CI builds from a pinned main commit and the existing public release process. An internal release channel, new distribution service or version reservation is not required.
- Preserve signed-out discovery, beta entitlement boundaries, optional MCP, explicit consent, global pins, admission/fences and isolated ANVIL_HOME roots.
- Reuse existing primitives. This module owns residual defects, not a replacement for MCPLH, ACTMO, DSV, GCTX or ONSW.
- MCPLH-007 alone owns a future supervisor/proxy. JREL-001 may contain unsafe live replacement without authorising that Draft architecture. Any required architecture/public-contract amendment follows the existing ADR process.
- CIB-405 and issue #4231 remain implementation owners for their existing scopes. JREL-011 owns the remaining whole-lifecycle budget and integration evidence; no duplicated ticket or copied implementation work.
- Ready permits execution only after Dependencies and any explicitly required decision gate are satisfied. File a per-item claim at execution, not during this intake.
- Validation must show the listed scenario actually ran; zero matching tests or skipped required legs cannot count as a pass. Narrow regressions come first; full crate commands are integration closeout, not a demand to repeat broad tests after every edit.

## Interfaces and Acceptance

CLI surfaces consume typed component outcomes and one canonical project/worktree identity. Runtime claims carry observed evidence and freshness; config presence, live PID and a disposable MCP probe are insufficient substitutes. Public commands are the acceptance boundary, including cancellation, failed repair and an upgrade from the previous public build.

**Coordinates with:** [MCPLH](./mcp-live-heal.aps.md), [ACTMO](./activation-mcp-optional.aps.md), [CIB](./continuous-improvement-backlog.aps.md), [JOURNEY](./release-user-journeys.aps.md).

## Attach diagnosis reconciliation (2026-09-06)

The [source review and proposed evidence contract](../specs/2026-09-06-mcp-attach-reconciliation.md)
reconciles the operator's 2026-09-05 MCP attach note against main and open
PR #4416. [Execution checkpoints](../execution/JREL-attach.actions.md) sequence
this slice within the existing JOURNEY gates. No additional module or claim
issue is created; JREL-002 implementation remains owned by PR #4416 / #4408.
Do not overwrite that branch's progress during planning reconciliation.

The following acceptance clarifications belong to the existing items below:

- **JREL-002:** Preserve #4416's registration and per-lease freshness work;
  additionally settle attached/readiness versus observed-validation semantics
  through its owning ADR before completion. Require session/worktree/daemon
  generation correlation, explicit probe exclusion, two instances of one client,
  bounded reconnect after daemon loss and no identity transfer on PID reuse.
  Lease freshness alone must not claim an observed validation call.
- **JREL-004:** Enforce convergence within ADR-036 execution scope, including
  same-scope duplicate detection, while preserving isolated homes and existing
  unsupported-boundary refusal. Registration, scan, GCTX and status use the same
  verified generation; stale identity must never target a replacement process.
- **JREL-005:** Replace MCP status's hardcoded local/not-wired provenance with
  the shared measured projection. Scope aggregate coverage to actual sessions;
  distinguish configuration, startability, attachment, last validation, graph
  readiness and watcher readiness. Preserve policy outcome versus capability,
  optional MCP, JSON compatibility and component-specific recovery actions.
- **JREL-010:** Settle root semantics with JREL-002 before implementation;
  complete canonical admitted-root integration after its session foundation.
  Client-supplied names or roots do not grant authority. Outside-repo and
  multiple-root sessions require explicit admitted bindings or truthful refusal.
- **JREL-011:** Include heartbeat/reconnect bursts at around 100 sessions and
  full-snapshot payload cost. Deliver bounded worktree/session attestation if
  measurements require it, retaining wire compatibility and CIB-405 ownership.
  Validate deadline and capacity behaviour rather than assuming short-lived
  connections cannot exhaust limits.
- **JREL-012:** Add the specification's real-process matrix, including
  probe-only negatives, multi-editor positives, expiry, duplicate endpoints,
  component degradation and upgrade. Missing required scenarios are failures.

JREL-003 retains save-time recovery ownership. JOURNEY-014/-015 consume the full
JREL evidence and installed-release rehearsal; this subset alone cannot close
those gates. CIB-384 is already implemented on main, so distribution inclusion
is a release verification task. The spec is proposed planning context: changes
to public evidence semantics require the existing ADR process, not implicit
acceptance through this intake. Item statuses and counters remain unchanged.

## Work Items

### JREL-001: Lossless MCP upgrade and session continuity

- **Status:** Ready
- **Priority:** P0
- **Intent:** An established MCP session survives a runtime update without losing accepted work.
- **Expected Outcome:** An update between requests or during a call preserves request/reply identity and protocol negotiation; pipelined input is not discarded, mutations are not repeated, and a failed update leaves a usable session. If transparent replacement cannot be proven, existing sessions explicitly require reconnect while pre-session replacement remains safe.
- **Dependencies:** none
- **Coordinates with:** MCPLH-002, MCPLH-008; MCPLH-007 remains the sole owner of any future supervisor/proxy implementation. This item owns regression containment and continuity evidence, not a second supervisor.
- **Files:** `crates/anvil-cli/src/commands/mcp.rs`, `crates/anvil-cli/src/mcp/reexec.rs`, `crates/anvil-cli/src/mcp/protocol/`, `crates/anvil-cli/tests/mcp_reexec.rs`
- **Validation:** `cargo test -p eddacraft-anvil --test mcp_reexec --no-fail-fast`; regressions exercise a successful live generation change after initialisation, sequential and pipelined calls, legacy and modern protocol paths, and failed replacement.
- **Confidence:** medium — source-reviewed; reproduce through public boundaries before fixing.

### JREL-002: Actual client evidence for protection claims

- **Status:** Merged
- **Priority:** P0
- **Intent:** Protection reporting identifies real client activity rather than inferring it from configured client count.
- **Expected Outcome:** A configured but closed editor never gains live pre-write status from durable registration or a disposable handshake probe. One or several clients have independently attributed, fresh worktree/session evidence. Connection, observed validation, graph readiness and enforcement mode remain distinct; stale evidence from one client cannot borrow another's freshness.
- **Dependencies:** none
- **Coordinates with:** CIB-384 is already Merged for durable-age handling; this is its explicitly unowned client-attribution residual. MCPLH-005 inventory is reused, not rewritten as connection proof.
- **Files:** `crates/anvil-cli/src/activation/daemon_evidence.rs`, `crates/anvil-cli/src/mcp/client_session.rs`, `crates/anvil-cli/src/commands/mcp.rs`, `crates/anvil-cli/src/registration.rs`, `crates/anvil-intercept/src/status.rs`, `crates/anvil-run/src/heartbeat.rs`, `plans/decisions/141-mcp-live-session-registration.md`
- **Validation:** `cargo test -p eddacraft-anvil --no-fail-fast`; closed-client, one-client, two-client, expired-client and temporary-probe scenarios verify both positive and negative protection claims.
- **Confidence:** medium — source-reviewed; reproduce through public boundaries before fixing.

### JREL-003: Recover and verify the save-time driver

- **Status:** Committed — PR #4428 (claim #4423). Closeout: a durable
  membership refresh now restores a dead save-time driver child (new
  `MembershipChange::Refreshed`, enqueued under the registry lock so a signal
  cannot be reordered ahead of the mutation it describes), dead children
  are reaped before liveness probes instead of being reported attached,
  respawns are bounded (three consecutive failures, one-minute backoff, early
  death counts as failure), and the child writes a readiness marker the daemon
  reports as the additive optional `save_time_driver_evidence`
  (`spawned` / `watches-installed` / `fresh-activity`). Registration waits a
  bounded second for the driver and names only a failed driver on the human
  `worktree:` line. Baseline reproduction through the real binary
  (`crates/anvil-cli/tests/save_time_driver_recovery.rs`): re-running bare
  `anvil` after killing only the child left the dead PID reported attached;
  after the fix exactly one new driver installs watches and a saved fixture
  yields a real AP-003 verdict. Closeout runs:
  `cargo test -p eddacraft-anvil-intercept --no-fail-fast` 1144 passed and
  `cargo test -p eddacraft-anvil --no-fail-fast` 4276 passed, with the only
  failures being the same pre-existing sandbox-environment tests noted under
  JREL-004. JREL-005 owns typed readiness presentation and exit status and can
  consume the new evidence. Council (general + adversarial) findings addressed
  in the same PR: the respawn bound is now measured from evidence that the
  child was actually alive (liveness probes and the readiness marker's
  modification time) rather than from when the next event happened to be
  processed, so a child that crashes immediately is capped even when refreshes
  are minutes apart; the readiness marker carries the writing child's PID and a
  marker from another generation is not read as the current child's evidence;
  stopping a driver waits a bounded time and escalates to a hard kill before
  releasing the entry, so a stop racing a registration cannot leave two
  children; membership signals are enqueued under the same lock as the mutation
  they describe, so an unregister racing a heartbeat can no longer respawn a
  driver for a worktree that has left durable membership; and the readiness
  wait short-circuits outcomes that cannot become attached, so a disabled or
  failed driver no longer costs every command the full budget. Failed drivers
  now reach the `--all` and `--json` registration surfaces through additive
  keys. Deferred: capping each status fetch against the remaining budget, which
  is documented at the constant instead.
- **Priority:** P1
- **Intent:** The daily command restores a failed worktree watcher while preserving durable registration.
- **Expected Outcome:** After child death or initial spawn failure, public bare/start registration paths restore exactly one ready driver or return a bounded failure. Membership refresh alone is not readiness. Ready evidence distinguishes spawned from watches-installed and fresh activity; persistent failure cannot enter an unbounded respawn loop.
- **Dependencies:** none
- **Coordinates with:** ACTMO-006, ACTMO-014, DSV background driver; preserve durable-membership and admission semantics.
- **Files:** `crates/anvil-intercept/src/save_time_driver.rs`, `crates/anvil-intercept/src/registry.rs`, `crates/anvil-cli/src/registration.rs`, `crates/anvil-cli/src/commands/ensure.rs`
- **Validation:** `cargo test -p eddacraft-anvil-intercept --no-fail-fast`; `cargo test -p eddacraft-anvil --no-fail-fast`; kill only the child, rerun the actual CLI, and save a fixture to obtain a real validation result. The test must not inject a membership event in place of CLI registration.
- **Confidence:** medium — source-reviewed; reproduce through public boundaries before fixing.

### JREL-004: One daemon identity through start and recycle

- **Status:** Committed — PR #4428 (claim #4424). Closeout: ensure probes the
  canonical endpoint then validated sibling endpoints before spawning and
  serialises spawn under the existing CIB-382 rendezvous coordinator; recycle
  signals only the instances it observed, re-probes when nothing was signalled
  and requires the replacement to report the CLI version; status reads the PID
  verified beside the answering socket. Baseline reproduction through the real
  binary (`crates/anvil-cli/tests/daemon_identity.rs`): XDG shell and
  stale-canonical-plus-live-sibling both spawned a duplicate (`daemon: started`)
  before the fix; isolated homes were already distinct. Closeout runs:
  `cargo test -p eddacraft-anvil-intercept --no-fail-fast` 1241 passed and
  `cargo test -p eddacraft-anvil --no-fail-fast` 4277 passed, with the only
  failures being pre-existing sandbox-environment tests (unset `USER`, uid 0
  bypassing `chmod` fixtures) in files this change does not touch. Council
  (general + adversarial) findings addressed in the same PR: forced restart now
  applies the replacement-version gate and reuses a concurrent replacement;
  ensure probes every candidate and reports two live same-scope daemons as a
  failure naming `anvil doctor --fix`; recycle refuses to signal when the PID
  snapshot holds more than one distinct live process. Deferred to JREL-011
  (lifecycle budget): a bounded wait on the rendezvous coordinator and start
  lock, and a typed outcome when the coordinator cannot be acquired (today it
  degrades to the per-install lock with a warning, per warnings-over-blocks).
  Deferred to JREL-005: surfacing the same-scope conflict through `intercept
  status` typed output.
- **Priority:** P1
- **Intent:** Shell and editor environments converge on the same intended daemon instance.
- **Expected Outcome:** Discovery, startup, registration, status and recycle agree on one verified instance across runtime/state-home endpoints. Concurrent cold starts or updates cannot produce duplicate daemons or stop a newly started replacement using stale identity. Expected replacement version and readiness are checked; isolated ANVIL_HOME installations remain distinct.
- **Dependencies:** none
- **Coordinates with:** DLIFE lifecycle, MCPLH-004 recycle, CIB-382 and merged sibling-discovery/repair fixes; CIB-405 separately owns caller migration to connection reuse.
- **Files:** `crates/anvil-intercept/src/ensure.rs`, `crates/anvil-intercept/src/ipc.rs`, `crates/anvil-cli/src/commands/daemon_recycle.rs`, `crates/anvil-cli/src/commands/intercept.rs`
- **Validation:** `cargo test -p eddacraft-anvil-intercept --no-fail-fast`; `cargo test -p eddacraft-anvil --no-fail-fast`; run set/unset XDG environments in both orders, concurrent starts/recycles, stale canonical plus live sibling, and intentional isolated homes.
- **Confidence:** medium — source-reviewed; reproduce through public boundaries before fixing.

### JREL-005: Typed readiness, failures and recovery outcomes

- **Status:** Ready
- **Priority:** P1
- **Intent:** Successful daily activation means the user's selected coverage is ready.
- **Expected Outcome:** Registration refusal/cap, failed selected MCP repair, dead watcher and unresponsive daemon remain typed failures through human/JSON output and exit status. Deliberately omitted MCP is not failure when selected save-time coverage works. Starting, ready, disabled, degraded and failed evidence is distinguishable; one next action names the actual failing component. Preserve current output compatibility or document an explicit versioned migration.
- **Dependencies:** JREL-002, JREL-003, JREL-004
- **Coordinates with:** ACTMO activation diagnostic, ONSW ensure, MCPLH-005 split inventory; JSIMP-002 owns later presentation/action contract migration.
- **Files:** `crates/anvil-cli/src/commands/ensure.rs`, `crates/anvil-cli/src/commands/start.rs`, `crates/anvil-cli/src/commands/status.rs`, `crates/anvil-cli/src/activation/`
- **Validation:** `cargo test -p eddacraft-anvil --no-fail-fast`; assert typed JSON, human next action and exit behaviour for every required/optional component combination, including warming and no-MCP.
- **Confidence:** medium — source-reviewed; reproduce through public boundaries before fixing.

### JREL-006: Preserve learning across projects and interrupted setup

- **Status:** Ready
- **Priority:** P1
- **Intent:** Returning users retain learning progress when they open another repository.
- **Expected Outcome:** A missing project first-run marker never deletes user-global tutorial progress. Visiting repo B after learning in A preserves progress even on immediate quit, failure, plain/JSON invocation or upgrade. Explicit reset remains deliberate; started, deferred and completed setup are not conflated with runtime health.
- **Dependencies:** none
- **Coordinates with:** JOURNEY-012 already Merged; preserve its completed-path suppression. JSIMP-003 later unifies the user flow without duplicating this fix.
- **Files:** `crates/anvil-cli/src/services/first_run.rs`, `crates/anvil-cli/src/commands/welcome.rs`, `crates/anvil-cli/src/commands/tutorial.rs`
- **Validation:** `cargo test -p eddacraft-anvil --no-fail-fast`; cross-repository, cancelled, failed and explicit-reset fixtures prove state ownership and retention.
- **Confidence:** medium — source-reviewed; reproduce through public boundaries before fixing.

### JREL-007: Correct project and cancellation through guided setup

- **Status:** Ready
- **Priority:** P1
- **Intent:** All guided setup operations honour the selected repository and actual outcome.
- **Expected Outcome:** Choosing B while launched from A makes config, scan, preview/apply, tutorial and markers consistently target B; A remains unchanged. Back returns to the prior step, Quit exits, and a write failure cannot be interpreted as Configured or continue silently into the success route. Existing preview/containment safeguards remain intact.
- **Dependencies:** none
- **Coordinates with:** UCFG discovery, existing welcome first-win path; JREL-010 covers MCP admission separately.
- **Files:** `crates/anvil-cli/src/commands/welcome.rs`, `crates/anvil-cli/src/commands/init.rs`, `crates/anvil-cli/src/commands/tutorial.rs`, `crates/anvil-tui/src/surfaces/init/`
- **Validation:** `cargo test -p eddacraft-anvil --no-fail-fast`; fixtures cover A-to-B selection, subdirectory/worktree invocation, cancellation at each stage and config-write failure.
- **Confidence:** medium — source-reviewed; reproduce through public boundaries before fixing.

### JREL-008: Restore the terminal on all welcome exits

- **Status:** Ready
- **Priority:** P1
- **Intent:** An interrupted or failed welcome session leaves the terminal usable.
- **Expected Outcome:** Ordinary Result errors and partial terminal setup restore raw mode and alternate-screen state, as do cancellation and panic paths. The existing guard/session ownership model is reused; a panic-only hook is not sufficient evidence.
- **Dependencies:** none
- **Coordinates with:** ACTTUI terminal session handling; retain existing panic restoration.
- **Files:** `crates/anvil-cli/src/commands/welcome.rs`, `crates/anvil-cli/src/tui.rs`, `crates/anvil-cli/tests/`
- **Validation:** `cargo test -p eddacraft-anvil --no-fail-fast`; PTY fault injection after terminal entry and partial setup checks terminal attributes, screen restoration and subsequent shell usability.
- **Confidence:** medium — source-reviewed; reproduce through public boundaries before fixing.

### JREL-009: Preserve explicit MCP launch choices during repair

- **Status:** Ready
- **Priority:** P1
- **Intent:** Daily repair fixes owned drift without undoing the user's intended client configuration.
- **Expected Outcome:** Supported explicit executable, scope, environment and per-entry options survive daily ensure. Managed obsolete paths may migrate, but intentional overrides, disabled state and pins remain respected. Unsafe/corrupt/conflicting entries are reported accurately and never overwritten or mislabelled absent. The ownership policy and any ADR-044 amendment are explicit before changing its contract.
- **Dependencies:** none
- **Coordinates with:** MCPLH-008 global pin is retained; JSIMP-004 owns the later unified integration-choice experience.
- **Files:** `crates/anvil-cli/src/activation/mcp_client.rs`, `crates/anvil-cli/src/activation/orchestrator/install.rs`, `crates/anvil-cli/src/commands/mcp.rs`, `crates/anvil-cli/src/commands/mcp_heal.rs`
- **Validation:** `cargo test -p eddacraft-anvil --no-fail-fast`; explicit absolute command/env, disabled/custom options, old managed path, global pin and unsafe-drift fixtures survive repeated ensure.
- **Confidence:** medium — source-reviewed; reproduce through public boundaries before fixing.

### JREL-010: Consistent admitted workspace identity in MCP

- **Status:** Ready
- **Priority:** P1
- **Intent:** MCP tools use the intended admitted project regardless of launch directory.
- **Expected Outcome:** Activation and MCP status, graph and validation agree on canonical project/worktree identity for root, package subdirectory, linked worktree and symlink launches. Outside-repo launch either establishes an explicitly admitted root or gives precise reconnection guidance. Unrelated or nested untrusted roots remain refused; no arbitrary caller-root escape is introduced.
- **Dependencies:** JREL-002
- **Coordinates with:** GCTX root admission, ACTMO registration, CIB-414 nested-root boundary; this item fixes identity mismatch without relaxing those controls.
- **Files:** `crates/anvil-cli/src/mcp/tools/shared.rs`, `crates/anvil-cli/src/mcp/tools/status.rs`, `crates/anvil-cli/src/mcp/protocol/domain.rs`, `crates/anvil-cli/src/registration.rs`
- **Validation:** `cargo test -p eddacraft-anvil --no-fail-fast`; launch-location matrix and negative containment tests prove common identity without widened admission.
- **Confidence:** medium — source-reviewed; reproduce through public boundaries before fixing.

### JREL-011: Bound the complete lifecycle and integration exchange

- **Status:** Ready
- **Priority:** P1
- **Intent:** Every startup or repair attempt completes within an explicit overall deadline.
- **Expected Outcome:** Lock acquisition, endpoint discovery, spawn, registration and readiness share a measured monotonic budget; slow partial replies cannot extend it indefinitely. Unresponsive and absent stay distinct without unsafe duplicate spawning. CIB-405 owns connection-reuse migrations and issue #4231 owns its MCP-validation timeout fix; their verified outcomes are integrated here without duplicate implementations.
- **Dependencies:** JREL-004
- **Coordinates with:** CIB-405 (connection reuse), #4231 (MCP validation deadline); claim/reconcile those existing owners when executing rather than creating replacement items.
- **Files:** `crates/anvil-intercept/src/ensure.rs`, `crates/anvil-cli/src/commands/daemon_recycle.rs`, `crates/anvil-cli/src/registration.rs`, `crates/anvil-cli/src/mcp/validation.rs`
- **Validation:** `cargo test -p eddacraft-anvil-intercept --no-fail-fast`; `cargo test -p eddacraft-anvil --no-fail-fast`; delayed lock, unreachable endpoint, slow-drip peer and responsive-invalid peer finish within the documented budget. Record tests/evidence from CIB-405 and #4231.
- **Confidence:** medium — source-reviewed; reproduce through public boundaries before fixing.

### JREL-012: Require executable product-journey verification

- **Status:** Ready
- **Priority:** P1
- **Intent:** A green journey job proves the actual anvil binary and required scenarios ran.
- **Expected Outcome:** Existing CI/test tooling can build or consume a binary from a pinned main commit and fails when a required binary, scenario or expected test is missing/skipped. It exercises normal activation and real daemon/MCP transport, not only dev/no-daemon/no-MCP shortcuts. The reusable gate records source/binary/platform/client identity, supports previous-public-build upgrade testing, and provides a documented command for conductor rehearsals without an internal release service.
- **Dependencies:** none
- **Coordinates with:** JOURNEY-014/-015 own acceptance; existing apps/e2e and Rust PTY/process suites remain in place.
- **Files:** `.github/workflows/ci.yml`, `.github/workflows/rust.yml`, `apps/e2e/`, `crates/anvil-cli/tests/`, `package.json`
- **Validation:** `pnpm --filter @eddacraft/anvil-e2e test:cli`; `pnpm --filter @eddacraft/anvil-e2e test:smoke`; `cargo test -p eddacraft-anvil --no-fail-fast`; remove the binary and a required scenario to prove the gate fails. Completion publishes the exact no-skip rehearsal command consumed by JOURNEY-014/-015.
- **Confidence:** medium — source-reviewed; reproduce through public boundaries before fixing.

### JREL-013: Scope pre-write protection evidence to one worktree

- **Status:** Ready
- **Priority:** P2
- **Intent:** Reading protection evidence on the pre-write path costs one worktree's worth of work, not the whole daemon's.
- **Expected Outcome:** A caller that needs one worktree's attestation can ask for exactly that. The pre-write MCP path no longer materialises every registered session to answer a single-worktree question, so evidence cost stops scaling with unrelated sessions on the host. Existing full-snapshot consumers (`anvil status`, `anvil workspace list`, activation diagnostics) keep their current answers; freshness, attribution and fail-closed behaviour are unchanged by the narrowing.
- **Dependencies:** JREL-002
- **Coordinates with:** ADR-141 names this as follow-up; CIB-405 separately owns caller migration to connection reuse. MLP2-051f/-051h freshness and anchor semantics are reused, not restated.
- **Files:** `crates/anvil-intercept-proto/src/lib.rs`, `crates/anvil-intercept/src/ipc.rs`, `crates/anvil-intercept/src/status.rs`, `crates/anvil-cli/src/mcp/validation.rs`, `crates/anvil-cli/src/commands/intercept.rs`
- **Validation:** `cargo test -p eddacraft-anvil --no-fail-fast`; `cargo test -p eddacraft-anvil-intercept --no-fail-fast`; a scoped query and a full snapshot must agree on the same worktree's claim across attested, unattested, stale and fenced fixtures, and a pre-write request must not read sessions belonging to other worktrees.
- **Confidence:** medium — ADR-141 records the measured motivation (95 concurrent `anvil mcp serve` processes on one host; `QueryStatus` is unscoped and `mcp/validation.rs` fetches the full snapshot on the pre-write path). Confirm the cost against a real snapshot before choosing between a new verb and a filter parameter.
