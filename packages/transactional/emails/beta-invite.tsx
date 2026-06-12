import { Body, Container, Head, Hr, Html, Link, Preview, Row, Section, Text } from 'react-email';
import * as React from 'react';

interface BetaInviteProps {
  email: string;
  unsubscribeMailto: string;
}

export function BetaInvite({
  email = 'you@example.com',
  unsubscribeMailto = 'mailto:anvil@updates.eddacraft.ai',
}: BetaInviteProps) {
  return (
    <Html>
      <Head />
      <Preview>You&apos;re in &mdash; anvil beta access</Preview>
      <Body style={body}>
        <Container style={container}>
          <Section style={header}>
            <Row>
              <Text style={headerText}>
                <span style={prompt}>$ </span>
                <span style={headerBold}>anvil</span>
                <span style={prompt}> :: beta invite</span>
              </Text>
            </Row>
          </Section>

          <Hr style={divider} />

          <Section style={content}>
            <Text style={okBadge}>[ OK ] Beta access approved</Text>
            <Text style={bodyText}>
              Your email <strong style={emailHighlight}>{email}</strong> has been approved for the
              anvil beta.
            </Text>

            <Text style={sectionLabel}>First, install anvil (macOS / Linux):</Text>
            <Section style={codeBlock}>
              <Text style={codeText}>$ curl -fsSL https://install.eddacraft.ai | sh</Text>
            </Section>
            <Text style={muted}>
              On Windows:{' '}
              <span style={codeInline}>irm https://install.eddacraft.ai/windows | iex</span>
            </Text>

            <Text style={sectionLabel}>Then sign in from your terminal:</Text>
            <Section style={codeBlock}>
              <Text style={codeText}>$ anvil auth login</Text>
            </Section>
            <Text style={muted}>
              You&apos;ll be shown a short code and a github.com link &mdash; open it on any device
              and approve to finish signing in with GitHub.
            </Text>

            <Text style={sectionLabel}>No GitHub account? Use a one-time email code instead:</Text>
            <Section style={codeBlock}>
              <Text style={codeText}>$ anvil auth login --otp</Text>
            </Section>
            <Text style={muted}>The one-time code is sent to this email address.</Text>
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

export default BetaInvite;

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
  margin: '0 0 24px',
  fontSize: '15px',
  color: '#EBEBEB',
};

const emailHighlight: React.CSSProperties = {
  color: '#f5f5f5',
};

const sectionLabel: React.CSSProperties = {
  margin: '0 0 8px',
  fontSize: '14px',
  color: '#737373',
};

const codeBlock: React.CSSProperties = {
  backgroundColor: '#1a1a1a',
  border: '1px solid #2A2A2E',
  borderRadius: '0px',
  padding: '12px 16px',
  marginBottom: '12px',
};

const codeText: React.CSSProperties = {
  margin: 0,
  fontSize: '15px',
  color: '#EBEBEB',
  fontFamily: fontMono,
};

const codeInline: React.CSSProperties = {
  fontFamily: fontMono,
  color: '#EBEBEB',
};

const muted: React.CSSProperties = {
  margin: '0 0 24px',
  fontSize: '14px',
  color: '#a3a3a3',
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
