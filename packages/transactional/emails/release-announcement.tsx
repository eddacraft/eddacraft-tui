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

const V070_DEFAULTS: Omit<ReleaseAnnouncementProps, 'email' | 'unsubscribeMailto'> = {
  version: 'v0.7.0-beta',
  theme: 'Daemon-Working End-to-End Protection',
  intro:
    'A new Anvil release is live. Every protection layer now operates as a single verifiable claim — from file-save through commit, push, and wrapped agent launch.',
  highlights: [
    {
      title: 'End-to-end daemon-backed protection',
      body: 'Hooks, the witness chain, baseline adoption, L4 policy, and `anvil-run` operate as one pipeline. Every commit is witnessed, every save passes the same checks, and every agent-driven write is attributable.',
    },
    {
      title: 'One typed protection claim across every surface',
      body: '`anvil status`, `anvil doctor`, the MCP server, and the TypeScript driver-client all emit the same shape. Editors, CI, and agents read identical state.',
    },
    {
      title: 'Wrapped agent launch via `anvil-run`',
      body: '`anvil-run --tool claude-code -- <command>` wraps Claude Code, Codex, Aider, Cursor, and Windsurf so the daemon can attribute work, enforce fences, and clean up stale sessions.',
    },
    {
      title: 'Signed `anvil update`',
      body: 'Every supported install path verifies a minisign signature on the downloaded binary before replacing the running one.',
    },
    {
      title: 'Hook coexistence with lefthook, husky, and pre-commit-framework',
      body: 'Anvil registers as a managed entry under your host hook manager instead of overwriting `.git/hooks/`.',
    },
    {
      title: '`anvil insights` weekly summary',
      body: 'A periodic value signal during the quiet middle of normal use, derived from the witness chain. Local-only, no telemetry.',
    },
  ],
  releaseUrl: 'https://github.com/eddacraft/anvil/releases/tag/v0.7.0-beta',
  upgradeCommands: [
    { label: 'Homebrew', command: 'brew upgrade eddacraft/tap/anvil' },
    { label: 'curl installer', command: 'curl -fsSL https://install.eddacraft.ai | sh' },
    { label: 'WinGet', command: 'winget upgrade --id eddacraft.anvil' },
    { label: 'Scoop', command: 'scoop update anvil' },
  ],
  firstInvocationNote: {
    state: 'authRequired',
    recovery: 'anvil auth login',
    rationale:
      'If your refresh session has been idle, your first `anvil status` or `anvil start` after upgrade will surface `state: "authRequired"`. The v0.7.0-beta auth surface deliberately distinguishes "needs login" from "login failed" so editors and scripts can route you to the right step.',
  },
  migrationUrl:
    'https://github.com/eddacraft/anvil/blob/main/docs/runbooks/v0.6.x-to-v0.7.0-beta-migration.md',
  knownGaps: [
    {
      title: 'Daemon-side `session.report_process` IPC handler unimplemented',
      body: "`anvil-run` recovers gracefully and exits cleanly, but you'll see a one-line stderr warning on each launch until the handler ships.",
      trackingUrl: 'https://github.com/eddacraft/anvil-001/issues/1827',
    },
    {
      title: 'Marketplace publishing track deferred',
      body: 'The `anvil-action` GitHub Action is still blocked on the licensing / pricing model lock and is not part of this release.',
    },
  ],
  boringWeekAsk: {
    durationLabel: 'one calendar week',
    participantCount: 'three or more',
    replyInstruction:
      "Reply with \"I'm in\" and the project you'll run it against. We'll send a one-page note covering what to log and where to file feedback.",
  },
  feedbackEmail: 'anvil@updates.eddacraft.ai',
};

export function ReleaseAnnouncement(propsIn: Partial<ReleaseAnnouncementProps>) {
  // The v0.7.0-beta defaults apply only when the caller doesn't override the
  // identifying release fields. A future release with `version="v0.7.1-beta"`
  // gets a blank canvas — optional sections (firstInvocationNote, knownGaps,
  // boringWeekAsk, migrationUrl) stay omitted unless the caller provides them.
  const useV070Defaults = !propsIn.version && !propsIn.theme;
  const merged = useV070Defaults ? { ...V070_DEFAULTS, ...propsIn } : propsIn;

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
  const preview = `Anvil ${version} — ${theme}`;
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

            <Text style={sectionLabel}>Upgrade &mdash; one command:</Text>
            {upgradeCommands.map((c, i) => (
              <Section key={i} style={codeBlock}>
                <Text style={codeLabelInline}>{c.label}</Text>
                <Text style={codeText}>$ {c.command}</Text>
              </Section>
            ))}

            {firstInvocationNote ? (
              <>
                <Text style={sectionLabel}>One thing to know about first invocation:</Text>
                <Text style={bodyText}>{firstInvocationNote.rationale}</Text>
                <Text style={bodyText}>Recovery is one command:</Text>
                <Section style={codeBlock}>
                  <Text style={codeText}>$ {firstInvocationNote.recovery}</Text>
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
