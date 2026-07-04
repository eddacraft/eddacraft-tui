# APS Loop Journal

One entry per loop cycle. Resume point for interrupted runs and the
operator's audit trail. Bookkeeping only — committed with plan changes,
never with feature work.

## Cycle 1 — 2026-06-10

- Item: UJ-005 — `anvil status` always states the save-time posture
- Outcome: done (validation: `cargo test -p eddacraft-anvil
  commands::status` 52 green incl. 6 new posture tests; workspace clippy
  `-D warnings` clean; fresh-context outcome verification PASS;
  quick-tier council review no blocking findings, both MINOR findings
  addressed). PR #2500, auto-merge armed. Manual transcript blocked by
  the intentional beta licence gate — unit tests carry the rendering
  validation.
- Plan changes: UJ-005 → Merged 2026-06-10 via PR #2500; UJ module +
  index counts 1/11 (script-managed).
- Checkpoints raised: none
- Next: UJ-006 (watch help/advisory daemon guidance) — independent
  files; bookkeeping flips deferred until #2500 lands to avoid index
  count-cell collisions.

## Cycle 2 — 2026-06-10

- Item: UJ-006 — Daemon guidance on the watch surface and help
- Outcome: done (validation: `cargo test -p eddacraft-anvil
  commands::watch` 75 green incl. 2 new tests — long-help assertion via
  the real `Cli::command()` surface + fallback-advisory pointer;
  workspace clippy clean; `anvil watch --help` transcript shows the
  Save-time daemon section; fresh-context verification CONDITIONAL PASS
  — both MAJOR findings fixed: misattached doc-comment split, `=1`
  wording corrected to match the warned-fallback implementation; ASCII
  advisory per the watch banner policy). PR #2501.
- Plan changes: UJ-006 → Merged 2026-06-10 via PR #2501; UJ counts 2/11
  (script-managed).
- Checkpoints raised: none
- Next: UJ-001 (golden-path next-step threading in install/welcome/
  init/start).

## Cycle 3 — 2026-06-10

- Item: UJ-001 — Golden-path next-step threading in CLI and install output
- Outcome: done (validation: 6 new closing-hint tests green — start ×3
  incl. restart_required honesty case, init, welcome ×2; full crate
  suite green; workspace clippy + fmt clean; `bash -n install.sh`
  clean; fresh-context verification PASS — all four surfaces' exit
  paths traced, live-claim-at-restart_required finding fixed with a
  pinning test). PR #2502.
- Plan changes: UJ-001 → Merged 2026-06-10 via PR #2502; UJ counts 3/11
  (script-managed).
- Checkpoints raised: none
- Next: UJ-003 (entry docs rewritten around the two paths) — in flight
  in a parallel worktree.

## Cycle 4 — 2026-06-10

- Item: UJ-003 — Quickstart and beta guide rewritten around the two paths
- Outcome: done (validation: `pnpm docs:check` 8/8 surfaces green;
  `pnpm docs:index` + oxfmt clean; fresh-context verification
  CONDITIONAL PASS — both findings fixed: "in every routing state"
  save-time claim corrected for the =0 opt-out, ANVIL_HOME note
  re-worded to platform-standard paths incl. Windows %APPDATA%. Drift
  corrected: item said `anvil mcp-install`; real surface is `anvil
  start` wiring + `anvil mcp-config --target`; quickstart documents the
  real commands and the module item carries the correction note.)
  PR #2503.
- Plan changes: UJ-003 → Merged 2026-06-10 via PR #2503; UJ counts 4/11
  (script-managed); UJ-003 body annotated with the mcp-install drift
  correction.
- Checkpoints raised: none
- Next: UJ-008 (consolidated save-time validation guide).

## Cycle 5 — 2026-06-10

- Item: UJ-008 — Consolidated save-time validation guide
- Outcome: done (validation: `pnpm docs:check` 8/8; docs:index + oxfmt
  clean; new guide carries governance + Upstream/Downstream tables and
  the sidebar entry; fresh-context fact-check FAIL→fixed: nonexistent
  `anvil workspace remove` corrected to `deny` (also a pre-existing
  config.md bug), nested-backtick span, `--foreground` requiredness,
  reconnect-rescan guarantee softened to best-effort, Windows parity
  claim narrowed to daemon-status correlation). PR #2504.
