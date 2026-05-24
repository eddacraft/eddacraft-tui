<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Email Broadcast Surface

| Scope | Owner | Priority | Status |
| ----- | ----- | -------- | ------ |
| EMAIL | —     | Medium   | Done   |

**Last reviewed:** 2026-05-24

## Purpose

Unify the operator-facing email surface in `apps/anvil-api`. Today every
mail-to-many path is bespoke: `/admin/send-migration` (`admin.ts:638`) has the
disciplined snapshot/preview/drift contract, but `/admin/invite` and
`/admin/approve` fire `sendBetaInvite` directly, and the recently-landed
`release-announcement` template has no sender or endpoint at all. The result
is that adding the next broadcast — release announcements for v0.7.0-beta and
beyond — would either grow another bespoke endpoint or quietly ride on
`/admin/send-migration`'s mechanics without inheriting its safety rails.

This module collapses all mail-to-many flows onto a single
`POST /admin/broadcast` endpoint backed by a named-cohort audience taxonomy
and a template registry. Per-recipient transactional mail (OTP, beta invite,
waitlist confirmation) stays on its existing path — the registry's
`kind: 'transactional'` flag is the guardrail that keeps secret-minting flows
off the broadcast surface.

## In Scope

- Audience resolver registry with six named cohorts queried against the
  existing `beta_users`, `waitlist`, `access_tokens`, and `refresh_tokens`
  tables. Hard exclusions for suspended/banned/suppression-listed addresses
  applied uniformly across every resolver.
- Generalisation of `send_migration_snapshots` into
  `send_broadcast_snapshots` carrying `template`, `template_props`,
  `audience_key`, and `audience_params`.
- Template registry in `apps/anvil-api/src/lib/email-registry.ts`
  discriminating `broadcast` vs. `transactional` kinds.
- `sendReleaseAnnouncement` helper in `apps/anvil-api/src/lib/email.ts`
  mirroring the shape of `sendWaitlistMigration`.
- `POST /admin/broadcast` endpoint preserving the existing
  `send-migration` two-step (`dryRun` → `previewToken` →
  consume + cohort drift) wire contract and error code taxonomy.
- `/admin/send-migration` reduced to a thin back-compat shim forwarding to
  the broadcast handler so the admin CLI keeps working unmodified.

## Out of Scope

- `/admin/send-test` single-recipient render-and-mail (deferred to Phase 2).
- Resend webhook + `suppressions` table (deferred to Phase 2 — referenced by
  the hard-exclusion plumbing here, populated later).
- Operator-issued `/admin/invite/resend` and `/admin/otp/resend` recovery
  endpoints (Phase 3).
- Resend Contacts audience reconciliation (Phase 3).
- Free-form SQL or CSV-upload audiences. The taxonomy is a closed set; new
  audiences land as named entries in the registry, not via runtime queries.
- Combined `waitlist ∪ beta` audiences. Operators send to the right cohort
  with the right template — merged audiences hide which relationship the
  message is addressing.

## Interfaces

**Depends on:** ADMINCLI (complete), ADMINCLIH (complete — the
`send-migration` snapshot/preview/drift contract this module generalises).

**Exposes:**

- `POST /admin/broadcast` — uniform endpoint for any mail-to-many template.
- `lib/audiences.ts` — `AudienceKey`, `resolveAudience()`, and the six
  resolver functions.
- `lib/email-registry.ts` — `EMAIL_REGISTRY` keyed by template name with
  `kind`, `propsSchema`, and `sender` per entry.
- Generalised `send_broadcast_snapshots` table accessible through the
  existing `insertSendMigrationSnapshot` / `findSendMigrationSnapshot` /
  `consumeSendMigrationSnapshot` query surface (signatures widen to carry
  the additional columns; callers update in the same change).

## Design Decisions

Confirmed during planning on 2026-05-24:

1. **Activity window.** `beta:active-recent` is `refresh_tokens.created_at >
   now() - 30d` joined to `beta_users` rows where `status='active'` and
   `revoked_at IS NULL`.
2. **Already-invited rows in `waitlist:source`.** Excluded by default —
   resolver joins against `beta_users` and filters them out. This narrows
   the cohort vs. what `/admin/send-migration` selects today
   (`queries.ts:758`) and must be called out in the back-compat shim's PR
   prose. A future `waitlist:source-include-invited` variant can be added
   if a legitimate re-target case emerges; not in v1.
