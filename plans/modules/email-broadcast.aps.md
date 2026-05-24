<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Email Broadcast Surface

| Scope | Owner | Priority | Status      |
| ----- | ----- | -------- | ----------- |
| EMAIL | —     | Medium   | In Progress |

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

- **Status:** Ready
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
- **Validation:** `pnpm --filter @anvil/api test email-registry`
- **Files:** `apps/anvil-api/src/lib/email-registry.ts` (new),
  `apps/anvil-api/src/lib/__tests__/email-registry.test.ts` (new).
- **changeType:** internal
- **releaseIntent:** never
- **releaseScope:** none
- **releaseNote:** audience `none`

### EMAIL-004 — `sendReleaseAnnouncement` helper

- **Status:** Ready
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
- **Validation:** `pnpm --filter @anvil/api test email`
- **Files:** `apps/anvil-api/src/lib/email.ts`,
  `packages/transactional/emails/release-announcement.tsx` (no edits —
  the template already ships defaults for v0.7.0-beta).
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** none (`apps/anvil-api` is not part of the release tag
  surface; lib change is internal to the API deploy)
- **releaseNote:** audience `none`

## Phase 2 — Broadcast Endpoint

### EMAIL-005 — `POST /admin/broadcast` handler

- **Status:** Ready
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
- **Validation:** `pnpm --filter @anvil/api test admin-broadcast`
- **Files:** `apps/anvil-api/src/routes/admin.ts`,
  `apps/anvil-api/src/__tests__/admin-broadcast.test.ts` (new).
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** none
- **releaseNote:** audience `operator`, type `added`,
  text "Admin API gains `POST /admin/broadcast` for sending release
  announcements and other broadcasts to named audiences."
- **Dependencies:** EMAIL-001, EMAIL-002, EMAIL-003, EMAIL-004
- **Risks:** The bait-and-switch invariant (snapshot wins, request body
  loses) must hold across every snapshot field — not just `audience` as
  in today's `source` case. Tests should cover an operator changing
  `templateProps` between dry-run and real-send and confirm the snapshot
  props are what get sent.

### EMAIL-006 — `/admin/send-migration` back-compat shim

- **Status:** Ready
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
- **Validation:** `pnpm --filter @anvil/api test admin send-migration`
- **Files:** `apps/anvil-api/src/routes/admin.ts`,
  `apps/anvil-api/src/__tests__/admin.test.ts`.
- **changeType:** fix
- **releaseIntent:** candidate
- **releaseScope:** none
- **releaseNote:** audience `operator`, type `changed`,
  text "`/admin/send-migration` now excludes addresses already in
  `beta_users` from the recipient cohort."
- **Dependencies:** EMAIL-005

## Future Phases (Not Tasked Yet)

The following items were on the original menu but are deliberately not
tasked in this module. Each becomes a task when its phase opens:

- **Phase 3 — Operator safety.** `POST /admin/send-test` for
  single-recipient render-and-mail of any registered template; built on
  the registry already in place.
- **Phase 4 — Deliverability.** `POST /admin/email/webhook/resend` for
  Resend bounce / complaint webhooks, `suppressions` table populated by
  the webhook, suppression check wired through the resolver
  hard-exclusion hook from EMAIL-001.
- **Phase 5 — Recovery and reconcile.** `POST /admin/invite/resend`,
  `POST /admin/otp/resend`, `POST /admin/audience/reconcile` for Resend
  Contacts drift repair.

Each phase opens by adding its tasks here, advancing the module status to
`In Progress` if not already, and re-running readiness checks.

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
