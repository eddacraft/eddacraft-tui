import { z } from 'zod';

import { API_SCOPE_NAMES, type ApiScopeName } from '../lib/feature-flags.js';

// Derived from the api.scope.* flag manifest in ../lib/feature-flags.ts —
// the manifest is the single source of truth for valid scope names.
export const ALLOWED_SCOPES: readonly ApiScopeName[] = API_SCOPE_NAMES;

export const inviteSchema = z
  .object({
    email: z.string().email().max(254),
    name: z.string().max(200).optional(),
    notes: z.string().max(1000).optional(),
    days: z.number().int().positive().max(365).default(90),
    scopes: z.array(z.enum(API_SCOPE_NAMES)).default(['beta']),
    tokenOnly: z.boolean().default(false),
    edict: z.boolean().default(false),
  })
  .refine((data) => !data.edict || data.tokenOnly, {
    message: 'edict requires tokenOnly',
    path: ['edict'],
  });

export const approveSchema = z.union([
  z.object({ email: z.string().email().max(254) }),
  z.object({ batch: z.number().int().min(1).max(100) }),
]);

export const revokeSchema = z
  .object({
    email: z.string().email().max(254).optional(),
    token: z.string().max(200).optional(),
  })
  // Exactly one of {email, token} must be provided. The two paths carry
  // different semantics (account-level vs grant-level, see
  // `POST /admin/revoke` in `routes/admin.ts`); the admin-cli already
  // refuses to combine them client-side, but the server enforces XOR
  // independently so operators cannot get an ambiguous "silent take the
  // email branch" outcome when both are supplied.
  .refine((data) => Boolean(data.email) !== Boolean(data.token), {
    message: 'Exactly one of email or token must be provided',
  });

export const migrationSchema = z.object({
  source: z.enum(['import', 'website', 'manual']).default('import'),
  dryRun: z.boolean().default(false),
  // Aligned with broadcastSchema's cap (was 100). /admin/send-migration
  // is now a shim over the same executeBroadcastFromSnapshot loop, so
  // the same Vercel-timeout + Resend-p99 derivation applies.
  limit: z.number().int().min(1).max(80).default(20),
  // previewToken is required for real-sends but the handler enforces
  // that conditionally (with a specific 400 error code), so the schema
  // keeps it optional to avoid a generic zod error collapsing the two
  // failure modes.
  previewToken: z.string().min(1).max(128).optional(),
});

// Error body returned with 409 when the recipient set the operator
// confirmed (the snapshot) no longer matches what a fresh query returns.
// Added/removed lists are email strings, symmetric-difference-style.
export const driftDiffSchema = z.object({
  code: z.literal('cohort_drift'),
  error: z.string(),
  added: z.array(z.string()),
  removed: z.array(z.string()),
});

// POST /admin/broadcast request body. Template + audience values are
// validated as plain strings here; the handler narrows them against the
// EMAIL_REGISTRY and AUDIENCE_KEYS so unknown values yield specific
// `template_unknown` / `audience_unknown` codes instead of a generic
// zod enum mismatch.
//
// `limit` is capped at 80 — derived from Vercel Pro default 60s timeout,
// Resend p99 ~500ms, ~50ms per-iteration overhead, plus a 5s response /
// 3s cold-start budget. The synchronous send loop can survive 80
// recipients with margin. Raising the cap requires either bounded
// concurrency in the loop OR moving to a job-queue dispatch — both
// deferred to the EMAIL Phase 6 hardening slice.
export const broadcastSchema = z.object({
  template: z.string().min(1).max(64),
  audience: z.string().min(1).max(64),
  // Caps prevent megabyte-scale templateProps blobs from being
  // persisted into the snapshot table. Per-template propsSchema
  // does the structural validation; these are envelope guards.
  audienceParams: z
    .record(z.string().max(64), z.string().max(1024))
    .refine((o) => Object.keys(o).length <= 16, {
      message: 'audienceParams may not have more than 16 keys',
    })
    .optional()
    .default({}),
  templateProps: z
    .record(z.string().max(64), z.unknown())
    .refine((o) => Object.keys(o).length <= 64, {
      message: 'templateProps may not have more than 64 keys',
    })
    .optional()
    .default({}),
  limit: z.number().int().min(1).max(80).default(80),
  dryRun: z.boolean().default(false),
  previewToken: z.string().min(1).max(128).optional(),
});

export const userEmailUpdateSchema = z.object({
  currentEmail: z.string().email().max(254),
  newEmail: z.string().email().max(254),
});

// Query string schemas — numeric fields arrive as strings via URL and
// are coerced to numbers with bounded ranges. Defaults mirror the design
// spec (pending / all / 50 / 0).

export const WAITLIST_STATUSES = ['pending', 'approved', 'all'] as const;
export const WAITLIST_SOURCES = ['manual', 'website', 'import', 'all'] as const;

export const waitlistListQuerySchema = z.object({
  status: z.enum(WAITLIST_STATUSES).default('pending'),
  source: z.enum(WAITLIST_SOURCES).default('all'),
  limit: z.coerce.number().int().finite().min(1).max(200).default(50),
  offset: z.coerce.number().int().finite().min(0).default(0),
});

export const auditListQuerySchema = z.object({
  action: z.string().trim().min(1).max(100).optional(),
  // Actor values stored in audit_log are either email addresses
  // (OAuth writes use user.email; CLI writes pass X-Admin-Actor) or
  // the literal 'admin' fallback from resolveAdminActor when no header
  // is present. Match the write-time sanitisation (printable ASCII,
  // trimmed, ≤200 chars) rather than enforce email-only.
  actor: z
    .string()
    .trim()
    .min(1)
    .max(200)
    .regex(/^[\x20-\x7E]+$/, 'actor must be printable ASCII')
    .optional(),
  limit: z.coerce.number().int().finite().min(1).max(200).default(50),
  offset: z.coerce.number().int().finite().min(0).default(0),
});

export type InviteInput = z.infer<typeof inviteSchema>;
export type ApproveInput = z.infer<typeof approveSchema>;
export type RevokeInput = z.infer<typeof revokeSchema>;
export type MigrationInput = z.infer<typeof migrationSchema>;
export type BroadcastInput = z.infer<typeof broadcastSchema>;
export type DriftDiffResponse = z.infer<typeof driftDiffSchema>;
export type UserEmailUpdateInput = z.infer<typeof userEmailUpdateSchema>;
export type WaitlistListQuery = z.infer<typeof waitlistListQuerySchema>;
export type AuditListQuery = z.infer<typeof auditListQuerySchema>;