3. **Complementary idle definition.** `beta:active-idle` is strictly the
   set-complement of `beta:active-recent` within `beta:active`. No gap
   bucket — every active beta user is either recent or idle, never neither.

## Audience Taxonomy

Six named audiences keyed by `<table>:<filter>`. Every resolver returns the
same row shape: `{ email, name, user_id }` (where `user_id` is null for
waitlist-only audiences). Template-specific data — release version,
activation URL, theme — flows in via the operator-supplied `templateProps`
on the broadcast call, never out of the audience layer.

| Key                            | Backing query (intent)                                                 | Use case                                       |
| ------------------------------ | ---------------------------------------------------------------------- | ---------------------------------------------- |
| `beta:active`                  | `beta_users` where `status='active'`                                   | Default "active users" — release notes         |
| `beta:active-recent`           | `beta:active` ∩ refresh token issued within 30d                        | High-signal release nudges                     |
| `beta:active-idle`             | `beta:active` ∖ `beta:active-recent`                                   | Re-engagement of invited-then-gone-dark users  |
| `waitlist:pending`             | `waitlist` with no matching `beta_users` row                           | Capacity / ETA updates to non-invited signups  |
| `waitlist:source`              | `waitlist.source = $params.source` and no matching `beta_users` row    | Source-specific migration / follow-up cohorts  |
| `waitlist:approved-no-token`   | Has `beta_users` row but no active `access_tokens`                     | "Invited, never activated" re-invite nudges    |

**Hard exclusions baked into every resolver:**

- `beta_users.status NOT IN ('suspended', 'banned')` for any audience
  touching the users table.
- Future `suppressions` table join applied uniformly once Phase 2 lands —
  resolver scaffolding leaves a `LEFT JOIN suppressions` hook so the table
  can be wired in without rewriting every resolver.
- `LOWER(email)` distinct as a belt-and-braces dedupe even though `citext`
  should already handle case folding.

## `POST /admin/broadcast` Contract

**Request body:**

```ts
{
  template: 'release-announcement' | 'waitlist-migration',
  audience: AudienceKey,
  audienceParams?: Record<string, string>,   // e.g. { source: 'import' }
  templateProps: Record<string, unknown>,    // validated against template's propsSchema
  limit: number,                             // default 1000, max 5000
  dryRun: boolean,
  previewToken?: string,                     // required when dryRun=false
}
```

**Dry-run flow:**

1. Validate `templateProps` against `EMAIL_REGISTRY[template].propsSchema`
   before resolving the audience. Reject a bad render up front rather than
   deferring to send-time.
2. Resolve audience → rows applying the hard exclusions.
3. Snapshot `{ token, template, template_props, audience_key,
   audience_params, recipients, created_by_actor, expires_at = now() +
   10min }` into `send_broadcast_snapshots`.
4. Return `{ dryRun: true, template, audience, count, recipients,
   templateProps, previewToken, expiresAt }`.

**Real-send flow:**

1. Require `previewToken`. Same `preview_token_*` error codes as
   `send-migration` today (`admin.ts:670`).
2. Atomically consume the snapshot row.
3. Snapshot is the source of truth for `template`, `templateProps`,
   `audience_key`, and `audience_params`. Request-body values for those
   fields are ignored — preserves the same anti-bait-and-switch invariant
   that `send-migration` enforces on `source`.
4. Re-resolve audience using snapshot params with
   `freshLimit = max(snapshot.recipients.length, 1)`. Compute drift on
   `email` equality.
5. On drift → `409 cohort_drift` with `{ added, removed }`. Identical
   shape to the existing endpoint.
6. On clean → iterate snapshot rows, call
   `EMAIL_REGISTRY[template].sender(row, templateProps)`. Per-row data
   (`name`) comes from the snapshot, not re-fetched.
7. Audit-log `broadcast.email.sent` with `{ template, audience,
   audienceParams, sent, failed, previewToken }`.

**Error code taxonomy:**

