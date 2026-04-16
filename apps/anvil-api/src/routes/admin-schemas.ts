import { z } from 'zod';

export const ALLOWED_SCOPES = ['beta', 'preview', 'internal'] as const;

export const inviteSchema = z.object({
  email: z.string().email().max(254),
  name: z.string().max(200).optional(),
  notes: z.string().max(1000).optional(),
  days: z.number().int().positive().max(365).default(90),
  scopes: z.array(z.enum(ALLOWED_SCOPES)).default(['beta']),
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
export type WaitlistListQuery = z.infer<typeof waitlistListQuerySchema>;
export type AuditListQuery = z.infer<typeof auditListQuerySchema>;
