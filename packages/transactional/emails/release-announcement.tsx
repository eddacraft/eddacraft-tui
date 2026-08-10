import { Body, Container, Head, Hr, Html, Link, Preview, Row, Section, Text } from 'react-email';
import * as React from 'react';

export interface ReleaseHighlight {
  title: string;
  body: string;
}

export interface UpgradeCommand {
  label: string;
  command: string;
}

export interface FirstInvocationNote {
  state: string;
  recovery: string;
  rationale: string;
}

export interface KnownGap {
  title: string;
  body: string;
  trackingUrl?: string;
}

export interface BoringWeekAsk {
  durationLabel: string;
  participantCount: string;
  replyInstruction: string;
}

export interface ReleaseAnnouncementProps {
  email: string;
  version: string;
  theme: string;
  intro: string;
  highlights: ReleaseHighlight[];
  releaseUrl: string;
  upgradeCommands: UpgradeCommand[];
  firstInvocationNote?: FirstInvocationNote;
  migrationUrl?: string;
  knownGaps?: KnownGap[];
  boringWeekAsk?: BoringWeekAsk;
  feedbackEmail?: string;
  unsubscribeMailto: string;
}

// Exported so `sendReleaseAnnouncement` can derive a matching subject line
// when the operator omits version + theme. Removing or renaming this requires
// updating the sender's subject derivation in apps/anvil-api/src/lib/email.ts.
//
// Content tracks the latest published beta (v0.9.4-beta). Production broadcasts
// should still pass explicit templateProps; these defaults back previews and
// partial subject fallback. `V070_DEFAULTS` is a historical export name kept as
// a stable alias for importers.
export const CURRENT_RELEASE_DEFAULTS: Omit<
  ReleaseAnnouncementProps,
  'email' | 'unsubscribeMailto'
> = {
  version: 'v0.9.4-beta',
  theme: 'Beta roundup — new sign-in and a clearer daily path',
  intro:
    'A lot has landed since the last broad release note. This mail is a short roundup through v0.9.4-beta: how you sign in, how assistants use the graph, and how install advice matches reality — plus one important upgrade note for this hop only.',
  highlights: [
    {
      title: 'New sign-in: GitHub device flow (and OTP if you need it)',
      body: 'Default login is now `anvil auth login`: you get a short code and a GitHub verification link (works over SSH/tmux). No GitHub? `anvil auth login --otp` sends a one-time code to your beta email. After upgrade, sign in once with the new path if you still hold a legacy token or an idle session.',
    },
    {
      title: 'Assistant graph context over MCP',
      body: 'Supported AI clients can search symbols, dependents, callers, change impact, and affected tests — identity-only by default, with source snippets only behind operator consent.',
    },
    {
      title: 'First-run wins and quieter daily activation',
      body: '`anvil welcome` can surface a real finding on your own project. Repeat `anvil start` is more honest about what activation does (and what still needs watch/MCP). Multi-client MCP install and MCP 2.0 reconnect keep Claude, Codex, and friends attached.',
    },
    {
      title: 'Install method and upgrade advice that match reality',
      body: 'Official Windows and macOS installs report the right method, Windows upgrade copy is PowerShell (not a Unix pipe), and registration waits for durable membership before claiming failure. Fewer path-shaped “secret” false positives and leaner MCP allow replies when a write is clean.',
    },
    {
      title: 'Signed updates once you are current',
      body: 'Supported channels verify a minisign signature on the binary. After this one manual hop (below), day-to-day upgrades are `anvil update` on the channel you already use.',
    },
  ],
  releaseUrl: 'https://github.com/eddacraft/anvil/releases/tag/v0.9.4-beta',
  upgradeCommands: [
    {
      label: 'Homebrew (if that is how you installed)',
      command: 'brew upgrade eddacraft/tap/anvil',
    },
    {
      label: 'curl installer (macOS / Linux — reinstall once is fine)',
      command: 'curl -fsSL https://install.eddacraft.ai | sh',
    },
    {
      label: 'Windows official installer (PowerShell)',
      command: 'irm https://install.eddacraft.ai/windows | iex',
    },
    {
      label: 'WinGet (if that is how you installed)',
      command: 'winget upgrade --id eddacraft.anvil',
    },
    {
      label: 'Scoop (if that is how you installed)',
      command: 'scoop update anvil',
    },
  ],
  firstInvocationNote: {
    state: 'authRequired',
    recovery: 'anvil auth login',
    rationale:
      'This hop is special: if `anvil update` is missing, stale, or cannot see a newer channel, upgrade once with the same method you used originally (Homebrew / WinGet / Scoop) or re-run the curl / PowerShell installer. After that, `anvil update` is the normal path. Then sign in with the new auth surface if status or start asks: `anvil auth login` (GitHub device flow) or `anvil auth login --otp`.',
  },
  migrationUrl: 'https://docs.eddacraft.ai/anvil/beta-testing-guide',
  feedbackEmail: 'anvil@updates.eddacraft.ai',
};