| HTTP | `code`                            | When                                                 |
| ---- | --------------------------------- | ---------------------------------------------------- |
| 400  | `preview_token_required`          | Real-send with no token                              |
| 400  | `template_props_invalid`          | `templateProps` fails template schema                |
| 400  | `audience_params_missing`         | Resolver needs a param the request didn't supply     |
| 400  | `template_kind_not_broadcastable` | Registry says `kind: 'transactional'`                |
| 410  | `preview_token_missing`           | Token unknown to actor (also covers cross-actor)     |
| 410  | `preview_token_expired`           | TTL passed                                           |
| 410  | `preview_token_consumed`          | Already used                                         |
| 409  | `cohort_drift`                    | Re-resolved recipient set differs                    |

**Rate limit:** `adminRateLimit({ windowMs: 60 * 60 * 1000, max: 5, scope:
'broadcast' })` — same envelope as `send-migration` today, applied to the
broadcast endpoint as a whole so alternating templates can't dodge the cap.

## DB Migration Strategy

Migration `013-broadcast-snapshots.sql`:

1. `ALTER TABLE send_migration_snapshots RENAME TO send_broadcast_snapshots`.
2. Add `template`, `template_props`, `audience_key`, `audience_params`
   columns with temporary defaults sufficient for the backfill.
3. Backfill `audience_params` from the existing `source` column, then drop
   `source`.
4. Drop the temporary defaults so new rows must always supply explicit
   values.

Postgres renames the table's indexes automatically; no separate index
migration. The reap loop in `queries.ts:840` updates to the new table name
in the same change.

---

## Phase 1 — Audience and Registry Foundation

### EMAIL-001 — Audience resolver registry

- **Status:** Done
- **Priority:** Medium
- **Confidence:** High
- **Intent:** Introduce `lib/broadcast-audiences.ts` exposing six named
  resolvers returning the canonical `AudienceRow` shape, with hard
  exclusions applied uniformly. No callers yet — the registry exists so
  EMAIL-005 can depend on it. Named `broadcast-audiences.ts` to avoid
  collision with the existing `audience.ts` (Resend Contacts mirror).
- **Expected Outcome:** `resolveAudience(sql, key, params)` returns
  `AudienceRow[]` for any of the six keys. Suspended and banned users
  excluded everywhere. `waitlist:source` excludes rows already in
  `beta_users`.
- **Validation:** `pnpm --filter @eddacraft/anvil-api test broadcast-audiences`
- **Files:** `apps/anvil-api/src/lib/broadcast-audiences.ts` (new),
  `apps/anvil-api/src/lib/__tests__/broadcast-audiences.test.ts` (new).
- **Notes:** Landed 2026-05-24. Six resolvers + `AUDIENCE_KEYS`,
  `RECENT_ACTIVITY_DAYS = 30`, and `resolveAudience()` dispatcher.
  Hard exclusion implemented as `status = 'active'` on every
  `beta_users`-touching resolver (tighter than the spec's
  `NOT IN ('suspended','banned')` — also excludes `pending`). 33 tests
  green; full anvil-api suite has pre-existing failures unrelated to
  this change (missing workspace-dep builds).
- **changeType:** internal
- **releaseIntent:** never
- **releaseScope:** none
- **releaseNote:** audience `none`

### EMAIL-002 — Generalise snapshot table to broadcasts

- **Status:** Done
- **Priority:** Medium
- **Confidence:** High
- **Intent:** Rename `send_migration_snapshots` to
  `send_broadcast_snapshots` and add `template`, `template_props`,
  `audience_key`, `audience_params`. Update the query surface
  (`insert*`, `find*`, `consume*BroadcastSnapshot`) to read and write
  the new columns while keeping the existing `/admin/send-migration`
  call site working unchanged.
- **Expected Outcome:** Migration 013 applies cleanly against a database
  populated by 006. Existing `/admin/send-migration` integration tests
  still pass against the renamed table without behavioural change.
- **Validation:** `pnpm exec vitest --run src/__tests__/admin.test.ts`
- **Files:** `apps/anvil-api/src/db/migrations/013-broadcast-snapshots.sql`
  (new), `apps/anvil-api/src/db/queries.ts`,
  `apps/anvil-api/src/db/schema.sql` (broadcast-snapshots table +
  expires_at index now mirrored; the original 006 table was never
  mirrored at all, so this also closes a pre-existing fresh-install
  gap), `apps/anvil-api/src/routes/admin.ts` (uses the new function
  names + supplies waitlist-migration defaults inline pending the
  EMAIL-006 shim), `apps/anvil-api/src/__tests__/admin.test.ts`
  (mocks updated to the new function names + snapshot fixture
  widened).
