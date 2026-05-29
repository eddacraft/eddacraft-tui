<!--
APS Module: v0.4.0-beta Release Follow-Ups
===========================================
Hardening items consciously deferred from the v0.4.0-beta release council
process (three rounds, six reviewers, one external Codex CLI). All items
are non-blocking for the H1 hype-builder tag but should ride the v0.5.0-beta
release or the next sweep so they don't accumulate as silent debt.

Each work item carries the reviewer that flagged it and the original
finding ID where applicable, so the rationale is traceable.

See: plans/aps-rules.md
-->

# v0.4.0-beta Release Follow-Ups

| ID    | Owner | Status |
| ----- | ----- | ------ |
| V050F | —     | In Progress |

16 work items. Items 001–010 captured from the three council rounds +
Codex CLI external review during release prep; V050F-011 added 2026-04-26
from the copilot review on PR #1081 (`scan_content` recompile / silent-
error); V050F-012 + V050F-013 + V050F-014 added from the v0.4.0-beta
tag run (workflow 24937011902) and post-tag prod deploy (run 24937001778)
— scoop PAT scope failure, winget gh CLI arg-count regression, and the
absence of a migration runner. All three v0.4.0-beta surface gaps were
closed manually (scoop bucket commit `4f3becf6`, winget PR
microsoft/winget-pkgs#365186, prod migrations applied by hand);
CI / deploy pipelines must be repaired before the next tag.
V050F-015 added 2026-04-26 from the copilot review on PR #1090 — track
the `svix → uuid` override exception so the global `uuid >=14.0.0`
security floor is restored uniformly when the upstream dependency
chain ships ESM-aware uuid. V050F-016 added 2026-04-27 after the private
`eddacraft/anvil-001` v0.4.0-beta release was published as a prerelease,
leaving v0.3.3-beta marked Latest.

## Purpose

Capture every hardening item the v0.4.0-beta release council and
follow-up release runs surfaced that was deliberately deferred so the
tag could ship. Three council rounds + one external Codex CLI review
produced ~25 findings; 18 were fixed in-flight, and the remaining
follow-ups here were judged non-blocking against the H1 gate criterion
("no first-touch papercuts that would make a wait-listed viewer close
the tab") or were discovered by the subsequent tag/deploy path. They
are tracked here for v0.5.0-beta / next sweep.

This module exists so the deferral does not silently rot. None of these
require a coordinated bundle — pick them off in any order.

## Work Items

### V050F-001: Pin cargo-dist installer by SHA256 in the release workflow

- **Surface:** `.github/workflows/release.yml:69-79`
- **Flagged by:** security-analyst (round 2 + round 3), operations-reviewer
  (round 2 + round 3), Codex CLI external (round 2)
- **Intent:** The release pipeline currently fetches
  `cargo-dist-installer.sh` from a GitHub-hosted URL and pipes to `sh`
  with no checksum verification. The in-file TODO at lines 72–74
  acknowledges this. A compromise of the installer script would
  backdoor every Anvil binary shipped through the release workflow,
  including the Homebrew tap, WinGet, and Scoop bucket.
- **Expected outcome:** Either pin the installer by a vendored
  `expected.sha256`, or replace the curl-pipe with `cargo install
  --locked cargo-dist@<pinned-version>` if cargo-dist publishes a
  matching crates.io release.
- **Confidence:** medium — depends on whether cargo-dist upstream
  publishes a SHA256 for the installer (per pragmatic-lead's R3
  comment, they may not).
- **Resolution:** Vendored a pinned SHA256 for `cargo-dist-installer.sh`
  v0.31.0 (`e79d87e418b9d2cbe992d014985457c28a5a7c553add3da4ed1047e161c928f4`)
  alongside `DIST_VERSION` in the workflow's step env. The step now
  downloads the installer to `$RUNNER_TEMP`, verifies the SHA256 via
  `sha256sum --check`, then runs it. Mismatch emits a `::error::`
  annotation with both expected and actual hashes and halts the job.
  Comment block above the step documents the bump procedure (compute
  `sha256sum` of the new version's installer, update both env vars
  together). Removed the previous `TODO(security)` since the gap is
  closed.
- **Status:** Done

### V050F-002: Correct auth_method attribution for rejected admin tokens when per-operator mode is active

- **Surface:** `apps/anvil-api/src/middleware/admin-auth.ts:181-182`
- **Flagged by:** security-analyst (round 2), pragmatic-lead (round 3
  — elevated from deferred to should-fix)
- **Intent:** When `ADMIN_PER_OPERATOR_KEYS=1` is enabled and a presented
  bearer fails the per-operator lookup AND fails the shared-key match,
  the audit row currently stamps `auth_method: 'shared'` (hardcoded),
  mis-attributing the attempt to the shared-key surface. Real attacker
  credential-stuffing campaigns against per-operator keys would be
  invisible inside shared-key noise.
- **Expected outcome:** Stamp `auth_method: 'per_operator'` on
  rejected_unknown when `perOperatorActive` is true. One-line fix.
- **Confidence:** high
- **Status:** Done

### V050F-003: Cascade access_tokens revoke when refresh family is revoked on theft detection

- **Surface:** `apps/anvil-api/src/routes/auth-session.ts:53,80`
- **Flagged by:** security-analyst (round 2)
- **Intent:** `revokeRefreshTokenFamily` revokes the refresh-token
  family on theft detection but leaves the corresponding
  `access_tokens` rows valid. A leaked JWT licence remains valid for up
  to its 7-day TTL after the user is locked out of refresh.
- **Expected outcome:** On family-theft detection, also revoke the
  corresponding `access_tokens` rows for the user (or at minimum
  bound the leaked JWT's lifetime by tracking refresh-revoked users
  and refusing to verify their tokens). Pairs with V050F-008 below.
- **Confidence:** medium
- **Status:** Done

### V050F-004: Flag-gate `/admin/approve` granted scopes via `resolveApiScope`

- **Surface:** `apps/anvil-api/src/routes/admin.ts:295-303` (the new
  scope-preservation block landed in commit `f9961b28`)
- **Flagged by:** security-analyst (round 3, S3-001)
- **Intent:** `/admin/invite` validates each scope through
  `resolveApiScope()` so an operator can disable a scope by flipping
  its `api.scope.*` feature flag. `/admin/approve` now preserves prior
  graded scopes from `findActiveScopesForUser` and unions with
  `DEFAULT_APPROVAL_SCOPES`, but never re-validates that union. A
  previously-granted `preview`/`internal` scope that has since been
  disabled by flag will be re-issued on every approve, bypassing the
  kill-switch the FLAGM-005 contract promises operators.
- **Expected outcome:** Filter the granted-scopes union through
  `resolveApiScope` and audit any dropped scopes so the operator sees
  the kill-switch took effect.
- **Confidence:** high
- **Status:** Done

### V050F-005: Regression tests for graded-scope preservation across `/admin/approve` and `/auth/otp`

- **Surface:**
  `apps/anvil-api/src/__tests__/admin.test.ts:556-668`,
  `apps/anvil-api/src/__tests__/auth-otp.test.ts:354`
- **Flagged by:** council-reviewer (round 3), security-analyst (round
  3, S3-005), Codex CLI (round 3, finding 2)
- **Intent:** `auth-session.test.ts:158-169` asserts `/session/refresh`
  preserves a `['preview', 'beta']` grant. The OTP and approve fixes
  in commits `eae47b3d` / `f9961b28` are correct on inspection but
  have no equivalent regression test — a future refactor could
  silently re-introduce FLAGM-005 on either path.
- **Expected outcome:** Add a happy-path test in each suite that mocks
  `findActiveScopesForUser` to return `['preview', 'beta']` and
  asserts the issued JWT carries that union (decodeJwt the licence
  claim, same shape as the auth-session regression test).
- **Confidence:** high
- **Status:** Done

### V050F-006: Cache compiled allowlist regexes in `is_file_allowlisted`

- **Surface:** `crates/anvil-checks/src/antipattern/scanner.rs:176-184`
- **Flagged by:** kernel-maintainer (round 2)
- **Intent:** `is_file_allowlisted` calls `glob_to_regex` per pattern
  per file, recompiling a `Regex` on every call. `PREPARED_PATTERNS`
  caches the primary scan regex but not the allowlist regexes. With N
  patterns each carrying M allowlist entries, a scan of K files pays
  N×M regex compilations.
- **Expected outcome:** Compile allowlist regexes once in
  `prepare_pattern` and store alongside `primary_regex`. Re-bench
  `antipattern_scan/parallel_mixed_corpus` after the change to
  confirm the speedup.
- **Confidence:** high
- **Status:** Complete — branch `fix/v050f-scanner-hotpath`. Added
  `AllowlistGlob` (compiled `Regex` plus the precomputed
  `is_path_glob: bool` match-base flag — original glob strings stay
  in `pattern.allowlist`) and a `compile_allowlist` helper;
  `prepare_pattern` now compiles the allowlist once and stores the
  runtime artefacts on `PreparedPattern.allowlist_regexes`. Hot-path
  call site uses the new `is_file_allowlisted_compiled` which
  preserves match-base semantics without re-parsing or
  `pattern.contains('/')` per match.

### V050F-007: Initialise rayon pool eagerly in the binary entry point

- **Surface:** `crates/anvil-kernel/src/watch.rs:160-170` (`POOL_INIT`),
  `crates/anvil-checks/src/antipattern/scanner.rs:630-635`
  (`scan_artifacts`)
- **Flagged by:** kernel-maintainer (round 2 + round 3)
- **Intent:** `POOL_INIT` builds the rayon global pool capped at half
  cores for VS Code coexistence. `scan_artifacts` (anvil-checks) also
  uses rayon's default global pool but never calls `build_global`. If
  `scan_artifacts` is called before any `run_watch` invocation (e.g.
  from `anvil check`), rayon initialises the global pool to
  `num_cpus` threads, and the subsequent `POOL_INIT.call_once` is a
  no-op. The half-cores cap is silently absent.
- **Expected outcome:** Initialise the rayon pool in the binary entry
  point (or NAPI init hook) before either consumer can reach rayon.
- **Confidence:** high
- **Status:** Complete — branch `fix/v050f-rayon-init`. Centralised the
  half-cores cap in a new dedicated micro-crate
  `anvil-rayon-init::init_global` (idempotent via `std::sync::Once`);
  replaced the duplicated `POOL_INIT.call_once` blocks in
  `kernel/src/watch.rs` and `kernel/src/embedded.rs` with delegating
  calls. The CLI binary entry point (`crates/anvil-cli/src/main.rs`)
  calls `init_global` as the first statement in `main()`, before any
  subcommand can dispatch to a rayon-using path. The NAPI binding
  (`crates/anvil-checks-napi`) calls `init_global` at the top of
  `scan_artifact_json` so the editor host that loads the binding
  inherits the cap before any `scan_artifact_rust` `par_iter` runs.
  The helper lives in its own crate (rather than `anvil-kernel`) to
  avoid linking the full kernel graph into the NAPI cdylib for what
  is genuinely four lines of pool init — council finding,
  kernel-maintainer.

### V050F-008: Bench baselines collected on a CI-class machine

- **Surface:** `crates/anvil-bench/README.md:46-89`
- **Flagged by:** kernel-maintainer (round 2 + round 3)
- **Intent:** Current baselines (antipattern_scan ≈ 11.2 ms, stress
  scenarios in the post-RUSTNX-008 entry) were collected on a local
  dev machine. The README correctly notes that GitHub-hosted runners
  have 2 cores and produce materially lower throughput, and that the
  2× regression guard must be evaluated against a same-class
  baseline. There is no CI-collected baseline — only a dev-box
  number. Without a CI-class anchor the regression guard is not
  enforceable.
- **Expected outcome:** Add a CI workflow step that runs the
  antipattern_scan and stress benches on a GitHub-hosted runner and
  records the baseline numbers in the bench README (or a separate
  `baseline-ci.json`). The 2× regression check then has a
  reproducible anchor.
- **Confidence:** medium — needs a stable runner cadence
- **Status:** Todo

### V050F-009: Add `release/*` to the Rust CI push filter

- **Surface:** `.github/workflows/rust.yml:5`
- **Flagged by:** operations-reviewer (round 2)
- **Intent:** `cargo hakari verify` (rust.yml:321) and
  `cargo deny check` (rust.yml:339-340) run on push to `main`, `dev`,
  and `rust-*` — but not on `release/*` branches. The release branch
  never gets these checks except via a PR to one of those targets,
  and the release preflight did not run them either. A workspace-hack
  drift or licence violation introduced on a release branch would
  ship undetected.
- **Expected outcome:** Add `release/*` to the rust.yml push filter,
  AND add `cargo hakari verify` + `cargo deny check` to
  the release preflight path so local preflight catches what CI catches.
- **Confidence:** high
- **Resolution:** Added `release/*` to the Rust workflow push branch filter.
  Added `cargo hakari verify` and `cargo deny check` to the local release
  preflight script before the existing clippy/test checks; RELORCH later
  replaced the legacy single-file runner with `scripts/release/preflight.sh`.
- **Status:** Done

### V050F-011: Refactor `scan_content` to surface custom-pattern compile errors

- **Surface:** `crates/anvil-checks/src/secret/scanner.rs:17`
- **Flagged by:** copilot reviewer (PR #1081 review, 2026-04-26)
- **Intent:** `scan_content` recompiles `config.custom_patterns` on every
  call (per file) and discards the compile diagnostics
  (`_custom_errors`). The gate path already collects pattern errors via
  `run_secret_check` (gate.rs:385), so end users running `anvil gate`
  are not affected. The silent-loss risk is on third-party callers
  using `scan_content` directly — they get the redundant compilation
  cost AND no signal when a custom pattern fails to compile.
- **Expected outcome:** Either (a) compile custom patterns once per
  check run and pass the compiled slice into `scan_content`, or
  (b) change `scan_content` to return both findings and pattern errors
  so the error-reporting contract is enforced by the function
  signature. Pairs naturally with V050F-006 (allowlist regex caching)
  since both are in the secret/scanner hot path.
- **Confidence:** medium — signature change touches every direct
  caller of `scan_content`
- **Status:** Complete — branch `fix/v050f-scanner-hotpath`. Both
  options shipped: (a) new hot-path primitive
  `scan_content_with_compiled_patterns` takes a pre-compiled
  `&[CompiledPattern]` slice and `run_secret_check` now compiles
  custom patterns once and threads the slice through rayon workers;
  (b) new `scan_content_with_pattern_errors_and_stats` returns
  `(Vec<SecretFinding>, ScanStats, Vec<String>)` so third-party
  callers can route the errors. Legacy `scan_content_with_*`
  wrappers preserve their signatures but emit `tracing::warn!` on
  any dropped error so the silent-loss path is observable.

### V050F-012: Fix CI scoop publisher PAT scope

- **Surface:** `.github/workflows/release.yml` `scoop` job; `ANVIL_RELEASES_TOKEN` PAT
- **Flagged by:** v0.4.0-beta release run (workflow run 24937011902)
- **Intent:** The scoop publisher job failed with
  `HTTP 403 — Resource not accessible by personal access token` when
  pushing the manifest to `eddacraft/scoop-bucket`. The bucket was updated
  manually for v0.4.0-beta via `gh api PUT contents/bucket/anvil.json`
  from a developer machine (commit
  `eddacraft/scoop-bucket@4f3becf6`), but the CI path needs to work
  for the next tag.
- **Expected outcome:** Either rotate `ANVIL_RELEASES_TOKEN` to a PAT
  with `contents:write` on `eddacraft/scoop-bucket`, or migrate the
  scoop push to a GitHub App / fine-grained token that scopes correctly.
  Verify with a dry-run on a test tag before the next real release.
- **Confidence:** high
- **Resolution:** Scoop job rewritten in PR #1106 with a token
  pre-flight that reads `repos/${BUCKET}` before the PUT and bails with
  a clear `::error::` annotation when the token cannot reach the
  bucket. The PUT itself captures stderr and emits a distinct error
  for `HTTP 403` (write denied, matches v0.4.0-beta failure shape)
  vs other failures. Token-scope runbook lives at
  [`docs/runbooks/release-token-scope.md`](../../docs/runbooks/release-token-scope.md);
  it leads with the in-place edit path (the actual fix) and keeps
  full mint+install+revoke as a fallback for true rotation cases.
  Operator added `contents:write` on `eddacraft/scoop-bucket` to the
  existing PAT 2026-04-26 — no rotation, no Secret value change. Next
  release tag will exercise the full path.
- **Status:** Done

### V050F-013: Fix CI winget publisher `gh` arg-count regression

- **Surface:** `.github/workflows/release.yml` `winget` job (lines ~570–660,
  the manifest-generation + fork + PR step)
- **Flagged by:** v0.4.0-beta release run (workflow run 24937011902)
- **Intent:** The winget publisher job failed with
  `gh` CLI `accepts 1 arg(s), received 2` mid-script after the manifest
  YAMLs generated. First v0.4.0-beta tag with the new ARM64
  `Installers` entry; possibly a `gh repo fork` / `gh pr create`
  arg-shape regression triggered by the runner's `gh` version, or a
  shell-quoting issue on one of the substituted strings. The PR was
  created manually via API for v0.4.0-beta
  (microsoft/winget-pkgs#365186). The manual recovery used
  `EddaCraft/anvil` casing initially and was rejected by the WinGet
  validator — fix to lowercase pushed and accepted; record a defensive
  assertion that the workflow's URL substitution stays lowercase too.
- **Expected outcome:** Reproduce the failure locally with the same
  `gh` version the runner uses (check the runner image manifest for
  the `gh` version pin), repair the offending command, add a defensive
  test (script-level `set -x` or a smoke step that runs the same
  fork+commit+pr flow against a stub repo), assert URL casing on the
  generated manifest before push.
- **Confidence:** medium — root cause not yet diagnosed
- **Resolution:** Defensive workflow rewrite landed in PR #1098.
  Replaces the `gh repo fork ... --clone=false 2>/dev/null || true`
  pattern (which swallowed the cobra diagnostic) with an explicit
  fork-existence check that branches on `HTTP 404` (create the fork)
  vs any other stderr (auth/rate-limit/network — halt loudly).
  Switches `$SHA_ARG` from a string to a bash array (`SHA_ARGS=()` /
  `"${SHA_ARGS[@]}"`) to avoid word-split / globbing hazards when
  conditionally building flag strings. Adds `gh --version` log line
  for future repro and an explicit lowercase assertion on `REPO`
  before any URL substitution. Root cause stays unproven without a
  matching-version local repro; the defensive form is sufficient
  to unblock the next tag.
- **Status:** Done

### V050F-014: Wire a database-migration runner into the deploy pipeline

- **Surface:** `apps/anvil-api/src/db/migrations/`,
  `apps/anvil-api/src/db/migrate.ts`,
  `apps/anvil-api/scripts/migrate.mjs`,
  `.github/workflows/infra.yml` (`up` job — Pulumi Up actually lives
  here, not in `release.yml`),
  `docs/runbooks/db-migrations.md`,
  `docs/runbooks/post-deploy-smoke-check.md`
- **Flagged by:** v0.4.0-beta post-tag prod deploy (workflow run
  24937001778, Pulumi Up failure on missing `admin_keys` table); also
  flagged earlier by operations-reviewer round 3 (consider C-1: "no
  migration runner is documented or automated").
- **Intent:** Migrations are SQL files in `apps/anvil-api/src/db/migrations/`,
  but no CI step applies them to staging or production. Each release
  that adds a migration silently breaks the next deploy until an operator
  runs `psql` by hand. v0.4.0-beta added migrations 007–010; the prod
  Pulumi deploy failed with `relation "admin_keys" does not exist` and
  was unblocked only by manually applying 007/008/009/010 against the
  Neon prod database.
- **Expected outcome:** A migration runner that:
  1. Tracks applied migrations in a `_migrations` table (filename + sha
     of the file at apply-time).
  2. Discovers all `.sql` files in `apps/anvil-api/src/db/migrations/`
     in lexical order, applies any not yet recorded in `_migrations`,
     in a single transaction per file.
  3. Refuses to apply a migration whose recorded sha differs from the
     on-disk sha (catches retroactive edits to applied migrations).
  4. Runs as a workflow step in `infra.yml`'s `up` job before the
     Pulumi Up step. Migrations apply BEFORE infra so Pulumi can rely
     on the schema being current. (The original spec said `release.yml`
     between `host` and Pulumi Up; Pulumi Up actually lives in
     `infra.yml` not `release.yml` — corrected during implementation.)
  5. Has a manual operator runbook entry for ad-hoc apply (recovery,
     staging tests).
- **Confidence:** medium — needs a small design pass on whether to
  reuse Drizzle Kit (already in the workspace), `node-pg-migrate`, or
  ship a minimal first-party runner. Per-migration transaction +
  `_migrations` tracking are the non-negotiable parts.
- **Action plan:** [`plans/execution/V050F-014.steps.md`](../execution/V050F-014.steps.md)
- **Resolution:** First-party runner shipped in PR #1099. Lib at
  `apps/anvil-api/src/db/migrate.ts`, CLI at
  `apps/anvil-api/scripts/migrate.mjs`, runbook at
  `docs/runbooks/db-migrations.md`. CI wiring landed in
  `.github/workflows/infra.yml` `up` job — DATABASE_URL fetched from
  Key Vault, runner invoked between Azure Login and Pulumi Up so the
  schema is current before infra apply. Path filter expanded so
  migration-only changes also trigger Infrastructure. Prod
  `_migrations` backfilled 2026-04-26 — 10/10 rows recorded, runner
  dry-run reports `0 pending`.
- **Status:** Done

### V050F-015: Remove `svix>uuid` override exception once dependency chain ships ESM-aware uuid

- **Surface:** `package.json` (`overrides.svix.uuid` and
  `pnpm.overrides["svix>uuid"]`), introduced in PR #1090
- **Flagged by:** copilot reviewer on PR #1090, 2026-04-26
- **Intent:** PR #1090 reintroduces `uuid@10` into the svix subtree to
  unblock prod (svix is CJS, uuid v14 is ESM-only, ERR_REQUIRE_ESM
  crashed every cold start). The global `uuid: >=14.0.0` floor that
  closed advisory `GHSA-w5hq-g745-h8pq` (added in `a0fe63de`) is
  preserved for every other consumer; svix is the only exception.
  The exception should not become permanent — it exists because
  `resend@6.x → svix@1.90.0` is CJS and uuid v14 dropped CJS support.
  When the chain ships an ESM-aware uuid (either svix bumps uuid to a
  dual-mode version, or resend ships a major that drops svix, or
  uuid republishes a CJS-compatible v14+), this override should come
  out so the security floor applies uniformly.
- **Expected outcome:** Override exception removed; `pnpm-lock.yaml`
  has no `uuid@<14` entry; `apps/anvil-api/scripts/check-runtime-cjs.cjs`
  still passes (svix loads under CJS).
- **How to detect readiness:** Watch `resend` and `svix` releases for
  ESM/dual-mode announcements; alternately, run a periodic dry-run that
  removes the override and runs the postbuild smoke check — if it
  passes, the exception is no longer needed.
- **Confidence:** medium — depends on upstream cadence
- **Status:** Todo

### V050F-010: Document `WAITLIST_PAUSED` kill-switch in the operator runbook

- **Surface:** `docs/runbooks/waitlist-email-operations.md`,
  `docs/runbooks/post-deploy-smoke-check.md`
- **Flagged by:** operations-reviewer (round 2 + round 3)
- **Intent:** `apps/anvil-api/src/routes/waitlist.ts:19-21` honours a
  `WAITLIST_PAUSED` env var and returns 503 when set. The only
  documentation lives in archived plan files (`plans/archive/…`). An
  on-call operator responding to a waitlist write storm during the
  H1 launch spike has no documented path to throttle it.
- **Expected outcome:** Add a section to
  `docs/runbooks/waitlist-email-operations.md` covering when to set
  it, how to set it in Vercel (env-var toggle + redeploy), what the
  caller sees, and when to unset it. Cross-reference from
  `post-deploy-smoke-check.md`.
- **Resolution:** Added the operator pause path to the waitlist email runbook:
  set `WAITLIST_PAUSED=true` in the Vercel `anvil-api` project, redeploy,
  expect HTTP `503`, verify the pause, then unset and redeploy when the
  incident or maintenance window ends. The post-deploy smoke runbook now links
  to those steps when the waitlist submission smoke check returns `503`.
- **Confidence:** high
- **Status:** Done

### V050F-016: Promote private beta releases to Latest

- **Surface:** `.github/workflows/release.yml`,
  `docs/guides/release-doc-checklist.md`
- **Intent:** Keep the private `eddacraft/anvil-001` release record aligned
  with the public distribution release when publishing beta tags.
- **Expected outcome:** Beta tags published by the release workflow are not
  left as prereleases on `eddacraft/anvil-001`, so GitHub marks the newest tag
  as Latest.
- **Validation:** `gh release list --repo eddacraft/anvil-001 --limit 3`
- **Source:** Reported 2026-04-27 after `v0.4.0-beta` was not shown as Latest.
- **Confidence:** high
- **Status:** Done

## Cross-cutting notes

- V050F-002, V050F-003, V050F-004 share the same auth surface and
  could land as a single PR if you want to pay the test-rebuild once.
- V050F-006 + V050F-007 are kernel hot-path hardening; pair if a
  bench rerun is on the calendar anyway.
- V050F-009 + V050F-010 are pure ops; pair with the `release/*` CI
  filter since a documentation-only branch wouldn't otherwise need a
  rust.yml run.

## Tracking

The original council artefacts that produced these items live in
`plans/reviews/` (round 1, round 2, round 3 transcripts) and the
release-tracking issue `EddaCraft/anvil-001#1080`.