/** @deprecated Prefer CURRENT_RELEASE_DEFAULTS; kept for stable import paths. */
export const V070_DEFAULTS = CURRENT_RELEASE_DEFAULTS;

export function ReleaseAnnouncement(propsIn: Partial<ReleaseAnnouncementProps>) {
  // Current-release defaults apply only when the caller omits both identifying
  // fields. A send with `version="v0.9.5-beta"` (and no theme) is a blank canvas
  // for optional sections unless the operator supplies them explicitly.
  const useCurrentDefaults = !propsIn.version && !propsIn.theme;
  const merged = useCurrentDefaults ? { ...CURRENT_RELEASE_DEFAULTS, ...propsIn } : propsIn;

  const email = merged.email ?? 'you@example.com';
  const version = merged.version ?? 'v0.0.0';
  const theme = merged.theme ?? '';
  const intro = merged.intro ?? '';
  const highlights = merged.highlights ?? [];
  const releaseUrl = merged.releaseUrl ?? '';
  const upgradeCommands = merged.upgradeCommands ?? [];
  const firstInvocationNote = merged.firstInvocationNote;
  const migrationUrl = merged.migrationUrl;
  const knownGaps = merged.knownGaps;
  const boringWeekAsk = merged.boringWeekAsk;
  const feedbackEmail = merged.feedbackEmail;
  const unsubscribeMailto =
    merged.unsubscribeMailto ?? 'mailto:anvil@updates.eddacraft.ai?subject=Unsubscribe';
  const preview = `anvil ${version} — ${theme}`;
  return (
    <Html>
      <Head />
      <Preview>{preview}</Preview>
      <Body style={body}>
        <Container style={container}>
          <Section style={header}>
            <Row>
              <Text style={headerText}>
                <span style={prompt}>$ </span>
                <span style={headerBold}>anvil</span>
                <span style={prompt}> :: release notice</span>
              </Text>
            </Row>
          </Section>

          <Hr style={divider} />

          <Section style={content}>
            <Text style={okBadge}>
              [ OK ] {version} &mdash; {theme}
            </Text>
            <Text style={bodyText}>{intro}</Text>

            <Text style={emailLine}>
              For <strong style={emailHighlight}>{email}</strong>
            </Text>

            <Text style={sectionLabel}>What&apos;s new:</Text>
            {highlights.map((h, i) => (
              <Text key={i} style={bodyText}>
                <strong style={emailHighlight}>{h.title}</strong> &mdash; {h.body}
              </Text>
            ))}

            <Text style={bodyText}>
              Full release notes:{' '}
              <Link href={releaseUrl} style={inlineLink}>
                {releaseUrl}
              </Link>
            </Text>

            <Text style={sectionLabel}>
              Upgrade this time &mdash; original method or reinstall once:
            </Text>
            <Text style={muted}>
              Pick the path that matches how you installed (or use the official installer line).
              After this hop, day-to-day upgrades are <code style={inlineCode}>anvil update</code>.
            </Text>
            {upgradeCommands.map((c, i) => (
              <Section key={i} style={codeBlock}>
                <Text style={codeLabelInline}>{c.label}</Text>
                <Text style={codeText}>$ {c.command}</Text>
              </Section>
            ))}

            {firstInvocationNote ? (
              <>
                <Text style={sectionLabel}>After you upgrade:</Text>
                <Text style={bodyText}>{firstInvocationNote.rationale}</Text>
                <Text style={bodyText}>Sign in (if prompted):</Text>
                <Section style={codeBlock}>
                  <Text style={codeText}>$ {firstInvocationNote.recovery}</Text>
                </Section>
                <Section style={codeBlock}>
                  <Text style={codeText}>$ anvil auth login --otp</Text>
                </Section>
                {migrationUrl ? (
                  <Text style={muted}>
                    Migration note:{' '}
                    <Link href={migrationUrl} style={inlineLink}>
                      {migrationUrl}
                    </Link>
                  </Text>
                ) : null}
              </>
            ) : null}

            {knownGaps && knownGaps.length > 0 ? (
              <>
                <Text style={sectionLabel}>Known gaps we&apos;re transparent about:</Text>
                {knownGaps.map((g, i) => (
                  <Text key={i} style={bodyText}>
                    <strong style={emailHighlight}>{g.title}</strong> &mdash; {g.body}
                    {g.trackingUrl ? (
                      <>
                        {' '}
                        Tracked at{' '}
                        <Link href={g.trackingUrl} style={inlineLink}>
                          {g.trackingUrl}
                        </Link>
                        .
                      </>
                    ) : null}
                  </Text>
                ))}
              </>
            ) : null}

            {boringWeekAsk ? (
              <>
                <Text style={sectionLabel}>A specific ask &mdash; the Boring Week:</Text>
                <Text style={bodyText}>
                  This release is the one we want to <strong style={emailHighlight}>sit on</strong>{' '}
                  for a while. We&apos;d like{' '}
                  <strong style={emailHighlight}>{boringWeekAsk.participantCount}</strong> of you to
                  run {version} on real work for{' '}
                  <strong style={emailHighlight}>{boringWeekAsk.durationLabel}</strong> under
                  fresh-user config (no developer overrides). Any disabled check, unresolved
                  suppression, or hook bypass you hit blocks the next release &mdash; so we&apos;d
                  rather hear about it now than later.
                </Text>
                <Text style={muted}>{boringWeekAsk.replyInstruction}</Text>
              </>
            ) : null}

            <Text style={sectionLabel}>How to report friction:</Text>
            <Text style={muted}>
              Reply to this email &mdash;{' '}
              {feedbackEmail ? (
                <>
                  <Link href={`mailto:${feedbackEmail}`} style={inlineLink}>
                    {feedbackEmail}
                  </Link>{' '}
                  reaches Josh directly. Bugs are best paired with{' '}
                </>
              ) : (
                <>Bugs are best paired with </>
              )}
              <code style={inlineCode}>anvil doctor --json</code> output, or opened as a GitHub
              issue at{' '}
              <Link href="https://github.com/eddacraft/anvil-001/issues" style={inlineLink}>
                github.com/eddacraft/anvil-001/issues
              </Link>
              .
            </Text>

            <Text style={footerSignoff}>
              &mdash; Josh
              <br />
              <span style={prompt}>anvil :: eddacraft.ai</span>
            </Text>
          </Section>

          <Hr style={divider} />

          <Section style={footer}>
            <Text style={footerBrand}>anvil :: eddacraft.ai</Text>
            <Text style={footerUnsub}>
              <Link href={unsubscribeMailto} style={unsubscribeLink}>
                unsubscribe
              </Link>
            </Text>
          </Section>
        </Container>
      </Body>
    </Html>
  );
}

