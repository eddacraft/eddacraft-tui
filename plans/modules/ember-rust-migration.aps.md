<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if work items exist and status is Ready. -->

# Ember Rust Migration

| ID      | Owner | Priority | Status      |
| ------- | ----- | -------- | ----------- |
| EMBERRS | —     | P1       | In Progress |

**Packages:** anvil-cli, anvil-kernel-types, anvil-runtime, anvil-edda-stack,
flags-catalogue

**Authority:** Operator direction on 2026-09-05: retire unused JavaScript
runtime paths, disconnect TypeScript Ember, default-disable Ember, and plan its
Rust migration. Windows is supported; code signing is outside this work.

**Exclusive module:** One writer owns this migration and its closeout.

## Purpose

Make Ember a Rust-owned candidate-memory capability with an explicit activation
boundary. A Rust list command over a historical database is not a functioning
candidate-generation service, and completed TypeScript work is not Rust parity.

## Current state and disposition

Source audited at `96bcaf39fd436ea5d68415dedc62c90976db1236`:

- Rust implements `anvil ember list` over the historical proposal SQLite schema.
- TypeScript owns the old aggregation, evaluation, persistence, decay and
  observation-hook implementations; no production construction of those services
  was found. EMBERRS-001 retires that implementation, its package export and
  native SQLite dependency. Git history preserves migration evidence.
- JS watch and agent/lock/queue orchestration have no application callers in the
  current tree. EMBERRS-001 removes them and their exports; the cache's atomic
  text-write helper survives as a private cache utility.
- The API/docs feature-flag resolver remains live and is not retired.
- Shared TS memory contracts, fixtures and the separate Edda implementation
  remain. They do not instantiate or reconnect an Ember implementation.

## Contracts

- `ember.enabled` defaults off, with no automatic environment/audience rollout.
  `ANVIL_EMBER=1` explicitly opens the existing Rust reader only. `ANVIL_DEV`
  and admin credentials do not opt into Ember. Invalid opt-in values remain off.
- The CLI is unlisted and hidden while default-off. Disabled invocation returns
  a non-zero, structured `feature_disabled` result before locating/opening the
  proposal database. Opt-in does not start a JS process or candidate generator.
- No retirement operation deletes or rewrites user databases, memories, Git
  history or installed hooks. Old generated Node hooks are unsupported; the
  migration audit must document detection and safe removal without rewriting
  unrelated user hook content.
- Future processing is bounded and replay-safe. Proposal persistence and pending
  publication commit atomically, retries retain proposal/event identity, and
  consumers tolerate at-least-once delivery. No exactly-once delivery claim.
- Ember proposes meaning; promotion into Edda remains an explicit attributed
  human decision. The migration does not turn probabilistic interpretation into
  enforcement or bypass Edda provenance requirements.
- Windows, Linux and macOS are first-class verification targets. No Node runtime
  is required for the completed Ember journey.

## Boundaries and dependencies

Coordinates with [CLAWOPEN-009](clawpatch-open-findings-repair-wave.aps.md):
retirement resolves the affected JS reachability; its publication-failure
regression becomes mandatory Rust evidence under EMBERRS-004. Retirement is not
evidence that the historical implementation was fixed.

Coordinates with [RCLI3](rust-cli-tier3.aps.md): RCLI3-005 is the existing
reader; RCLI3-006/-007 and Edda promotion work must share EMBERRS storage and
lifecycle contracts, not introduce a second proposal writer. Their existing
records stay open until their own acceptance criteria are met.

[AGOV](agent-governance-patterns.aps.md) and [PFGW](pocketflow-gateway.aps.md)
must target Rust interfaces for future Ember integration. Archived EMBER/EERB
plans are historical TypeScript evidence only.

Out of scope: implementing the Rust migration in the containment PR; enabling
Ember by default; migrating the whole Edda system; new AI/model dependencies;
hosted/multi-tenant memory; Windows signing; unrelated TS API/docs retirement.

## Work Items