- Plan changes: UJ-008 → Merged 2026-06-10 via PR #2504; UJ counts
  (script-managed).
- Checkpoints raised: none
- Next: UJ-002 validation + UJ-009.

## Cycle 6 — 2026-06-10

- Item: UJ-002 — Welcome path lands on a populated surface
- Outcome: done, no change needed (validation: code trace — welcome hub
  options all gather live data at launch; first-run lands on the
  discovery-scan-populated tutorial; no welcome surface recommends a
  persisted-data target. Item body carries the disposition.)
- Plan changes: UJ-002 → Done 2026-06-10 (verified — no change needed).
- Checkpoints raised: none
- Next: UJ-009 (gate-summary reach) — implemented in a parallel
  worktree, PR to follow.

## Cycle 7 — 2026-06-10

- Item: UJ-009 — Gate-summary dashboard reaches existing projects
- Outcome: done (chosen path: zero-write built-in fallback — the picker
  and `anvil dashboard gate-summary` serve the embedded
  GATE_SUMMARY_SPEC whenever no saved spec shadows it; saved specs
  always win; embedded spec stays the CIB-053 single source. Validation:
  4 new tests green + full anvil/anvil-tui suites; clippy+fmt clean;
  real transcripts in a fresh repo — picker --json lists the builtin
  entry, direct launch resolves where it previously bailed. Subagent
  verification unavailable (spend limit); all six surfaces traced
  inline for shadowing consistency. Accepted edge: outside any
  workspace the direct launch falls through to the unknown-name error.)
  PR #2505.
- Plan changes: UJ-009 → Merged 2026-06-10 via PR #2505; UJ counts 7/11
  (script-managed).
