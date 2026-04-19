/**
 * Shared admin-API response schemas.
 *
 * Canonical source of truth for the JSON shape every admin endpoint
 * emits. Both `@eddacraft/anvil-api` (server) and `@eddacraft/admin-cli`
 * (client) import from here so a response-shape drift shows up as a
 * type error at build time rather than as an undefined-access crash
 * inside the CLI renderer.
 *
 * Response schemas only — request schemas live alongside the route in
 * `apps/anvil-api/src/routes/admin-schemas.ts` where zValidator consumes
 * them. Keeping the two split avoids pulling the request-side schemas
 * into the CLI bundle and keeps the server wiring unchanged.
 */

import { z } from 'zod';

// ---------------------------------------------------------------------------
// Primitive fragments
// ---------------------------------------------------------------------------

/**
 * DB row ids round-trip as numeric strings. Some Neon columns arrive as
 * bigint-backed strings; coerce uniformly to `string` on the boundary.
 */
const IdString = z.union([z.string(), z.number(), z.bigint()]).transform((v) => String(v));

/** ISO-8601 timestamps. Dates are stringified for JSON transport. */
const IsoTimestamp = z.union([
  z.string().datetime({ offset: true }),
  z.date().transform((v) => v.toISOString()),
]);

const NullableIsoTimestamp = z.union([IsoTimestamp, z.null()]);

export const AuthMethodSchema = z.enum(['shared', 'per_operator']);
export type AuthMethod = z.infer<typeof AuthMethodSchema>;

// ---------------------------------------------------------------------------
// /admin/invite
// ---------------------------------------------------------------------------

/**
 * Default flow: invite email sent, device-code flow opens. `token` and
 * `expiresAt` are absent.
 *
 * tokenOnly=true flow: raw access token returned exactly once. Both
 * branches are valid server responses, so the schema carries both as
 * optional and the CLI enforces the semantic coupling in the renderer.
 */
export const InviteResponseSchema = z.object({
  user: z.object({ email: z.string(), id: IdString }),
  scopes: z.array(z.string()),
  token: z.string().optional(),
  expiresAt: IsoTimestamp.optional(),
});
export type InviteResponse = z.infer<typeof InviteResponseSchema>;

// ---------------------------------------------------------------------------
// /admin/approve
// ---------------------------------------------------------------------------

const ApprovedEntrySchema = z.object({
  email: z.string(),
  expiresAt: IsoTimestamp,
});

const SkippedEntrySchema = z.object({
  email: z.string(),
  reason: z.string(),
  message: z.string().optional(),
});

export const ApproveResponseSchema = z.object({
  approved: z.array(ApprovedEntrySchema),
  skipped: z.array(SkippedEntrySchema).optional(),
});
export type ApproveResponse = z.infer<typeof ApproveResponseSchema>;
export type ApprovedEntry = z.infer<typeof ApprovedEntrySchema>;
export type SkippedEntry = z.infer<typeof SkippedEntrySchema>;

// ---------------------------------------------------------------------------
// /admin/revoke
// ---------------------------------------------------------------------------

export const RevokeResponseSchema = z.object({
  revoked: z.number().int().min(0),
});
export type RevokeResponse = z.infer<typeof RevokeResponseSchema>;

// ---------------------------------------------------------------------------
// /admin/send-migration — dry-run + real-send branches
// ---------------------------------------------------------------------------

export const MIGRATION_SOURCES = ['import', 'website', 'manual'] as const;
export const MigrationSourceSchema = z.enum(MIGRATION_SOURCES);
export type MigrationSource = z.infer<typeof MigrationSourceSchema>;

const MigrationRecipientSchema = z.object({
  email: z.string(),
  name: z.string().nullable(),
});
export type MigrationRecipient = z.infer<typeof MigrationRecipientSchema>;

export const DryRunResponseSchema = z.object({
  dryRun: z.literal(true),
  source: MigrationSourceSchema,
  count: z.number().int().min(0),
  recipients: z.array(MigrationRecipientSchema),
  previewToken: z.string(),
  expiresAt: IsoTimestamp,
});
export type DryRunResponse = z.infer<typeof DryRunResponseSchema>;