- **Notes:** Landed 2026-05-24. Renamed `SendMigrationSnapshot` →
  `BroadcastSnapshot`, `insertSendMigrationSnapshot` →
  `insertBroadcastSnapshot` (same for find/consume). The
  `send-migration` real-send path now reads `consumed.audience_params.source`
  in place of `consumed.source`. admin.test.ts 85/85 green; migrate
  runner picks up 013 via filename-ordered readdir, no manifest edit
  required.
- **changeType:** internal
- **releaseIntent:** never
- **releaseScope:** none
- **releaseNote:** audience `none`
- **Risks:** Renames touch every query in `queries.ts:776`–`915`. The
  parser-level reap loop (`queries.ts:840`) must update in the same PR or
  the loop targets a missing table.

### EMAIL-003 — Email template registry with kind discrimination

- **Status:** Done
- **Priority:** Medium
- **Confidence:** High
- **Intent:** Introduce `lib/email-registry.ts` mapping template names
  (`release-announcement`, `waitlist-migration`, `beta-invite`,
  `otp-code`, `waitlist-confirmation`) to
  `{ kind, propsSchema, sender }`. Existing transactional senders are
  registered with `kind: 'transactional'`; broadcast senders with
  `kind: 'broadcast'`. The registry is the single source of truth for
  the broadcast handler's guardrail.
- **Expected Outcome:** Importers can resolve a template's
  `propsSchema` and `sender` by key without `switch` statements scattered
  across routes. Template `kind` is exhaustively enumerable in tests.
- **Validation:** `pnpm exec vitest --run src/lib/__tests__/email-registry.test.ts`
- **Files:** `apps/anvil-api/src/lib/email-registry.ts` (new),
  `apps/anvil-api/src/lib/__tests__/email-registry.test.ts` (new).
- **Notes:** Landed 2026-05-24. Discriminated union: broadcast entries
  carry `{ kind, propsSchema, sender }`, transactional entries carry
  `{ kind, propsSchema }` (no sender — `/admin/send-test` in Phase 3
  will need its own dispatch since the existing transactional senders
  have heterogeneous positional signatures). Strict schemas reject
  `email` / `unsubscribeMailto` from operator-supplied props since
  those are recipient/send-time concerns. `release-announcement` sender
  is a placeholder that throws until EMAIL-004 wires the real
  `sendReleaseAnnouncement`. 26/26 tests green; combined run with
  EMAIL-001/-002 still 144/144.
- **changeType:** internal
- **releaseIntent:** never
- **releaseScope:** none
- **releaseNote:** audience `none`

### EMAIL-004 — `sendReleaseAnnouncement` helper

- **Status:** Done
- **Priority:** Medium
- **Confidence:** High
- **Intent:** Add `sendReleaseAnnouncement(email, props)` to
  `lib/email.ts` mirroring the shape of `sendWaitlistMigration`. Returns
  `EmailDeliveryResult`, builds the `List-Unsubscribe` mailto header,
  computes per-recipient `unsubscribeMailto`, tags the message with
  `category: release-announcement`.
- **Expected Outcome:** Calling the helper renders
  `ReleaseAnnouncement` with operator-supplied props and ships it via
  Resend, with the same delivery semantics and error shape as the other
  senders.
- **Validation:** `pnpm exec vitest --run src/lib/__tests__/email.test.ts`
- **Files:** `apps/anvil-api/src/lib/email.ts`,
  `apps/anvil-api/src/lib/email-registry.ts` (placeholder sender swapped
  for the real call), `apps/anvil-api/src/lib/__tests__/email.test.ts`
  (new sendReleaseAnnouncement describe block, mocks widened to
  include ReleaseAnnouncement + V070_DEFAULTS),
  `apps/anvil-api/src/lib/__tests__/email-registry.test.ts` (placeholder
  throw test replaced with forwarding assertion),
  `packages/transactional/emails/release-announcement.tsx` (V070_DEFAULTS
  promoted from a private const to an export), `packages/transactional/emails/index.ts`
  (re-export V070_DEFAULTS).
