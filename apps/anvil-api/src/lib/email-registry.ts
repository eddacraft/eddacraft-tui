import { z } from 'zod';
import type { AudienceRow } from './broadcast-audiences.js';
import type { EmailDeliveryResult, ReleaseAnnouncementSendProps } from './email.js';
import { sendReleaseAnnouncement, sendWaitlistMigration } from './email.js';

const emptyPropsSchema = z.object({}).strict();

const releaseHighlightSchema = z.object({
  title: z.string(),
  body: z.string(),
});

const upgradeCommandSchema = z.object({
  label: z.string(),
  command: z.string(),
});

const firstInvocationNoteSchema = z.object({
  state: z.string(),
  recovery: z.string(),
  rationale: z.string(),
});

// URL fields are constrained to https:// (no javascript:, no data:, no http://)
// and capped at 2048 bytes. These render into <Link href={...}> in the
// release-announcement template; an admin actor (or compromised admin key)
// who can shape templateProps would otherwise have a high-trust phishing
// vector under the from-address's valid SPF/DKIM/DMARC.
const httpsUrlSchema = z
  .string()
  .url()
  .max(2048)
  .refine((s) => s.startsWith('https://'), {
    message: 'must be an https:// URL',
  });

const knownGapSchema = z.object({
  title: z.string(),
  body: z.string(),
  trackingUrl: httpsUrlSchema.optional(),
});

const boringWeekAskSchema = z.object({
  durationLabel: z.string(),
  participantCount: z.string(),
  replyInstruction: z.string(),
});

// `email` and `unsubscribeMailto` are intentionally excluded: the first
// comes from the recipient row, the second is computed at send time.
// `strict()` ensures an operator cannot smuggle either through as a
// template prop.
export const releaseAnnouncementPropsSchema = z
  .object({
    version: z.string().max(64).optional(),
    theme: z.string().max(256).optional(),
    intro: z.string().max(4096).optional(),
    highlights: z.array(releaseHighlightSchema).optional(),
    releaseUrl: httpsUrlSchema.optional(),
    upgradeCommands: z.array(upgradeCommandSchema).optional(),
    firstInvocationNote: firstInvocationNoteSchema.optional(),
    migrationUrl: httpsUrlSchema.optional(),
    knownGaps: z.array(knownGapSchema).optional(),
    boringWeekAsk: boringWeekAskSchema.optional(),
    feedbackEmail: z.string().email().max(254).optional(),
  })
  .strict();

export const otpCodePropsSchema = z.object({ code: z.string() }).strict();

export const betaInvitePropsSchema = z
  .object({
    userCode: z.string(),
    activateUrl: z.string(),
  })
  .strict();

export type BroadcastTemplateKey = 'release-announcement' | 'waitlist-migration';
export type TransactionalTemplateKey = 'beta-invite' | 'otp-code' | 'waitlist-confirmation';
export type TemplateKey = BroadcastTemplateKey | TransactionalTemplateKey;

interface BroadcastTemplateEntry<T extends z.ZodTypeAny> {
  kind: 'broadcast';
  propsSchema: T;
  sender: (row: AudienceRow, props: z.infer<T>) => Promise<EmailDeliveryResult>;
}

interface TransactionalTemplateEntry<T extends z.ZodTypeAny> {
  kind: 'transactional';
  propsSchema: T;
}

export type EmailTemplateEntry =
  | BroadcastTemplateEntry<z.ZodTypeAny>
  | TransactionalTemplateEntry<z.ZodTypeAny>;

export const EMAIL_REGISTRY: Record<TemplateKey, EmailTemplateEntry> = {
  'release-announcement': {
    kind: 'broadcast',
    propsSchema: releaseAnnouncementPropsSchema,
    // The discriminated union widens props to z.infer<ZodTypeAny> = unknown
    // at this call site; propsSchema.parse() at the /admin/broadcast boundary
    // guarantees the shape so the cast is safe.
    sender: (row, props) =>
      sendReleaseAnnouncement(row.email, props as ReleaseAnnouncementSendProps),
  },
  'waitlist-migration': {
    kind: 'broadcast',
    propsSchema: emptyPropsSchema,
    sender: (row) => sendWaitlistMigration(row.email, row.name ?? undefined),
  },
  'beta-invite': {
    kind: 'transactional',
    propsSchema: betaInvitePropsSchema,
  },
  'otp-code': {
    kind: 'transactional',
    propsSchema: otpCodePropsSchema,
  },
  'waitlist-confirmation': {
    kind: 'transactional',
    propsSchema: emptyPropsSchema,
  },
};

export const TEMPLATE_KEYS = Object.keys(EMAIL_REGISTRY) as TemplateKey[];

export function isBroadcastTemplate(key: TemplateKey): boolean {
  return EMAIL_REGISTRY[key].kind === 'broadcast';
}
