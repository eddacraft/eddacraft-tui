# Council Review — ADR-030 Driver-Framework Pivot

|                  |                                                                                                                                                                                                                                      |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Date**         | 2026-04-24                                                                                                                                                                                                                            |
| **Target**       | `origin/dev` through `57be8fc1` + PR #1062                                                                                                                                                                                            |
| **Scope**        | ADR-030, DRVR module, editor-and-mcp-driver-design spec, TSRET updates, KERN Phase 5 supersession, napi pattern-registry code (`crates/anvil-checks-napi/src/lib.rs`)                                                                 |
| **Mode**         | `/council-full` — 5 reviewers, 3 debates resolved by judge                                                                                                                                                                            |
| **Reviewers**    | council-reviewer, security-analyst, adversarial-reviewer, operations-reviewer, pragmatic-lead                                                                                                                                         |
| **Verdict**      | **BLOCK** — 2 critical + 16 major + 7 minor + 5 consider = 30 findings                                                                                                                                                                |
| **Tracking PR**  | (this PR)                                                                                                                                                                                                                             |
| **Blocks**       | DRVR-002 protocol sign-off; DRVR-001 implementation start (depends on several must-fix items). Release v0.4.0-beta unblocked once C1 landed in #1064.                                                                                 |

---

## Status at a glance (2026-04-24)

| Status                                     | Count | Items                                                                                                   |
| ------------------------------------------ | ----- | ------------------------------------------------------------------------------------------------------- |
| **Landed** (code + docs merged)            | 10    | C1, M8, M13, M16, S1, S2, S3, S4, S5, X1                                                                |
| **Landing in PR #1068**                    | 4     | M15, X2, X3, X4 — napi hygiene bundle                                                                   |
| **Routed to APS** (have a work-item home)  | 15    | C2, M1, M2, M3, M4, M5, M6, M7, M9, M10, M11, M12, M14, S6, S7                                          |
| **Decided** (recorded in ADR-030)          | 1     | X5 — **Option A: INTD picked up straight after v0.4.0-beta**                                            |

All 30 findings now have a home — this tracker PR is closeable once
#1068 merges. Remaining execution visibility is in the DRVR / INTD /
TSRET work items themselves.

---

## Debates resolved

| Topic                                                                        | Verdict   | Outcome                                                                                                                                                                           |
| ---------------------------------------------------------------------------- | --------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| LazyLock severity in napi cache (adversarial MAJOR vs operations CRITICAL)   | split     | Critical on user-visible failure mode (C1); major on code-comment accuracy (M16). Both landed in #1064.                                                                            |
| MCP translation table severity (council-reviewer MAJOR vs adversarial CRIT)  | upgraded  | **Critical** — documentation gap is a subset of a structural "pivot doesn't close" issue. Routed to DRVR-006 in #1065.                                                             |
| Pivot direction dissent (pragmatic-lead Option A vs Option B vs settled)     | split     | Not a reviewer contradiction; captured as `consider` item X5. Team decision owed.                                                                                                 |

---

## Must-fix (2 critical)

### C1. napi registry cache process-lifetime poison — **Landed**

- [x] **Fixed in PR #1064** (commit `deecb3c0`) — `load_registry_or_err`
      helper inverts the silent-empty default; every napi entry point
      (`scan_artifact_json`, `get_default_patterns_json`,
      `get_pattern_json`) now fails loudly with a remediation hint
      when the compiled registry can't be loaded.
- [x] **Test:** `__tests__/registry-missing.test.mjs` forces the
      failure path via `ANVIL_REGISTRY_PATH` and asserts loud
      failure, not silent empty.

### C2. MCP translation table unrealisable within INTD scope — **Routed to APS**