- Checkpoints raised: none
- Next: UJ-010 (PR #2506 open, flips deferred); then only design-gated
  items (UJ-004, UJ-007) and UJ-011 shaping remain.

## Cycle 8 — 2026-06-10

- Item: UJ-010 — Post-upgrade what's-new one-liner
- Outcome: done (validation: 5 new `whats_new` tests green — seed-
  silently, same-version silent, announce-with-pointer, exactly-once
  across runs, suppressed/gated runs write nothing; full anvil +
  anvil-tui suites green; workspace clippy + fmt clean. Manual
  transcript blocked by the licence gate; scope note: the line carries
  version + changelog pointer, a fetched release headline would need
  the network probe and was deliberately left out.) PR #2506.
- Plan changes: UJ-010 → Merged 2026-06-10 via PR #2506; UJ counts 8/11
  (script-managed); UJ-011 annotated with the shaping draft
  (`plans/execution/UJ-011.shaping.md`); lessons recorded in
  `plans/execution/lessons/2026-06-10-uj-loop-run.md`.
- Checkpoints raised:
  - UJ-004 (auth-wall placement vs first wow): needs a new ADR — every
    manual transcript this run hit the login wall before any value,
    exactly the failure the item names; `dashboard` is already ungated
    and demonstrates the alternative. Proposal for the operator: ungate
    read-only `anvil welcome` as the demo surface, keep durable
    surfaces (`init`, `start`, `watch`) gated.
  - UJ-007 (watch offers to start the daemon): UJ-001/UJ-006 shipped
    "run `anvil start`" guidance on every relevant surface; the item
    itself names guidance-only as an acceptable resolution. Proposal:
    affirm guidance-only (no prompt, no auto-start) as an ADR-075
    amendment and close UJ-007.
  - UJ-011: shaping draft ready for operator review (see above);
    follow-up items UJ-012..015 file on approval.
- Next: stop — no Ready items remain; everything left is
  checkpoint-level (UJ-004, UJ-007, UJ-011 review).

## Cycle 9 — 2026-06-10

- Item: UJ-009 follow-up — seeded gate-summary stem shadows the builtin
- Outcome: done (a project memory note surfaced a shipped bug in
  #2505 minutes after merge: `spec::discover` names saved specs by file
  stem, so the init-seeded spec is `gate-summary.dashboard`, which the
  exact-name shadow check missed — seeded projects double-listed the
  dashboard and the direct launch served the embedded copy over the
  saved spec. Fix: `.dashboard`-suffix-aware shadow check + saved-spec
  precedence at both launch dispatch points. Validation: new
  red→green test, 29 dashboard tests green, clippy+fmt clean, real
  transcript on a simulated seeded project — single listing, direct
  launch resolves to the saved spec.) PR #2507.
- Plan changes: none (fix to already-Merged UJ-009; no status change).
- Checkpoints raised: none new (UJ-004 / UJ-007 / UJ-011 review stand
  from cycle 8).
- Next: stop — no Ready items remain; UJ is 8/11 with three
  checkpoint-level items (UJ-004 ADR, UJ-007 decision, UJ-011 shaping
  review) awaiting the operator.

## Cycle 10 — 2026-06-10

- Item: UJ-007 + UJ-011 — operator decisions executed
- Outcome: done (operator: "guidance-only" for UJ-007 → ADR-079
  Accepted, zero code; UJ-011 shaping approved with both open questions
  answered — fold ci.md into the GitHub guide, align web tutorials with
  the in-terminal `anvil tutorial` narrative). UJ-004 ("ungate
  welcome") executes next as ADR-080 + the CLI_GATED_COMMANDS change.
- Plan changes: UJ-007 → Done (ADR-079); UJ-011 → Done (shaping
  approved); UJ-012..015 filed as Ready (tutorial execution set);
  DECISION-LOG row for ADR-079; shaping doc marked approved.
- Checkpoints raised: none — all three cycle-8 checkpoints now have
  operator answers.
- Next: UJ-004 (ADR-080 + ungate `welcome`), then UJ-012..015 are the
  remaining Ready work.

## Cycle 11 — 2026-06-10

- Item: UJ-004 — Auth-wall placement vs first wow
- Outcome: done (operator: "ungate welcome" → ADR-080 Accepted +
  `welcome` removed from CLI_GATED_COMMANDS. Validation: TDD red→green
  on `requires_auth_welcome` (now asserts NOT gated) +
  `command_needs_licence_gate_rejects_bypass_commands` pins the
  exclusion; full suite, clippy, fmt green; real transcript —
  unauthenticated `anvil welcome` renders the surface ending in the
  `anvil start` handoff, where it previously printed the
  refresh-token error. ADR records the accepted consequence that
  welcome's guided setup can seed config without auth.) PR #2509.
- Plan changes: UJ-004 → Merged 2026-06-10 via PR #2509; UJ counts
  11/15; DECISION-LOG row for ADR-080.
- Checkpoints raised: none — all operator decisions executed.
- Next: UJ-012..015 (tutorial execution set) are the module's remaining
  Ready work.

## Cycle 12 — 2026-06-10

- Item: UJ-012 — "Your first save caught" tutorial
- Outcome: done (new flagship tutorial
  `docs/public/anvil/tutorials/first-save-caught.md` walking
  `anvil start` → daemon (`anvil intercept start --foreground`) →
  `anvil watch --source` → deliberate bad save → reading the finding →
  `anvil status` posture, narrative-aligned with the in-terminal
  ProtectionLoop path; linked from the quickstart daily-value step and
  the tutorials index. Validation: `pnpm docs:check` 8/8,
  `pnpm docs:index` clean, `format:check` exit 0; fresh-context
  verification confirmed every transcript snippet against the rendering
  code and caught one overclaim — `anvil start` does not launch the
  daemon (activation orchestrator excludes daemon spawn by design) —
  fixed before PR. Live end-to-end walk blocked by the agent
  environment's licence gate + inotify limits; gate-unavailable is not
  a content veto.) PR #2510.
- Plan changes: UJ-012 → Merged 2026-06-10 via PR #2510; UJ counts
  12/15; stale "UJ is 10/15" release-plan row note reconciled.
- Checkpoints raised: none.
- Next: UJ-013 (Rust analysis tutorial) — Ready, independent of UJ-012.

## Cycle 13 — 2026-06-10

- Item: UJ-013 — "Analyse a Rust project" tutorial
- Outcome: done (new tutorial
  `docs/public/anvil/tutorials/rust-project.md`: discovery via
  `anvil welcome` / `anvil check --all`, RS-001..005 advisory catalogue
  with the cfg(test)-exclusion scope stated per rule, suppression
  syntax, the `anvil start --verify` language-profile claim, ending on
  the daily-value handoff. Listed in the tutorials index. Validation:
  docs:check 8/8, docs:index clean, format:check exit 0; fresh-context
  verification caught two real inaccuracies pre-PR — test exclusion is
  RS-001/002-only, and the sample output was missing the message line —
  both fixed.) PR #2511.
- Plan changes: UJ-013 → Merged 2026-06-10 via PR #2511; UJ counts
  13/15.
- Checkpoints raised: none.
- Next: UJ-014 (tutorial refresh + index rewrite) — architecture/drift
  legs already drafted in a parallel worktree; index rewrite unblocks
  once #2511 merges.

## Cycle 14 — 2026-06-10

- Item: UJ-014 — Refresh surviving tutorials + journey-aligned index
- Outcome: done (architecture.md gains the Rust architecture.yaml
  example, resolution-semantics note, and Rust fix example; its
  misleading Suppress block — `@anvil-ignore ARCH-001` is not wired to
  boundary violations — replaced with depends_on edits + the
  new-edges-only baseline posture; drift.md cross-links the
  dashboard/insights guides; index.md rewritten around the two beta
  paths, naming the in-terminal `anvil tutorial` as the interactive
  sibling, listing the final five-tutorial set with guide pointers
  replacing the ci/suppressions rows. Validation: docs:check 8/8,
  docs:index clean, format:check exit 0; two fresh-context reviews —
  the suppression doc bug was their catch.) PR #2513.
- Plan changes: UJ-014 → Merged 2026-06-10 via PR #2513; UJ counts
  14/15.
- Checkpoints raised: none.
- Next: UJ-015 (retire ci/suppressions into their guides) — unblocked
  once #2513 merges. Pre-verification already done: suppressions.md is
  heavily drifted (wrong suppressions.json schema vs the shipped
  loader, no SUP-001 rule, comma multi-rule unsupported by the ADR-029
  regex) — the fold carries only code-verified content.

## Cycle 15 — 2026-06-10

- Item: UJ-015 — Retire ci and suppressions tutorials into their guides
- Outcome: done (folds: github.md gains matrix/auth-tip/ci-profile/
  other-CI-systems; insights.md gains "Suppressing a finding" with the
  code-verified inline syntax; git-hooks.md gains the staged-only
  pre-commit example. Removals + sweep: both tutorial pages deleted;
  quickstart/first-project/git-hooks/git-hook-compatibility repointed;
  BOTH docs apps' manual sidebars updated — the sweep also added the
  UJ-012/UJ-013 pages that were missing from those explicit lists; the
  docs-check baseline regenerated canonically. Deliberately not
  carried: the old file-level/bulk-suppression sections (drift —
  suppressions.json is export/dashboard-only, schema mismatched the
  shipped loader), SUP-001 (nonexistent), comma multi-rule directives
  (unsupported). Validation: docs:check 8/8, docs:index clean,
  format:check exit 0, Docusaurus build SUCCESS; fresh-context review's
  two real findings fixed pre-PR, its third resolved by #2513's merge.)
  PR #2514.
- Plan changes: UJ-015 → Merged 2026-06-10 via PR #2514; UJ counts
  15/15; module status → Complete (archive deferred to release-tag
  inclusion per archiving rules).
- Checkpoints raised: none.
- Next: stop — the UJ module is fully dispositioned; no Ready items
  remain in the UJ-012..015 scope this run was invoked for.

## Cycle 16 — 2026-06-11

- Item: GHCLIAUTH-007 — Tombstone the activation page + rebuild
  admin-invite activation
- Outcome: done (activation page is a static tombstone — no 404 for
  outstanding invite links or shipped-CLI verification URLs; invite
  email rewritten around `anvil auth login` + `--otp`, dropping
  ACTIVATE_URL/userCode across sendBetaInvite, the BetaInvite template,
  and betaInvitePropsSchema; interactive device-code generation removed
  from BOTH /admin/invite and /admin/approve — the approve path was newly
  discovered scope, drift-corrected into the item; the tokenOnly path
  and approve's access_tokens scope-record insert are untouched.
  Validation: pnpm nx test @eddacraft/anvil-api 487 passed,
  @eddacraft/transactional tests, nx build website, typecheck + lint,
  format:check, index-counts --check; fresh-context verification PASS
  on all six Expected Outcome bullets, invited-user activation wiring
  traced end-to-end through linkOrCreateGitHubUser/OTP mint.) PR #2549.
- Plan changes: GHCLIAUTH-007 → Merged 2026-06-11 via PR #2549; counts
  7/11; cleanup slice 1/5 In Progress; drift correction recorded
  (approve path + access_tokens rationale + email-registry file added
  to Files). GHCLIAUTH-009 dispatched in parallel (worktree
  wt/ghcliauth-009, implementation complete pending parent review);
  its In Progress flip rides its own PR.
- Checkpoints raised: none.
- Next: GHCLIAUTH-009 (observability + runbook) review → PR; then 011
  (headless E2E smoke). 008 waits for the next CLI tag; 010 depends
  on 008.

## Cycle 17 — 2026-06-11

- Item: GHCLIAUTH-009 — Observability + ops hardening + runbook
- Outcome: done (ungated structured console.info — createInfoLogger in
  lib/debug.ts — at every device-flow upstream-call outcome: latency,
  outcome, error class, HTTP status; no-secrets contract with
  sanitizeForLog defence-in-depth and reserved-key/event-clamp
  hardening; authorization_pending deliberately suppressed from the
  info stream as per-poll spam, kept on the gated debug stream;
  log-hygiene tests assert real mock secret values absent and no
  [REDACTED] ever appears, mutation-verified by the fresh-context
  reviewer; docs/runbooks/github-device-flow.md lands topology, creds
  wiring, /health degraded gating, the pre-cutover "Device Flow
  enabled" smoke step, rate limits, mint semantics, log taxonomy,
  troubleshooting including the mint-race slow_down loser path, --otp
  fallback. Validation: pnpm nx test @eddacraft/anvil-api 490 passed,
  docs:check 8/8, docs:index clean, format:check exit 0, index-counts
  exit 0.) PR #2552; review fixes: per-poll pending suppression
  (major), identity.upstream ms measured on the fetch only, clamped
  upstream error string, strengthened hygiene assertions, Copilot
  reserved-key hardening.
- Plan changes: GHCLIAUTH-009 → Merged 2026-06-11 via PR #2552; counts
  8/11; cleanup slice 2/5. Drift recorded in the module note: /health
  inline in index.ts (no routes/health.ts); info stream reuses the
  auth-github-device namespace as a distinct event stream.
- Checkpoints raised: none.
- Next: GHCLIAUTH-011 (headless E2E smoke) — implemented, verified
  fresh-context PASS, In Progress on PR #2553 (auto-merge armed).
  After 011: 008 waits for the next CLI tag (only externally-gated
  work remains); 010 depends on 008.

## Cycle 18 — 2026-06-11

- Item: GHCLIAUTH-011 — End-to-end headless smoke test
- Outcome: done (five wiremock-backed integration tests in
  crates/anvil-cli/tests/device_flow_e2e.rs drive the real anvil binary:
  start→poll→confirmed asserts on-disk credential content and the whoami
  identity round-trip body-matched on the exact minted licence;
  slow_down proves back-off-not-bail with .expect() poll counts — fails
  against the pre-006 fatal-bail; expired/declined assert failure AND
  no persistence; --otp drives the real stdin prompt path. All poll
  mocks body-match pollToken; temp ANVIL_HOME re-roots credentials on
  every platform; ANVIL_DEV/ANVIL_LICENSE bypasses env_remove'd.
  Validation: cargo test -p eddacraft-anvil -- device_flow_e2e 5 passed
  in ~2s; workspace clippy -D warnings exit 0; cargo fmt --all --check
  exit 0; format:check + index-counts exit 0; fresh-context
  verification PASS on every clause including a could-this-green-a-
  broken-flow audit; five Copilot threads fixed + resolved.) PR #2553.
- Plan changes: GHCLIAUTH-011 → Merged 2026-06-11 via PR #2553; counts
  9/11; cleanup slice 3/5; whoami drift correction recorded in the item
  (resolves via live /api/v1/auth/verify, not offline). Lesson note
  added: plans/execution/lessons/2026-06-11-ghcliauth-loop-run.md.
- Checkpoints raised: none.
- Next: stop — no locally-actionable work remains in GHCLIAUTH. 008
  (confirm-endpoint removal) is sequenced after the next CLI tag ships
  the new login (ADR-066 decision 5); 010 (docs sync) depends on 008.
  The module stays In Progress at 9/11 until that external release
  gate opens.

## Cycle 19 — 2026-06-11

- Item: GHCLIAUTH-008 — Remove `POST /auth/device/confirm` + #1779
  dead code
- Outcome: done, under a recorded owner sequencing override (ADR-066
  decision 5 sequenced this after the next CLI tag; the owner
  authorised early removal — no active users, and the path has been
  un-completable since #1779, so shipped CLIs lose nothing that
  worked). Removed: the /confirm handler + schema + brute-force
  ceiling + attempt-counter/lockout, three confirm-only queries.ts
  functions, 24 confirm tests, and the orphaned requireAuth middleware
  (#1779 hardening whose only consumer was the deleted route;
  verifyLicence stays as the library surface, /health still probes the
  verifying key). Kept: /auth/device/start+poll (shipped CLIs; future
  retirement pass — poll's unreachable mint branch annotated), the
  F-C-003 dummy-row insertion (now guarding /poll anti-enumeration,
  comments repointed), applied migrations, OTP (0-line diff).
  Validation: pnpm nx test @eddacraft/anvil-api 476 passed; rg
  "device/confirm|confirmDeviceCode" clean except the immutable
  migration comment; rg requireAuth zero hits; typecheck/lint/format/
  index-counts exit 0; fresh-context verification PASS per clause with
  a no-breakage audit of start/poll/OTP.) PR #2556.
- Plan changes: GHCLIAUTH-008 → Merged 2026-06-11 via PR #2556; counts
  10/11; cleanup slice 4/5. Drift recorded: migration 012 keeps its
  historical /device/confirm comment; generic orphaned finders
  (findDeviceCodeByUserCode/findDeviceCodeByPollToken) predate the item
  and are left for a separate cleanup.
- Checkpoints raised: sequencing override requested and granted by the
  owner in-session ("noone can use the app right now"); recorded in the
  item, module prose, and Release wave section.
- Next: GHCLIAUTH-010 (docs sync) — the last item, now unblocked;
  dispatched immediately, In Progress flip rides its PR.

## Cycle 20 — 2026-06-11

- Item: GHCLIAUTH-010 — Docs sync (auth/activation/quickstart/beta
  guide)
- Outcome: done (auth-as-built.md rewritten to the device-flow default:
  RFC 8628 broker narrative, github_id-first linking precedence, a
  Legacy Device Code Flow section documenting start+poll as still-live
  shipped-CLI compatibility with /confirm recorded as removed,
  /api/v1 prefixes made consistent, stale auth-github.ts line pins
  refreshed against current code; quickstart auth step + beta guide
  Sign In aligned to open-on-any-device with a next-beta caveat,
  tag-ancestry-verified. Validation: docs:check 8/8, docs:index clean,
  format:check + index-counts exit 0; fresh-context verification PASS
  with every technical claim fact-checked against route/CLI/linking
  code; six Copilot accuracy threads fixed + resolved.) PR #2558.
- Plan changes: GHCLIAUTH-010 → Merged 2026-06-11 via PR #2558; counts
  11/11; cleanup slice 5/5 Complete; MODULE → Complete. Drift recorded:
  activation-as-built.md was a name-match false positive in the item's
  file list (anvil start product activation, no auth content). Archive
  deferred to release-tag inclusion per archiving rules; legacy
  start+poll retirement noted as future intake, not a module item.
- Checkpoints raised: none.
- Next: stop — GHCLIAUTH is fully dispositioned (11/11 Complete). The
  only follow-on is operational, not plan work: cut the next CLI tag so
  release-channel users receive the device-flow login.

## Cycle — 2026-07-04 (POLRESET-001)

- Item: POLRESET-001 — Policy value and enforcement design gate
- Outcome: done (Merged via PR #3121)
- Validation: `pnpm adr:check` 99/99 clean; `pnpm aps:active-lint` 108 clean;
  `cargo test -p eddacraft-anvil-intercept --test daemon_dep_boundary` 7/7
  (two new guards: policy crates forbidden on the daemon + CLI positive
  control). Planning council `plan-18c47503` converged (3 negotiations, all
  consensus); Phase-4 review 2 approvals, 3 objections fixed.
- Review: council + operator ratified every gate question; ADR-098 Accepted.
- Plan changes: ADR-098 landed (renumbered 097→098 after a numbering race
  with a sibling ADR mid-flight); ADR-015 ratified Proposed→Accepted;
  POLRESET-002/003/004/010 → Ready; POLRESET-005/007 phantom validation
  commands fixed; OPA test-file deletions (PR-A) rode the gate PR.
- Checkpoints raised: none (operator answered all gates interactively).
- Discoveries: `anvil gate check policy` is still the Go OPA subprocess with
  no regorus backing (live wiring, dormant config) — replacement is OPAE-003
  PR-B. CIB-150's non-anvil-peer test is flaky under dynamic /proc exe
  aliasing — fixed via PR #3132, tracked in issue #3130.
- Next: POLRESET-010.

## Cycle — 2026-07-04 (POLRESET-010)

- Item: POLRESET-010 — Enterprise policy backlog reset
- Outcome: done (PR #3134)
- Validation: `pnpm aps:active-lint` 108 clean; `pnpm aps:index:check` clean.
- Review: docs-only reclassification under accepted ADR-098 AD-7 (fast path).
- Plan changes: reset-posture notes on ORGHIER/POLLC/COMPLY/POLFED/CPACKS
  (expansion scope)/OPAG/AGOV/ACTAX; ORGHIER priority high→low.
- Checkpoints raised: none.
- Next: POLRESET-003 (OPAE reconcile), then POLRESET-002 (POLVAL first wave).

## Cycle — 2026-07-04 (POLRESET-003)

- Item: POLRESET-003 — OPAE product-contract reset
- Outcome: done (PR #3136)
- Validation: `pnpm aps:active-lint` 108 clean; `pnpm aps:index:check` clean.
- Review: contract reconciliation under accepted ADR-098 (fast path).
- Plan changes: OPAE-001 Done (satisfied by ADR-098); ADR-098 binding notes
  on OPAE-003 (PR-B repoint), OPAE-006 (off-daemon + fail-open budget),
  OPAE-007 (ControlDecision + kill switch + EXCEPT-006 gate); OPAE module
  Draft→Proposed; In-Scope OPA stance now CI-tooling-only.
- Checkpoints raised: none.
- Next: POLRESET-002 (POLVAL first wave — implementation already staged).

## Cycle — 2026-07-04 (POLRESET-002)

- Item: POLRESET-002 — Policy pack validation foundation
- Outcome: done (PR #3138; POLVAL-001..005 all Done)
- Validation: `cargo test -p eddacraft-anvil-policy-engine` 86 lib + 1
  integration green; `cargo test -p eddacraft-anvil -- policy_validate` 7
  green; clippy/fmt/hakari clean; aps + docs indexes clean.
- Review: pre-PR council review found 2 CONFIRMED criticals (symlink pack-dir
  escape evaluated external content; Windows rooted-path join discards base
  dir), 1 MAJOR (backtick raw-string discovery false positive fails healthy
  packs), 2 minors — all fixed with regression tests before the PR opened.
- Plan changes: POLVAL retargeted off the doomed OPA-era loader.rs/library.rs
  into anvil-policy-engine/src/pack/ (ADR-098 AD-2 topology); POLVAL module
  Draft→In Progress; gate preflight deferred to OPAE-003 PR-B.
- Checkpoints raised: none.
- Next: POLRESET-004 (CPOL/IORISK retarget + context contracts); 005/006
  remain gated on EXCEPT-006 and OPAE-006/007.

## Cycle — 2026-07-04 (POLRESET-004)

- Item: POLRESET-004 — Deterministic policy context and risk vocabulary
- Outcome: done (PR #3139; CPOL-001..003 + IORISK-001..003 all Done)
- Validation: engine crate 142 lib + 1 integration green; kernel-types 198 +
  10 doctests green; cargo check --workspace green (14 kernel-types
  dependents); clippy/fmt/hakari clean; aps lint clean.
- Review: pre-PR council review found 1 CONFIRMED critical (exponential
  ReDoS in a hand-rolled glob matcher, the workspace's fifth glob impl —
  replaced with globset in the kernel-watcher dialect + linearity regression
  test), 1 major (duplicated posture→decision rule — centralised in
  src/posture.rs), 1 minor (unvalidated assertions at evaluate/assess —
  boundary validation added), 1 nit — all fixed pre-PR.
- Plan changes: CPOL/IORISK retargeted off crates/anvil-policy per ADR-098
  AD-2; IORISK.steps.md migrated to .actions.md; module headers In Progress.
- Checkpoints raised: none.
- Next: POLRESET-005..009 are gated on external prerequisites (EXCEPT-006
  verdict-aware wiring first); park handoff and report.

## Cycle — 2026-07-04 (POLRESET-005 + OPAE wave)

- Items: POLRESET-005 (Done — satisfied by EXCEPT-005/006/007, verified);
  OPAE-002/003/004/005/006 all Done (PRs #3157, #3141+#3143, #3154, #3151);
  ADR-098 PR-B/PR-C complete (gate on regorus; 2,662 lines of OPA-subprocess
  code deleted; `which` left the shipped closure).
- Validation: per-PR gates green (engine crate 179; CLI policy 98/gate 140;
  policy crate 101 post-deletion); exception evidence 65+10 green on main.
- Review: every PR pre- or post-reviewed; notable catches — install symlink
  write-escape, phantom input.config (starter pack reworked advisory-first,
  OPAE-010 filed), discovery broken-symlink fail-closed gaps, journal
  backup-before-write ordering.
- Plan changes: OPAE-010 (pack configuration surface) filed as Proposed
  intake; OPAE validation targets drift-corrected off the deleted OPA crate.
- Checkpoints raised: none (EXCEPT-006 landed by the operator in parallel).
- Next: POLRESET-006 / OPAE-007 — enforcement routing + the ADR-098 AD-3
  vocabulary unification (now fully unblocked).

## Cycle — 2026-07-04 (POLRESET-006 / OPAE-007 + AD-3)

- Items: ADR-098 AD-3 vocabulary unification (PR #3162: ControlDecision
  +Fence +Unknown, one shared posture in kernel-types, action-time veto
  projection, is_veto isError fix); OPAE-007 + POLRESET-006 routing
  (PR #3165: neutral engine-free contract in intercept-rules + MCP
  pre-write evaluation with ANVIL_POLICY_ENFORCEMENT kill switch,
  strictest-wins merge, fail-open everywhere, one wall-clock deadline
  over the whole pass).
- Validation: routing filters 6+13 green; intercept 890; cli policy 111;
  daemon_dep_boundary 7/7 (no engine crate daemonward); workspace clean.
- Review: AD-3 clean (doc drift + Off merge tests fixed); routing review
  measured the uncached per-call compile cost (~450µs/pack) sitting
  outside the budget — fixed via a whole-pass deadline; OPAE-011
  (compiled-policy cache) filed with the measurement.
- Checkpoints raised: none.
- Next: POLRESET-007 starter-pack proof (evidence + flip), then 008
  (EVALCI-005/006 report-only CI) and 009 (ATC/PATT depth).

## Cycle — 2026-07-04 (POLRESET-007 / OPAE-008)

- Items: starter-pack end-to-end proof (PR #3167) — 7-test chain over the
  real anvil-baseline pack: install+provenance, admission, gate advisory
  surfacing, pre-write projection, warnings-never-veto invariant, frozen
  eval-v1 exercisability, vocabulary lockstep guard.
- Validation: starter_policy_pack 7; policy_prewrite_routing 14;
  gate::tests 141; policy 120; workspace clean.
- Review: the proof itself caught a production bug — both surfaces dropped
  the documented `warning` rule family (recognised warn/warnings only);
  fixed via the shared policy_vocab module consumed by gate.rs AND
  mcp/policy_prewrite.rs, regression-tested on both. Also survived a
  stale-base race with PR #3165 (worktree created minutes before the merge
  reached the local fetch) — caught by cross-checking the agent's
  no-second-extractor claim against known merged state.
- Checkpoints raised: none.
- Next: POLRESET-008 (EVALCI-005/006 report-only CI), then 009 (ATC/PATT).