- **Notes:** Landed 2026-05-24. Subject derivation mirrors the
  template's V070-defaults-when-both-missing rule so an operator
  sending the v0.7.0 broadcast with empty templateProps gets a
  matching subject. Spread order in the sender puts operator props
  first then overrides email + unsubscribeMailto — belt-and-braces
  with the email-registry strict schema. The registry sender uses a
  documented `as ReleaseAnnouncementSendProps` cast because the
  discriminated union widens `z.infer<ZodTypeAny>` to `unknown`;
  propsSchema.parse() at the /admin/broadcast boundary guarantees the
  shape. 11 new sendReleaseAnnouncement tests; full anvil-api suite
  363/363 green; typecheck clean.
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** none (`apps/anvil-api` is not part of the release tag
  surface; lib change is internal to the API deploy)
- **releaseNote:** audience `none`

## Phase 2 — Broadcast Endpoint

### EMAIL-005 — `POST /admin/broadcast` handler

- **Status:** Done
- **Priority:** Medium
- **Confidence:** Medium
- **Intent:** Implement the broadcast endpoint per the contract section
  above. Snapshot/preview/consume + drift flow lifted from
  `send-migration` and generalised over `(template, audience)` instead
  of just `(waitlist-migration, source)`. Validates `templateProps`
  against the registry schema before snapshotting. Rejects
  transactional templates with `template_kind_not_broadcastable`.
- **Expected Outcome:** Operator can dry-run a release-announcement to
  `beta:active-recent`, inspect the recipient list, then real-send with
  the returned `previewToken`. Cohort drift between the two calls
  rejects with `409 cohort_drift` carrying the recipient diff.
- **Validation:** `pnpm exec vitest --run src/__tests__/admin-broadcast.test.ts`
- **Files:** `apps/anvil-api/src/routes/admin.ts` (new handler +
  per-endpoint rate limit, renamed
  `SEND_MIGRATION_SNAPSHOT_TTL_SECONDS` → `BROADCAST_SNAPSHOT_TTL_SECONDS`
  with the send-migration handler updated in the same change),
  `apps/anvil-api/src/routes/admin-schemas.ts` (new `broadcastSchema` +
  `BroadcastInput` type),
  `apps/anvil-api/src/__tests__/admin-broadcast.test.ts` (new — 19
  tests covering input validation, dry-run flow, every real-send error
  code, and three bait-and-switch defences).
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** none
- **releaseNote:** audience `operator`, type `added`,
  text "Admin API gains `POST /admin/broadcast` for sending release
  announcements and other broadcasts to named audiences."