const SendResultEntrySchema = z.object({
  email: z.string(),
  sent: z.boolean(),
  error: z.string().optional(),
});
export type SendResultEntry = z.infer<typeof SendResultEntrySchema>;

export const SendResponseSchema = z.object({
  source: MigrationSourceSchema,
  total: z.number().int().min(0),
  sent: z.number().int().min(0),
  failed: z.number().int().min(0),
  results: z.array(SendResultEntrySchema),
});
export type SendResponse = z.infer<typeof SendResponseSchema>;

/**
 * The dry-run path returns a literal `dryRun: true` discriminator so the
 * CLI can discriminate on a single field. The real-send response has no
 * `dryRun` field at all.
 */
export const SendMigrationResponseSchema = z.union([DryRunResponseSchema, SendResponseSchema]);
export type SendMigrationResponse = z.infer<typeof SendMigrationResponseSchema>;

/**
 * 409 body returned when the recipient set has drifted since the
 * snapshot was taken. Consumed by the CLI's error-rewriter to produce a
 * tailored recovery message.
 */
export const DriftDiffResponseSchema = z.object({
  code: z.literal('cohort_drift'),
  error: z.string(),
  added: z.array(z.string()),
  removed: z.array(z.string()),
});
export type DriftDiffResponse = z.infer<typeof DriftDiffResponseSchema>;

// ---------------------------------------------------------------------------
// /admin/waitlist (list)
// ---------------------------------------------------------------------------

const WaitlistItemSchema = z.object({
  email: z.string(),
  name: z.string().nullable(),
  source: z.string(),
  created_at: IsoTimestamp,
  approved_at: NullableIsoTimestamp,
});
export type WaitlistItem = z.infer<typeof WaitlistItemSchema>;

export const WaitlistResponseSchema = z.object({
  total: z.number().int().min(0),
  items: z.array(WaitlistItemSchema),
});
export type WaitlistResponse = z.infer<typeof WaitlistResponseSchema>;

// ---------------------------------------------------------------------------
// /admin/audit (list)
// ---------------------------------------------------------------------------

const AuditItemSchema = z.object({
  id: IdString,
  action: z.string(),
  actor: z.string(),
  metadata: z.union([z.record(z.string(), z.unknown()), z.null()]).transform((v) => v ?? {}),
  created_at: IsoTimestamp,
  auth_method: z.union([AuthMethodSchema, z.null()]).optional(),
});
export type AuditItem = z.infer<typeof AuditItemSchema>;

export const AuditResponseSchema = z.object({
  total: z.number().int().min(0),
  items: z.array(AuditItemSchema),
});
export type AuditResponse = z.infer<typeof AuditResponseSchema>;

// ---------------------------------------------------------------------------
// /admin/user/:email
// ---------------------------------------------------------------------------

const ShowUserSchema = z.object({
  id: IdString,
  email: z.string(),
  name: z.string().nullable(),
  status: z.string(),
  notes: z.string().nullable(),
  created_at: IsoTimestamp,
  updated_at: IsoTimestamp,
});
export type ShowUser = z.infer<typeof ShowUserSchema>;

const ShowTokenSchema = z.object({
  id: IdString,
  scopes: z.array(z.string()),
  expires_at: IsoTimestamp,
  revoked_at: NullableIsoTimestamp,
  created_at: IsoTimestamp,
});
export type ShowToken = z.infer<typeof ShowTokenSchema>;

/** Reuse the audit-item shape for recent audit rows on /admin/user/:email. */
export const ShowAuditEntrySchema = AuditItemSchema;
export type ShowAuditEntry = z.infer<typeof ShowAuditEntrySchema>;

export const ShowResponseSchema = z.object({
  user: ShowUserSchema,
  tokens: z.array(ShowTokenSchema),
  recentAudit: z.array(ShowAuditEntrySchema),
  auditError: z.boolean().optional(),
});
export type ShowResponse = z.infer<typeof ShowResponseSchema>;