export default ReleaseAnnouncement;

const fontMono = "'JetBrains Mono', 'Courier New', Courier, monospace";
const fontBody = "'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif";

const body: React.CSSProperties = {
  margin: 0,
  padding: 0,
  backgroundColor: '#0D0D0F',
  fontFamily: fontBody,
  color: '#EBEBEB',
};

const container: React.CSSProperties = {
  maxWidth: '560px',
  margin: '0 auto',
  padding: '40px 20px',
};

const header: React.CSSProperties = {
  paddingBottom: '24px',
};

const headerText: React.CSSProperties = {
  margin: 0,
  fontSize: '14px',
  lineHeight: '1.5',
  fontFamily: fontMono,
};

const prompt: React.CSSProperties = {
  color: '#737373',
};

const headerBold: React.CSSProperties = {
  color: '#EBEBEB',
  fontWeight: 'bold',
};

const divider: React.CSSProperties = {
  borderColor: '#2A2A2E',
  borderTop: '1px solid #2A2A2E',
  margin: 0,
};

const content: React.CSSProperties = {
  padding: '32px 0',
};

const okBadge: React.CSSProperties = {
  margin: '0 0 16px',
  fontSize: '14px',
  fontFamily: fontMono,
  color: '#CC5500',
};

const bodyText: React.CSSProperties = {
  margin: '0 0 18px',
  fontSize: '14px',
  lineHeight: '1.55',
  color: '#EBEBEB',
};

const emailLine: React.CSSProperties = {
  margin: '0 0 24px',
  fontSize: '13px',
  color: '#a3a3a3',
};

const emailHighlight: React.CSSProperties = {
  color: '#f5f5f5',
};

const sectionLabel: React.CSSProperties = {
  margin: '24px 0 12px',
  fontSize: '13px',
  color: '#737373',
};

const inlineLink: React.CSSProperties = {
  color: '#CC5500',
  textDecoration: 'underline',
};

const inlineCode: React.CSSProperties = {
  fontFamily: fontMono,
  fontSize: '12px',
  color: '#EBEBEB',
  backgroundColor: '#1a1a1a',
  padding: '2px 4px',
};

const codeBlock: React.CSSProperties = {
  backgroundColor: '#1a1a1a',
  border: '1px solid #2A2A2E',
  borderRadius: '0px',
  padding: '12px 16px',
  marginBottom: '12px',
};

const codeLabelInline: React.CSSProperties = {
  margin: '0 0 4px',
  fontSize: '11px',
  color: '#737373',
  fontFamily: fontMono,
};

const codeText: React.CSSProperties = {
  margin: 0,
  fontSize: '14px',
  color: '#EBEBEB',
  fontFamily: fontMono,
};

const muted: React.CSSProperties = {
  margin: '0 0 18px',
  fontSize: '13px',
  lineHeight: '1.55',
  color: '#a3a3a3',
};

const footerSignoff: React.CSSProperties = {
  margin: '32px 0 0',
  fontSize: '14px',
  color: '#EBEBEB',
  fontFamily: fontMono,
};

const footer: React.CSSProperties = {
  paddingTop: '24px',
};

const footerBrand: React.CSSProperties = {
  margin: '0 0 8px',
  fontSize: '11px',
  color: '#525252',
};

const footerUnsub: React.CSSProperties = {
  margin: 0,
  fontSize: '10px',
};

const unsubscribeLink: React.CSSProperties = {
  color: '#525252',
  textDecoration: 'underline',
};