- **Dependencies:** EMAIL-001, EMAIL-002, EMAIL-003, EMAIL-004
- **Notes:** Landed 2026-05-24. The bait-and-switch defence is tested
  on three axes: templateProps (operator changes props between dry-run
  and real-send — snapshot wins), audience_key (operator changes
  audience — snapshot's key is what re-resolves), and audience_params.
  The `as Record<string, string>` cast on `consumed.audience_params`
  matches the storage contract — only string values are ever written
  via the request schema. Per-recipient send failures don't abort the
  batch; failed entries surface in the response `results` array with
  the provider error. Audit log writes `broadcast.email.sent` with
  template + audience + counts + previewToken. Full anvil-api suite
  382/382 across 20 files; typecheck clean.

### EMAIL-006 — `/admin/send-migration` back-compat shim

- **Status:** Done
- **Priority:** Medium
- **Confidence:** High
- **Intent:** Reduce `/admin/send-migration` to a thin handler that
  forwards `{ source, dryRun, limit, previewToken }` to the broadcast
  flow with `template: 'waitlist-migration'`, `audience:
  'waitlist:source'`, `audienceParams: { source }`. The admin CLI stays
  unchanged.
- **Expected Outcome:** Existing admin CLI calls to `/admin/send-migration`
  behave identically *except* that the resolver now excludes rows
  already in `beta_users` (per design decision 2). PR description calls
  out the narrowed cohort explicitly.
- **Validation:** `pnpm exec vitest --run src/__tests__/admin.test.ts`
- **Files:** `apps/anvil-api/src/routes/admin.ts` (extracted
  `executeBroadcastFromSnapshot` helper, refactored both
  `/admin/broadcast` and `/admin/send-migration` to use it; dropped
  `findWaitlistBySource` + direct `sendWaitlistMigration` imports),
  `apps/anvil-api/src/__tests__/admin.test.ts` (mocks switched from
  `findWaitlistBySource` to `resolveAudience` for send-migration tests;
  one assertion updated to new resolver signature).
- **changeType:** fix
- **releaseIntent:** candidate
- **releaseScope:** none
- **releaseNote:** audience `operator`, type `changed`,
  text "`/admin/send-migration` now excludes addresses already in
  `beta_users` from the recipient cohort."
- **Dependencies:** EMAIL-005
- **Notes:** Landed 2026-05-24. `executeBroadcastFromSnapshot(sql,
  consumed)` returns a discriminated union (`invalid_template` |
  `drift` | `sent`) that both handlers translate to their own response
  shape — broadcast emits `broadcast.email.sent` with template +
  audience + counts, send-migration emits `migration.email.sent` with
  source + counts (legacy shape preserved). Per design decision 2 the
  underlying `waitlist:source` resolver excludes already-invited
  addresses; admin-CLI behaviour otherwise unchanged. Full anvil-api
  suite 382/382 across 20 files; typecheck clean.

### EMAIL-007 — Council review remediation (Wave 1)

- **Status:** Done
- **Priority:** Medium
- **Confidence:** High
- **Intent:** Address the eight load-bearing findings from the full
  council review of EMAIL-001..006 (2026-05-24). The eight were
  selected from the council's 39 findings as the items the operator
  defends as must-fix; the remaining 31 are catalogued in Phase 6
  below for later attention.
- **Expected Outcome:** v0.7.0-beta release-announcement can be sent
  through `/admin/broadcast` without (a) breaking fresh-install paths,
  (b) producing a phishing-link surface via unvalidated URL props,
  (c) emailing service accounts or stale-token users, (d) silently
  missing cohort growth in drift detection, (e) crashing on a stale
  audience_key after consume, (f) running past the Vercel function
  timeout, (g) leaving a consumed snapshot with no audit trail, or
  (h) producing a malformed subject when only `version` or only
  `theme` is supplied.
- **Validation:** `pnpm exec vitest --run`
- **Files:**
  - `apps/anvil-api/src/db/schema.sql` — added `auth_method` column +
    index to `audit_log` mirroring migration 009 (fresh installs were
    breaking on the first `insertAuditLog`).
  - `apps/anvil-api/src/db/migrations/013-broadcast-snapshots.sql` —
    `IF EXISTS` / `IF NOT EXISTS` guards on every statement plus a
    `DO $$ ... $$` block around the source-column backfill so the
    migration is idempotent on schema-first fresh installs.
  - `apps/anvil-api/src/lib/broadcast-audiences.ts` — `waitlist:approved-no-token`
    excludes service accounts via `NOT EXISTS (... WHERE at.is_edict =
    true)`. Otherwise revoked/expired edict tokens would surface
    service accounts in the audience receiving 're-activate' emails
    intended for human users.
  - `apps/anvil-api/src/lib/email-registry.ts` — `httpsUrlSchema`
    helper constrains `releaseUrl`, `migrationUrl`, and
    `knownGaps[].trackingUrl` to `https://`-only URLs ≤ 2048 chars.
    `feedbackEmail` now uses `z.string().email().max(254)`. Closes
    the trusted-domain phishing vector.
  - `apps/anvil-api/src/lib/email.ts` — `sendReleaseAnnouncement`
    subject derivation switched from all-or-nothing
    `useDefaults` to per-field `props.X ?? V070_DEFAULTS.X`. Plain
    text body's releaseUrl fallback simplified to match.
  - `apps/anvil-api/src/routes/admin-schemas.ts` — `broadcastSchema.limit`
    capped at 80 (down from 5000) with default 80. Derivation in the
    schema comment: Vercel Pro 60 s default, Resend p99 500 ms,
    ~50 ms per-iteration overhead, 5 s response + 3 s cold-start
    budget → 80 recipients with margin. Raising the cap requires
    bounded concurrency or job-queue dispatch — both deferred to
    Phase 6.
  - `apps/anvil-api/src/routes/admin.ts` —
    `executeBroadcastFromSnapshot` validates `consumed.audience_key`
    against `AUDIENCE_KEYS` before `resolveAudience` (the switch had
    no default arm; a stale key would throw TypeError after the token
    was already consumed). `freshLimit` bumped to `snapshot_size + 1`
    so cohort growth surfaces as drift's `added` rather than being
    silently invisible. Per-recipient try/catch added inside the
    send loop so an SDK throw doesn't strand recipients with a
    consumed snapshot. Both `/admin/broadcast` and `/admin/send-migration`
    now write a `*.dispatch_started` audit row BEFORE the loop runs
    (recovery anchor if the function dies mid-loop) and a
    `*.blocked` audit row on the drift / invalid_template branches
    (consumed-token-with-no-send is itself a state change worth
    recording). `/admin/send-migration` now returns 400
    `template_kind_not_broadcastable` (matching `/admin/broadcast`)
    instead of 409 `cohort_drift` with empty arrays, fixing a
    client-re-preview infinite-loop bug. `snapshotSource` null guard
    prevents the literal string `"undefined"` leaking into the audit
    log and response.
- **Notes:** Landed 2026-05-24. 15 new tests; full anvil-api suite
  397/397 across 20 files (was 382/382); typecheck clean. Council
  review verdict was BLOCK on 13 must-fix items, downgraded by the
  operator to 8 after applying severity-pushback (see Phase 6 for
  the deferred 31).

## Phase 6 — Council Follow-Ups (Not Tasked Yet)

The full council review on 2026-05-24 surfaced 39 findings. 8 landed
in EMAIL-007 above; the remaining 31 are catalogued here for later
attention. Pulled in batches of 5–10 once an operator pressure point
surfaces.

### Wave 1 — Defensive hardening (when next broadcast scope tightens)

- **Hash snapshot token at rest.** Store `sha256(token)` as the
  PRIMARY KEY of `send_broadcast_snapshots`, return raw token only
  from `insertBroadcastSnapshot`, hash on lookup in
  `findBroadcastSnapshot` / `consumeBroadcastSnapshot`. Mirrors the
  `access_tokens` / `refresh_tokens` pattern. Severity: MINOR for
  consistency, not exploitability (consume endpoint requires
  `adminAuth`). ~30 lines.
- **`audience_params` cast at function boundary.** Tighten
  `insertBroadcastSnapshot`'s `audienceParams` parameter from
  `Record<string, unknown>` to `Record<string, string>`. Parse
  `consumed.audience_params` through `z.record(z.string(), z.string())`
  at the top of `executeBroadcastFromSnapshot` as belt-and-braces.
- **JSONB write surface caps.** Add max-key-count and max-value-size
  to `broadcastSchema.audienceParams` and `broadcastSchema.templateProps`.
  Currently uncapped — admin actor could write megabyte-scale blobs
  into the snapshot table.
- **`z.enum(AUDIENCE_KEYS)` / `z.enum(TEMPLATE_KEYS)`** in
  `broadcastSchema` instead of `z.string().min(1)`. Eliminates the
  drift risk between AUDIENCE_KEYS constant and the schema.

### Wave 2 — Observability and operations

- **Migration 013 `lock_timeout`.** Set `SET lock_timeout = '30s'`
  before the ALTER TABLE chain. Theoretical at current row count;
  best practice.
- **Reap DELETE structured logging.** Log row-count deleted at INFO
  on success, ERROR on failure. Currently `console.warn` with no
  count, so snapshot-table growth is invisible.
- **Structured logging in `lib/email.ts`.** Replace the
  `console.warn` / `console.error` calls with structured payloads
  carrying correlation ID, template name, Resend error code, and
  snapshot token. Currently a 200-recipient failure produces 200
  identical lines with no triage path.
- **Rate-limiter cluster-wide store.** Documented limitation in
  `admin-rate-limit.ts` — per-process counter on Vercel = `5 × N`
  burst across `N` warm instances. Move to shared store before
  scaling beyond single warm function. Document in runbook
  meanwhile.
- **Rate-limiter dry-run vs send split.** Operator iterating on
  previews burns the same budget as a real send. Split to
  `scope: 'broadcast:dry'` (looser) and `scope: 'broadcast:send'`
  (tighter).
- **`broadcast_id` for idempotency.** Surface the `previewToken` (or
  a separate dispatch UUID) in the broadcast response so an operator
  can correlate to Resend's dashboard if the HTTP response is lost.
- **Audit log namespace documentation.** `migration.email.*` and
  `broadcast.email.*` action names coexist; operators querying need
  both. Document the taxonomy in the audit-log schema comment.
- **`FROM_ADDRESS` / `REPLY_TO` env-configurable.** Currently
  hardcoded module constants — fine for a single operator, will bite
  the first staging/preview deploy that wants a different sender.

### Wave 3 — Compliance and deliverability

- **Suppression table + LEFT JOIN.** Persist Resend bounce/complaint
  events into a `suppressed_emails` table; every audience resolver
  joins against it and excludes matches. Honours the unsubscribe
  promise the rendered emails make. Originally pitched as MUST FIX
  by security-analyst; downgraded to ACCEPT-RISK by the operator
  given closed-beta scale + manual triage. Revisit when:
  (a) broadcast cadence exceeds quarterly, or (b) recipient base
  exceeds low hundreds, or (c) Gmail spam-folder rate exceeds 0.1%
  on a release-announcement send.
- **Suppression env-flag gate.** If the above is deferred, add
  `process.env.ADMIN_BROADCAST_ENABLED === 'true'` gate to the
  broadcast dispatch path. Council-debate landed on GATE-WITH-FLAG
  as the right control; operator pushed back as bureaucracy. Listed
  here so the next person revisiting compliance has the lever
  documented.
- **Per-recipient send-result audit detail.** The aggregate `sent` /
  `failed` audit metadata loses per-recipient attribution. Persist
  the full `results[]` array (already present in the response body)
  into the audit row's JSONB column so failed addresses can be
  recovered after a client-side log loss.

### Wave 4 — Semantic + UX polish

- **`waitlist:pending` rename.** Resolver returns "no `beta_users`
  row exists" — superset of the label "pending". Rename to
  `waitlist:not-invited` or `waitlist:no-beta-account`. NIT, churn.
- **`excluded_count` in dry-run response.** Suspended/banned waitlist
  users sit in a reachability gap. Surface the count of excluded
  rows so the operator sees the shortfall.
- **Drift name-change documentation.** `computeCohortDrift` compares
  email only; a `name` change between snapshot and consume produces
  stale personalisation invisibly. Document as accepted, or store
  `user_id` in `SnapshotRecipient` for a stricter check.
- **`now()` boundary documentation.** Time-windowed segmentation
  (`beta:active-recent` vs `beta:active-idle`) evaluates `now()`
  separately at snapshot vs consume. Document the inherent boundary
  gap inline in the resolver.
- **`consumed_at` column comment** in `schema.sql` describing the
  consume-once invariant.
- **`ReleaseAnnouncementSendProps` via `z.infer`.** Derive the type
  in `lib/email.ts` from `releaseAnnouncementPropsSchema` instead of
  hand-maintaining a parallel `Partial<{...}>`.
- **Operator CLI wrapper.** The current two-curl flow (preview, copy
  token, real-send) is error-prone. Phase 3 admin-CLI work picks
  this up.
- **`email-registry` sender-coupling watch.** Low-priority at five
  templates; if the registry grows, consider extracting sender
  registration into a separate index.

### Wave 5 — Original menu items (unchanged)

- **Phase 3 — Operator safety.** `POST /admin/send-test` for
  single-recipient render-and-mail of any registered template.
- **Phase 4 — Deliverability.** `POST /admin/email/webhook/resend`
  for Resend bounce / complaint webhooks (feeds Wave 3 suppression
  table).
- **Phase 5 — Recovery and reconcile.** `POST /admin/invite/resend`,
  `POST /admin/otp/resend`, `POST /admin/audience/reconcile`.

Each wave opens by adding its tasks here, advancing the module status
to `In Progress` if not already, and re-running readiness checks.

## Open Questions

1. **Per-row template props.** `release-announcement` accepts a `name`
   prop today via the template's signature but `sendWaitlistMigration`
   threads it through positionally. EMAIL-004 should standardise on
   passing the full props object so the broadcast handler can merge
   `{ ...templateProps, name: row.name }` once at the call site and
   every sender behaves identically. Confirm during EMAIL-004
   implementation.
2. **`limit` semantics under drift.** Current `send-migration` re-runs
   the query with `freshLimit = max(snapshot.recipients.length, 1)` to
   avoid false-positive drift from a request-body limit mismatch
   (`admin.ts:727`). The broadcast handler inherits this. Worth checking
   whether a smaller `limit` on a real-send than the dry-run captured
   should reject up front rather than re-resolving.
