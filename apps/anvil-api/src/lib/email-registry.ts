import { z } from 'zod';
import type { AudienceRow } from './broadcast-audiences.js';
import type { EmailDeliveryResult, ReleaseAnnouncementSendProps } from './email.js';
import { sendReleaseAnnouncement, sendWaitlistMigration } from './email.js';

const emptyPropsSchema = z.object({}).strict();

// Per-field length caps prevent a 20-element array (the outer cap) of
// arbitrarily large strings from accumulating to megabyte-scale JSONB
// payloads in the snapshot row + audit log. Limits are generous vs.
// V070_DEFAULTS sizes (titles ≤ ~80 chars, bodies ≤ ~400 chars).
const releaseHighlightSchema = z.object({
  title: z.string().max(256),
  body: z.string().max(2048),
});

const upgradeCommandSchema = z.object({
  label: z.string().max(64),
  command: z.string().max(512),
});

const firstInvocationNoteSchema = z.object({
  state: z.string().max(64),
  recovery: z.string().max(512),
  rationale: z.string().max(2048),
});

// URL fields are constrained to https:// (no javascript:, no data:, no http://)
// and capped at 2048 bytes. These render into <Link href={...}> in the
// release-announcement template; an admin actor (or compromised admin key)
// who can shape templateProps would otherwise have a high-trust phishing
// vector under the from-address's valid SPF/DKIM/DMARC.
//
// Three categories rejected via superRefine that `z.string().url() + starts-with`
// alone would let through:
//   1. Userinfo URLs like `https://user:pass@evil.com` — visually pass
//      'starts with https://' but resolve to evil.com.
//   2. Leading / trailing whitespace — passes z.url() in some parsers
//      and lands in href attributes.
//   3. Embedded newlines or other control chars — passes z.url() and
//      lands raw in HTML attributes.
const httpsUrlSchema = z
  .string()
  .max(2048)
  .superRefine((s, ctx) => {
    if (s !== s.trim()) {
      ctx.addIssue({
        code: 'custom',
        message: 'URL must not contain leading/trailing whitespace',
      });
      return;
    }
    if (/[\r\n\t]/.test(s)) {
      ctx.addIssue({ code: 'custom', message: 'URL must not contain control characters' });
      return;
    }
    let u: URL;
    try {
      u = new URL(s);
    } catch {
      ctx.addIssue({ code: 'custom', message: 'must be a valid URL' });
      return;
    }
    if (u.protocol !== 'https:') {
      ctx.addIssue({ code: 'custom', message: 'must be an https:// URL' });
    }
    if (u.username || u.password) {
      ctx.addIssue({ code: 'custom', message: 'URL must not contain userinfo (user:pass@)' });
    }
  });

const knownGapSchema = z.object({
  title: z.string().max(256),
  body: z.string().max(2048),
  trackingUrl: httpsUrlSchema.optional(),
});

const boringWeekAskSchema = z.object({
  durationLabel: z.string().max(64),
  participantCount: z.string().max(64),
  replyInstruction: z.string().max(512),
});

// `email` and `unsubscribeMailto` are intentionally excluded: the first
// comes from the recipient row, the second is computed at send time.
// `strict()` ensures an operator cannot smuggle either through as a
// template prop.
// Array caps prevent an admin actor from inflating the snapshot row
// (and the per-recipient send-loop cost) by submitting many-thousand
// element arrays. Generous limits — V070_DEFAULTS has 6 highlights, 4
// upgrade commands, 2 known gaps — so 20 each is well above realistic
// use.
export const releaseAnnouncementPropsSchema = z
  .object({
    version: z.string().max(64).optional(),
    theme: z.string().max(256).optional(),
    intro: z.string().max(4096).optional(),
    highlights: z.array(releaseHighlightSchema).max(20).optional(),
    releaseUrl: httpsUrlSchema.optional(),
    upgradeCommands: z.array(upgradeCommandSchema).max(20).optional(),
    firstInvocationNote: firstInvocationNoteSchema.optional(),
    migrationUrl: httpsUrlSchema.optional(),
    knownGaps: z.array(knownGapSchema).max(20).optional(),
    boringWeekAsk: boringWeekAskSchema.optional(),
    feedbackEmail: z.string().email().max(254).optional(),
  })
  .strict();

export const otpCodePropsSchema = z.object({ code: z.string() }).strict();

// The invite email carries no per-invite props since GHCLIAUTH-007: it
// directs the recipient to `anvil auth login` instead of an activation URL.
export const betaInvitePropsSchema = z.object({}).strict();

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
