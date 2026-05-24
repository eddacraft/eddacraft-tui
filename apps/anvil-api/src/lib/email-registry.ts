import { z } from 'zod';
import type { AudienceRow } from './broadcast-audiences.js';
import type { EmailDeliveryResult } from './email.js';
import { sendWaitlistMigration } from './email.js';

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

const knownGapSchema = z.object({
  title: z.string(),
  body: z.string(),
  trackingUrl: z.string().optional(),
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
    version: z.string().optional(),
    theme: z.string().optional(),
    intro: z.string().optional(),
    highlights: z.array(releaseHighlightSchema).optional(),
    releaseUrl: z.string().optional(),
    upgradeCommands: z.array(upgradeCommandSchema).optional(),
    firstInvocationNote: firstInvocationNoteSchema.optional(),
    migrationUrl: z.string().optional(),
    knownGaps: z.array(knownGapSchema).optional(),
    boringWeekAsk: boringWeekAskSchema.optional(),
    feedbackEmail: z.string().optional(),
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
    sender: async () => {
      throw new Error('sendReleaseAnnouncement not yet implemented (EMAIL-004)');
    },
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
