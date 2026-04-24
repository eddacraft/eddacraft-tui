# Council Review — ADR-030 Driver-Framework Pivot

|                  |                                                                                                                                                                                                                                      |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Date**         | 2026-04-24                                                                                                                                                                                                                           |
| **Target**       | `origin/dev` through `57be8fc1` + PR #1062                                                                                                                                                                                           |
| **Scope**        | ADR-030, DRVR module, editor-and-mcp-driver-design spec, TSRET updates, KERN Phase 5 supersession (PR #1062), napi pattern-registry code (`crates/anvil-checks-napi/src/lib.rs`)                                                     |
| **Mode**         | `/council-full` — 5 reviewers, 3 debates resolved by judge                                                                                                                                                                           |
| **Reviewers**    | council-reviewer, security-analyst, adversarial-reviewer, operations-reviewer, pragmatic-lead                                                                                                                                        |
| **Verdict**      | **BLOCK** — 2 critical + 16 major + 7 minor + 5 consider = 30 findings                                                                                                                                                               |
| **Tracking PR**  | (this PR)                                                                                                                                                                                                                            |
| **Blocks**       | Release v0.4.0-beta ships `origin/dev` as-is; DRVR-002 protocol spec sign-off; DRVR-001 implementation start (depends on several must-fix items)                                                                                     |

---

## Summary

ADR-030 driver-framework pivot is architecturally sound and plan-state is
consistent, but the design spec and napi code ship with two critical gaps
(process-lifetime poison in the napi registry cache, and an MCP translation
table unrealisable within current INTD scope) plus a dense band of security
and operational majors that must be resolved before DRVR-002 sign-off.

---

## Debates resolved

| Topic                                                                        | Verdict   | Outcome                                                                                                                                                                           |
| ---------------------------------------------------------------------------- | --------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| LazyLock severity in napi cache (adversarial MAJOR vs operations CRITICAL)   | split     | Critical on user-visible failure mode; major on code-comment accuracy. Both tracked below.                                                                                        |
| MCP translation table severity (council-reviewer MAJOR vs adversarial CRIT)  | upgraded  | **Critical** — documentation gap is a subset of a structural "pivot doesn't close" issue.                                                                                         |
| Pivot direction dissent (pragmatic-lead Option A vs Option B vs settled)     | split     | Not a reviewer contradiction; captured as a `consider` item requiring explicit team sequencing decision. The current unowned state is the worst outcome.                          |

---

## Must-fix (2 critical)

### C1. napi registry cache process-lifetime poison

- [ ] **Decide:** remove static `LazyLock` cache from the napi layer *or* detect
      poison explicitly and return a structured error
- [ ] **Implement** the chosen remediation
- [ ] **Test:** add a test that forces a panic during `LazyLock` init (or a
      poisoned `Mutex` in `registry_loader.rs::cache`) and asserts subsequent
      calls fail loudly rather than silently returning empty or lock-poison
      errors

**Location:** `crates/anvil-checks-napi/src/lib.rs:109` + `crates/anvil-checks/src/antipattern/registry_loader.rs` (cache `Mutex`)

**Impact:** VSCode-host process hits a bad registry load once → diagnostics
stop silently until editor restart. Not caught by `cargo test` (tests are
per-process). Currently live on `origin/dev` from PR #1060 pattern-registry
getter work.

**From:** operations-reviewer, adversarial-reviewer

---

### C2. MCP translation table unrealisable within INTD scope

- [ ] **Pick path:**
  - [ ] (a) Shrink §4.3 table to RPCs INTD actually exposes; downgrade the
        rest to MCP-server-local helpers
  - [ ] (b) Explicitly scope `npm audit` / OPA / coverage as driver-side
        composition that does not round-trip through the daemon
  - [ ] (c) Expand INTD scope with accepted cost and new work items
- [ ] **Update:** §4.3 table in `plans/specs/anvil-driver-framework/editor-and-mcp-driver-design.md`
- [ ] **Update:** DRVR-004 expected outcome in `plans/modules/surface-drivers.aps.md`
      to reflect the chosen path
- [ ] **Cross-check:** INTD module (`plans/modules/intercept-daemon.aps.md`)
      for any work items the chosen path requires

**Location:** `plans/specs/anvil-driver-framework/editor-and-mcp-driver-design.md:389`

**Impact:** Six listed RPCs (`scan.files`, `fix.apply`, `gate.run`,
`suppression.apply`, `status.query`, `architecture.queryBoundary`) have no
backing in INTD-001..-013. The `GateRunner` the MCP server currently calls
runs `npm audit`, OPA, coverage JSON reads — none in the daemon. Without
resolution, DRVR-004 ships a regression or INTD scope silently doubles.

**From:** adversarial-reviewer, council-reviewer (upgraded in debate)

---

## Must-fix (16 major)

### Protocol / design

- [ ] **M1.** Add JSON-RPC 2.0 conformance tests + end-to-end round-trip
      latency benchmark to INTD-002 (or a new INTD item).
      KERN-051 supersession dropped these. Without conformance,
      Neovim / Zed / Helix LSP clients may reject or silently drop daemon
      responses — "every LSP client gets Anvil for free" thesis fails at
      the transport layer.
      *(Location: `plans/modules/surface-drivers.aps.md:90`; from:
      adversarial-reviewer)*
- [ ] **M2.** Resolve §3.3 vs §3.5 contradiction on enforcement-participating
      behaviour when daemon drops mid-session. Pick one: fence on daemon loss
      (safe default) *or* fail-soft (availability default). Current spec is
      unresolvable.
      *(Location: `editor-and-mcp-driver-design.md:340`; from: adversarial-reviewer)*
- [ ] **M3.** Add `anvil/gate/request` to §3.2 method table with request /
      response shape, or remove the §3.7 reference.
      *(Location: `editor-and-mcp-driver-design.md:340`; from: council-reviewer)*
- [ ] **M4.** Promote multi-window open question (§6 Q3) to a DRVR-002 blocker
      with a named owner and decision deadline. Without this, DRVR-003 ships
      with undefined enforcement behaviour for two VSCode windows on the same
      worktree.
      *(Location: `editor-and-mcp-driver-design.md:124`; from: adversarial-reviewer)*

### Security

- [ ] **M5.** Re-pin telemetry subscription scoping as daemon-enforced
      (not driver-promised). Post KERN-052 supersession, per-session
      filtering moved from daemon fan-out (enforceable) to driver capability
      (opt-in promise). A hostile same-UID driver can subscribe to every
      violation event from every session — including file paths and
      content excerpts flagged by secret detection.
      *(Location: `editor-and-mcp-driver-design.md:84`; from: security-analyst)*
- [ ] **M6.** Specify redaction / allow-list contract for MCP response
      payloads before DRVR-004 code lands. Default-deny on content excerpts
      for remote transports. Without this, any MCP agent backed by a remote
      LLM becomes an invisible egress path for locally-flagged secrets.
      *(Location: `editor-and-mcp-driver-design.md:352`; from: security-analyst)*
- [ ] **M7.** Document the same-UID trust boundary explicitly. `SO_PEERCRED`
      is adequate for filesystem access but not for SIGKILL / fencing
      authority. Either accept the attack surface and write it down, or add
      a capability token / socket ACL layer on top of UID.
      *(Location: `editor-and-mcp-driver-design.md:124`; from: security-analyst)*
- [ ] **M8.** Pin IPC socket discovery and permissioning: socket mode 0600,
      resolve `$XDG_RUNTIME_DIR` parent-dir symlink policy, explicit Windows
      DACL, `PIPE_REJECT_REMOTE_CLIENTS`. Currently a pre-auth
      attacker-in-the-middle surface.
      *(Location: `plans/decisions/015-intercept-loop-enforcement.md:110`
      (AD-4); from: security-analyst)*
- [ ] **M9.** Re-home rate-limit / connection-cap / auth-timeout / frame-size
      invariants into INTD-002 (or new INTD-014) with numeric budgets.
      KERN Phase 5 supersession silently dropped these; INTD-002 is currently
      DoS-able by any same-UID peer.
      *(Location: `plans/archive/modules/rust-kernel.aps.md`; from:
      security-analyst, adversarial-reviewer)*
- [ ] **M10.** Negotiate LSP capability during `initialize` with fallback
      behaviour, or exclude non-capable clients from enforcement-participating
      mode. Without this, Neovim / Zed / Helix silently drop
      `anvil/enforcement/ack` (LSP spec: unknown notifications ignored),
      daemon interprets as refusal, escalates to worktree fence.
      *(Location: `editor-and-mcp-driver-design.md:124`; from: adversarial-reviewer)*
- [ ] **M11.** Key reliability-budget quarantine on a stable identity
      (signed capability token, install-time UUID, or binary hash) rather
      than self-declared `driverName`. Current design lets a quarantined
      driver rename itself and bypass.
      *(Location: `editor-and-mcp-driver-design.md:124`; from: adversarial-reviewer)*

### Operations

- [ ] **M12.** Specify correlationId retention in DRVR-002: window, on-disk
      store (or explicit non-persistence), and Kindling bridge shape.
      §2.7's "daemon log lookup gives the whole chain" is unsupported —
      daemon is a per-user singleton that restarts.
      *(Location: `editor-and-mcp-driver-design.md:124`; from: operations-reviewer)*
- [ ] **M13.** Specify DRVR-001 partial-failure surface: NDJSON partial
      frames, hung daemon, in-flight request cancellation, preservation of
      `retriable: true` on MCP transport drop mid-RPC.
      *(Location: `plans/modules/surface-drivers.aps.md`; from: operations-reviewer)*
- [ ] **M14.** Add engine-version field to diagnostic output for both TS and
      Rust scanners + a canary that fails if the two engines diverge on the
      repo's own ruleset. Without this, rules added post-TSRET-002 and
      pre-DRVR-003 are invisible to TS-scanner surfaces with no attribution
      path for bug reports.
      *(Location: `plans/modules/anvil-ts-scanner-retirement.aps.md`; from:
      operations-reviewer)*
- [ ] **M15.** Add explicit `timeout-minutes` to each `napi.yml` matrix job
      (default is 6 hours; a hung aarch64 cross-compile can eat a runner
      slot).
      *(Location: `.github/workflows/napi.yml`; from: operations-reviewer)*

### Code

- [ ] **M16.** Fix the `LazyLock` / `Mutex` code comment in
      `crates/anvil-checks-napi/src/lib.rs` to match actual poison behaviour,
      or relocate the commentary to `registry_loader.rs` where the real
      `Mutex` poison hazard lives.
      *(Pairs with C1 above; from: adversarial-reviewer)*

---

## Should-fix (7 minor)

- [ ] **S1.** Merge PR #1062 before v0.4.0-beta bump so the INTD pin fixes
      are on `dev` at release time. *(Plan-doc drift; not a binary blocker.)*
- [ ] **S2.** Remove or annotate TSRET Risks section entries that ADR-030
      vacated (napi-overhead on VSCode hot path; Windows arm64 niche
      targets). *(`plans/modules/anvil-ts-scanner-retirement.aps.md:222`)*
- [ ] **S3.** Re-attribute or mark TSRET M2 milestone as superseded (was
      attributed to now-superseded TSRET-003/-004).
      *(`plans/modules/anvil-ts-scanner-retirement.aps.md:237`)*
- [ ] **S4.** Mark `@eddacraft/anvil-checks-native` as internal (not
      published output) in TSRET Exposes section per ADR-030.
      *(`plans/modules/anvil-ts-scanner-retirement.aps.md:109`)*
- [ ] **S5.** Add INTD-007 (fence persistence) to the KERN-052 supersession
      note — original KERN-052 covered state persistence and the trace is
      currently broken.
      *(`plans/archive/modules/rust-kernel.aps.md`)*
- [ ] **S6.** Assign owners + deadlines to the five DRVR-002 open questions;
      mark which block DRVR-001 sign-off vs DRVR-002 vs DRVR-003.
      *(`editor-and-mcp-driver-design.md:§6`)*
- [ ] **S7.** Add an end-to-end latency harness before committing to the
      50ms daemon-side / 100ms total numbers in DRVR-002 exit criteria
      (current numbers are in-process embedded KERN benchmarks).
      *(`editor-and-mcp-driver-design.md:§3.4`)*

---

## Consider (5)

- [ ] **X1.** Update or remove the crate-level doc comment in
      `crates/anvil-checks-napi/src/lib.rs:10` referencing superseded
      TSRET-003/-004.
- [ ] **X2.** Update the stale "TSRET-003 prep" header in
      `crates/anvil-checks-napi/__tests__/pattern-registry.test.mjs:1`.
- [ ] **X3.** Sanitise `panic_message` before surfacing to JS errors (raw
      payloads may include absolute paths, partial file content, internal
      invariant strings). Low-priority; matters if napi errors ever reach
      non-local logs. *(`crates/anvil-checks-napi/src/lib.rs:109`)*
- [ ] **X4.** Tighten `napi.yml` path filter on `crates/anvil-checks/**`
      which currently fires the 8-job matrix on every scanner rule PR.
      Pre-existing issue; cheap to tighten now that ADR-030 recasts napi as
      an internal artifact.
- [ ] **X5.** **Sequencing decision owed.** Pragmatic-lead recommends the
      team pick explicitly between **Option A** (commit an owner to
      INTD-001 + INTD-002 this sprint; accept TSRET-005 is 2+ months away;
      document the parity-harness cost line with a known exit condition)
      and **Option B** (un-supersede TSRET-003/-004; annotate ADR-030 that
      napi is a stepping stone; land TSRET; delete the TS scanner; then
      build DRVR from a cleaner baseline). Current state — ADR accepted
      with no INTD owner named — is "the worst of both."

---

## How this tracks

This doc is the single source of truth for the review; keep it checked-box
in sync with landed fixes. Each must-fix / should-fix item can be addressed
in its own PR (small, scoped, easy to review) or bundled by area (e.g. all
security items together). When an item lands, check the box and note the
PR / commit. When all critical + major items close, retarget this tracking
PR or close it and archive the doc under `plans/reviews/archive/`.

Reviewer raw outputs (unabridged) are available in the session transcript;
this summary is deliberately condensed for operational use.
