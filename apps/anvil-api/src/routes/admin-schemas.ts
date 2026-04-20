import { z } from 'zod';

import { API_SCOPE_NAMES, type ApiScopeName } from '../lib/feature-flags.js';

// Derived from the api.scope.* flag manifest in ../lib/feature-flags.ts —
// the manifest is the single source of truth for valid scope names.
export const ALLOWED_SCOPES: readonly ApiScopeName[] = API_SCOPE_NAMES;

export const inviteSchema = z.object({
  email: z.string().email().max(254),
  name: z.string().max(200).optional(),
  notes: z.string().max(1000).optional(),
  days: z.number().int().positive().max(365).default(90),
  scopes: z.array(z.enum(API_SCOPE_NAMES)).default(['beta']),
  tokenOnly: z.boolean().default(false),
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
  .refine((data) => data.email || data.token, {
    message: 'Either email or token must be provided',
  });

export const migrationSchema = z.object({
  source: z.enum(['import', 'website', 'manual']).default('import'),
  dryRun: z.boolean().default(false),
  limit: z.number().int().min(1).max(100).default(20),
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
export type DriftDiffResponse = z.infer<typeof driftDiffSchema>;
export type UserEmailUpdateInput = z.infer<typeof userEmailUpdateSchema>;
export type WaitlistListQuery = z.infer<typeof waitlistListQuerySchema>;
export type AuditListQuery = z.infer<typeof auditListQuerySchema>;