### EMBERRS-001: Retire JS execution and default-disable Ember

- **Status:** Merged 2026-09-05 via PR #4399
- **Closeout evidence (2026-09-09):**
  [#4399](https://github.com/eddacraft/anvil-001/pull/4399) merged to `main` at
  `90ca2116` on 2026-09-05 (89 files, +1325/-12756), closing claim #4398 via
  `Fixes #4398`. JavaScript Ember plus the runtime watch and concurrency
  implementations and their package exports are deleted; the live cache and
  feature-flag surfaces are preserved. `ember.enabled` is canonical and
  default-off, `anvil ember` is hidden and refuses before database access, only
  explicit `ANVIL_EMBER=1` reaches the Rust historical reader, and existing
  databases are untouched. Three retirement regressions passed having failed
  first; the remaining Edda-stack suite ran 733 tests across 27 files. Required
  two-reviewer mini Council completed with no blocking findings.
- **Priority:** P1
- **Intent:** Remove obsolete JS execution surfaces and make Ember explicitly
  inactive.
- **Expected Outcome:** JS watch/concurrency and Ember service implementations
  are absent from source/build/package exports. Live cache/feature-flag imports
  still work. The Rust reader is hidden, unlisted and default-off; explicit
  opt-in retains read compatibility. Existing user data is untouched. Negative
  export/source checks and process-level flag tests enforce the boundary.
- **Files:** runtime and edda-stack packages, CLI Ember/flag dispatch, canonical
  flag manifests, e2e smoke, owning documentation and this module/index.
- **Validation:** `node --test scripts/ci/legacy-runtime-retirement.test.mjs`;
  `pnpm --filter @eddacraft/anvil-runtime test`;
  `pnpm --filter @eddacraft/anvil-edda-stack exec vitest run`;
  `cargo test -p eddacraft-anvil --test ember_gate`;
  `cargo test -p eddacraft-anvil --bin anvil ember`;
  `pnpm docs:catalogue:check`; `pnpm docs:check`
- **Claim:** [#4398](https://github.com/eddacraft/anvil-001/issues/4398)

### EMBERRS-002: Establish Rust contracts and migration evidence

- **Status:** Ready
- **Priority:** P1
- **Intent:** Fix the Rust ownership, storage and integration contracts before
  implementing parity.
- **Expected Outcome:** An ADR names the owning crate, public interfaces,
  Kindling handoff, configuration/flag precedence, Edda promotion boundary,
  processing budgets and recovery semantics. A capability matrix maps historical
  behaviour to retain/change/retire outcomes. Versioned fixtures cover real
  proposal schema, IDs, confidence, TTL, provenance and resolution records;
  existing database/hook discovery has a non-destructive disposition.
- **Dependencies:** EMBERRS-001
- **Validation:** `pnpm adr:check`; `pnpm docs:check`; review every parity row
  against pinned historical TS source and the Rust reader.

### EMBERRS-003: Own proposal storage and lifecycle in Rust

- **Status:** Proposed
- **Priority:** P1
- **Intent:** Provide one Rust authority for proposal persistence and lifecycle
  transitions.
- **Expected Outcome:** Historical databases remain readable and migrations are
  versioned, restart-safe and non-destructive. Create/query/update,
  confidence/TTL validation, capacity limits, expiry/pruning, dismissal and
  promotion claims have deterministic transaction semantics. Invalid or newer
  schemas fail explicitly; concurrent writers cannot exceed capacity or resolve
  the same proposal twice. The existing CLI reader uses the same store.
- **Dependencies:** EMBERRS-002
- **Validation:** `cargo test --workspace ember_storage --no-fail-fast` against
  real SQLite fixtures, concurrent writes and injected migration failures.

### EMBERRS-004: Make proposal creation and publication replay-safe

- **Status:** Proposed
- **Priority:** P1
- **Intent:** Prevent duplicate proposals and lost publication across failures
  and retries.
- **Expected Outcome:** A stable operation key deduplicates creation;
  conflicting key reuse fails. Proposal and pending event commit atomically.
  Publish failure preserves successful creation; bounded retry/restart retains
  event identity. Delivery acknowledgement crashes may redeliver but cannot
  duplicate downstream effects. Pending/failed publication is observable without
  logging proposal text.
- **Dependencies:** EMBERRS-003
- **Validation:** `cargo test --workspace ember_publication --no-fail-fast`;
  reproduce CLAWOPEN-009 publication failure, identical/concurrent retry,
  restart and delivery-before-ack crash against the real store.

### EMBERRS-005: Generate candidates from Kindling in Rust

- **Status:** Proposed
- **Priority:** P1
- **Intent:** Connect bounded observation processing to the Rust candidate
  lifecycle.
- **Expected Outcome:** An explicit Rust host consumes the supported Kindling
  handoff, aggregates/evaluates supported rules and persists candidates with
  exact observation provenance. Session replay is idempotent. Work, buffers,
  deadlines and shutdown are bounded. Flag-off prevents subscription, spawning,
  evaluation and writes; opt-in cannot fall back to TS. Deterministic rules work
  without a model provider.
- **Dependencies:** EMBERRS-004
- **Validation:** `cargo test --workspace ember_pipeline --no-fail-fast`;
  exercise observation-to-proposal integration, replay, overload, cancellation
  and disabled-state inactivity.

### EMBERRS-006: Complete candidate review and Edda handoff

- **Status:** Proposed
- **Priority:** P1
- **Intent:** Provide a coherent Rust review journey with explicit human
  authority.
- **Expected Outcome:** List/show/dismiss and the chosen promotion entry point
  use the shared Rust service with stable JSON errors. Promotion preserves
  actor, reason and source provenance, and reconciles partial failures across
  proposal state and Edda storage without duplicate memories or false success.
  CLI/MCP exposure shares the flag boundary; any new host is catalogued.
  Coordinate RCLI3-006/-007 and Edda promotion acceptance rather than
  duplicating their work.
- **Dependencies:** EMBERRS-003, EMBERRS-004, EMBERRS-005
- **Validation:** `cargo test --workspace ember_review --no-fail-fast`;
  real-store review/promotion journey including retry and cross-store failure.

### EMBERRS-007: Prove portability and retire migration compatibility

- **Status:** Proposed
- **Priority:** P1
- **Intent:** Prove that the complete candidate journey works without
  JavaScript.
- **Expected Outcome:** One candidate SHA passes Windows/Linux/macOS fresh-use,
  historical-database upgrade, restart, flag-off, replay and promotion scenarios
  with Node absent. No JS startup/import/package fallback remains. Contract
  fixtures survive removal of temporary migration readers/adapters.
  Documentation and product catalogue describe actual supported capabilities.
- **Dependencies:** EMBERRS-006
- **Validation:** `cargo test -p eddacraft-anvil --no-fail-fast`;
  `node --test scripts/ci/legacy-runtime-retirement.test.mjs`;
  `pnpm docs:check`; packaged three-platform journey evidence.

### EMBERRS-008: Review Ember activation

- **Status:** Proposed
- **Priority:** P2
- **Intent:** Make default activation a separate evidence-backed product
  decision.
- **Expected Outcome:** Review the flag by 2026-10-05. Record
  enable/continue-off with rationale and a next review date. Default enablement
  requires explicit operator acceptance after EMBERRS-007, aligned CLI
  visibility/catalogue state, resource budgets, recovery guidance and an
  operational disable path.
- **Dependencies:** EMBERRS-007
- **Validation:**
  `pnpm --filter @eddacraft/anvil-flags-catalogue exec vitest run`;
  `cargo test -p eddacraft-anvil --test ember_gate`; `pnpm docs:check`

## Verification policy

Rust test filters above are acceptance-suite names to establish in their owning
items, not a claim those suites already exist. Each item must record the exact
crate/target and prove a non-zero relevant test count before closeout. Mock-only
storage/publication tests and CLI list parity do not prove the full migration.