- [x] **APS item: DRVR-006** ("Pin MCP daemon-RPC surface — resolve
      translation-table scope"), filed in #1065
      (`plans/archive/modules/surface-drivers.aps.md`). Blocks DRVR-002
      sign-off. Three paths recorded (shrink / scope-local / expand
      INTD); choose before DRVR-002 freezes.

---

## Must-fix (16 major)

### Protocol / design

- [x] **M1.** → **APS: INTD-014** (JSON-RPC 2.0 conformance +
      round-trip latency benchmark). Filed in #1065
      (`plans/archive/modules/intercept-daemon.aps.md`).
- [x] **M2.** → **Embedded in DRVR-002 expected outcome** as a
      sign-off prerequisite (#1065). Must pick
      fence-on-daemon-loss vs fail-soft before DRVR-002 freezes.
- [x] **M3.** → **Embedded in DRVR-002 expected outcome** as a
      sign-off prerequisite (#1065). `anvil/gate/request` method
      table fix.
- [x] **M4.** → **Embedded in DRVR-002 expected outcome** as a
      sign-off prerequisite (#1065). Multi-window fan-out semantics.

### Security

- [x] **M5.** → **APS: INTD-015** (daemon-enforced telemetry
      subscription scoping) + **DRVR-007** (driver trust contract,
      which pulls from the filter).
- [x] **M6.** → **APS: DRVR-007** (MCP redaction contract is part
      of the driver trust contract) + also embedded in DRVR-002
      prerequisites.
- [x] **M7.** → **APS: DRVR-007** (same-UID trust boundary
      documented + hardened).
- [x] **M8.** **Landed** — INTD-002 Expected Outcome amended in
      #1065 (`plans/archive/modules/intercept-daemon.aps.md`) with full
      socket/pipe creation sequence: `lstat`/`openat` +
      `O_NOFOLLOW`, `mkdir` with explicit mode, post-creation
      `stat`/`fstat` verify, `fchmod` socket fd before `listen()`,
      Windows DACL + `PIPE_REJECT_REMOTE_CLIENTS`, driver-side
      ownership check. Tightened again in #1065's review fixes
      after copilot corrected the `O_NOFOLLOW`-on-`mkdir` mistake.
- [x] **M9.** → **APS: INTD-016** (DoS protection budgets:
      connection cap, RPS, timeouts, frame size, TLS stance).
      Filed in #1065.
- [x] **M10.** → **APS: DRVR-008** (non-VSCode LSP client
      capability negotiation — drivers advertise
      `supportedAnvilMethods`, daemon caps unadvertised clients at
      read-only). Filed in #1065.
- [x] **M11.** → **APS: DRVR-007** (reliability-budget quarantine
      keyed on stable identity, not `driverName`).

### Operations

- [x] **M12.** → **Embedded in DRVR-002 expected outcome** as a
      sign-off prerequisite (#1065). correlationId retention
      window + Kindling bridge shape.
- [x] **M13.** **Landed** — DRVR-001 Expected Outcome amended in
      #1065 (`plans/archive/modules/surface-drivers.aps.md`) with explicit
      partial-failure surface: NDJSON framer on parse error
      preserves connection, per-request timeout with structured
      retriable error, in-flight cancellation on transport drop
      preserves the retriable flag, driver-side socket-owner
      refusal.
- [x] **M14.** → **APS: TSRET-006** (engine-version diagnostic
      field + transition-window divergence canary). Filed in #1065
      (`plans/archive/modules/anvil-ts-scanner-retirement.aps.md`).
- [ ] **M15.** `napi.yml` per-job `timeout-minutes`. Still open.
      Hygiene fix — suitable for a small chore PR bundled with X2 /
      X3 / X4. *(Location: `.github/workflows/napi.yml`)*

### Code

- [x] **M16.** **Landed** in PR #1064 alongside C1 — the
      misleading `LazyLock` / `Mutex` commentary was replaced when
      the whole `crates/anvil-checks-napi/src/lib.rs` doc block was
      rewritten to describe the real registry-load behaviour.

---

## Should-fix (7 minor)

- [x] **S1.** **Landed** — PR #1062 merged, so the INTD pin fixes
      (INTD-004/-005/-010 → INTD-002/-003/-005/-013) are on `dev`.
- [x] **S2.** **Landed** — TSRET Risks section amended in #1065.
      Napi-overhead and Windows arm64 risks marked vacated; only
      transitive-consumers risk remains live.
      *(`plans/archive/modules/anvil-ts-scanner-retirement.aps.md`)*
- [x] **S3.** **Landed** — TSRET Milestones re-pointed in #1065.
      M2 now attributed to TSRET-006; old TSRET-003/-004 M2 marked
      superseded.
- [x] **S4.** **Landed** — TSRET Exposes section amended in #1065.
      `@eddacraft/anvil-checks-native` marked internal-only,
      private, not published to npm.
- [x] **S5.** **Landed** — KERN-052 supersession note in
      `plans/archive/modules/rust-kernel.aps.md` amended in #1065.
      Now points at INTD-003 (session registry), INTD-007 (fence
      persistence), and INTD-015 (daemon-enforced filter) — full
      trace restored.
- [x] **S6.** → **Embedded in DRVR-002 expected outcome** as a
      sign-off prerequisite (#1065). Owners + deadlines on the
      five §6 open questions.
- [x] **S7.** → **Embedded in DRVR-002 expected outcome** as a
      sign-off prerequisite (#1065). End-to-end latency harness
      before locking the 50ms / 100ms numbers.

---

## Consider (5)

- [x] **X1.** **Landed** in PR #1064 — `crates/anvil-checks-napi/src/lib.rs`
      doc comment was rewritten; stale TSRET-003/-004 reference is
      gone.
- [ ] **X2.** `crates/anvil-checks-napi/__tests__/pattern-registry.test.mjs`
      header still says "TSRET-003 prep". Still open — small
      chore-PR candidate.
- [ ] **X3.** Sanitise `panic_message` before surfacing to JS
      errors. Still open — small chore-PR candidate.
      *(`crates/anvil-checks-napi/src/lib.rs`)*
- [ ] **X4.** Tighten `napi.yml` path filter on
      `crates/anvil-checks/**`. Still open — small chore-PR
      candidate; bundle with M15 / X2 / X3 as the "napi hygiene"
      PR.
- [x] **X5.** **Decided 2026-04-24 — Option A.** INTD-001 and
      INTD-002 are picked up straight after the v0.4.0-beta
      release. The TS scanner and the `tests/scanner-parity/`
      harness remain live until TSRET-005 fires (expected 2+
      months), which is the explicit cost accepted under Option A
      for a single architecturally clean migration to the
      daemon-hosted scanner. Decision recorded in ADR-030
      References ("Sequencing decision" section).

---

## Landed PRs

| PR | What | Resolves |
|---|---|---|
| [#1064](https://github.com/eddacraft/anvil-001/pull/1064) | fix(napi): surface registry-unavailable as a loud error | C1, M16, X1 |
| [#1065](https://github.com/eddacraft/anvil-001/pull/1065) | docs(aps): route council findings into INTD, DRVR, TSRET work items | M8 + M13 (amendments); C2 / M1–M7 / M9–M12 / M14 / S6 / S7 (routed to APS items); S2 / S3 / S4 / S5 (plan doc clean-up) |
| [#1062](https://github.com/eddacraft/anvil-001/pull/1062) | docs(aps): KERN Phase 5 superseded by INTD | S1 (merge ordering now satisfied) |

## New APS work items (filed in #1065)

| Module | ID | Council ref |
|---|---|---|
| INTD | [INTD-014](../archive/modules/intercept-daemon.aps.md) JSON-RPC conformance + latency bench | M1 |
| INTD | [INTD-015](../archive/modules/intercept-daemon.aps.md) daemon-enforced telemetry scoping | M5 |
| INTD | [INTD-016](../archive/modules/intercept-daemon.aps.md) DoS protection budgets | M9 |
| DRVR | [DRVR-006](../archive/modules/surface-drivers.aps.md) Pin MCP daemon-RPC surface | C2 |
| DRVR | [DRVR-007](../archive/modules/surface-drivers.aps.md) Driver trust + enforcement contract | M5 / M6 / M7 / M11 |
| DRVR | [DRVR-008](../archive/modules/surface-drivers.aps.md) Non-VSCode LSP capability negotiation | M10 |
| TSRET | [TSRET-006](../archive/modules/anvil-ts-scanner-retirement.aps.md) Engine-version + divergence canary | M14 |

Six more findings are baked into existing work-item expected outcomes rather than new items:

- **DRVR-002 expected outcome** carries M2, M3, M4, M6, M12, S6, S7 as explicit sign-off prerequisites.
- **DRVR-001 expected outcome** carries M13 (partial-failure surface) and the driver-side half of M8.
- **INTD-002 expected outcome** carries the full M8 socket-creation sequence.

---

## Still-open checklist

All 30 findings now have a home. Remaining execution lives in
follow-up work items rather than on this tracker:

- **M15, X2, X3, X4** — landing in PR #1068 (napi hygiene bundle).
- **X5** — decided. Option A recorded in ADR-030 References.
- **C2, M1–M7, M9–M12, M14, S6, S7** — follow via their APS work
  items (DRVR-001 through DRVR-008, INTD-002 / -014 / -015 / -016,
  TSRET-006, plus the expected-outcome amendments on DRVR-001 /
  DRVR-002 / INTD-002).

---

## How this tracks

This doc is the single source of truth for the review. In the checklist
above, `[x]` means the finding is **accounted for** — either the fix has
landed, or the finding has a specific APS work-item home, or a decision
has been recorded in the ADR. It does NOT mean the follow-up work is
implemented. Execution status lives in the linked DRVR / INTD / TSRET
work items and any inline status notes next to each finding; keep those
in sync as work progresses, but don't use the checkbox itself to
distinguish "routed to APS" from "fully implemented." The mapping is the
authoritative trace from "council reviewer said X" back to the owning
follow-up or landed change.

When the tracker's purpose is spent, close or retarget this PR and move
the already-tracked doc to `plans/reviews/archive/` (new files under
`plans/reviews/` are gitignored and need to be added explicitly; see
existing files in the tree for the pattern).

Reviewer raw outputs (unabridged) live in the session transcript; this
summary is deliberately condensed for operational use.
