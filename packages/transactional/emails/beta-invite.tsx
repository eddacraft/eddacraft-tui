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
            <Text style={okBadge}>[ OK ] Your beta access is ready</Text>
            <Text style={bodyText}>
              Your email <strong style={emailHighlight}>{email}</strong> has been granted access to
              the anvil beta.
            </Text>

            <Text style={sectionHeading}>Start here</Text>
            <Text style={bodyText}>
              Before installing, we recommend reviewing the{' '}
              <Link href="https://docs.eddacraft.ai/anvil/beta-testing-guide" style={inlineLink}>
                beta guide
              </Link>
              . Installation options for macOS, Linux, and Windows are available at{' '}
              <Link href="https://install.eddacraft.ai" style={inlineLink}>
                install.eddacraft.ai
              </Link>
              .
            </Text>

            <Text style={sectionHeading}>Quick install</Text>
            <Text style={sectionLabel}>macOS / Linux</Text>
            <Section style={codeBlock}>
              <Text style={codeText}>curl -fsSL https://install.eddacraft.ai | sh</Text>
            </Section>
            <Text style={sectionLabel}>Windows</Text>
            <Section style={codeBlock}>
              <Text style={codeText}>irm https://install.eddacraft.ai/windows | iex</Text>
            </Section>

            <Text style={sectionHeading}>Sign in</Text>
            <Text style={sectionLabel}>Once installed:</Text>
            <Section style={codeBlock}>
              <Text style={codeText}>anvil auth login</Text>
            </Section>
            <Text style={muted}>
              You&apos;ll be shown a short code and a GitHub verification link. Open the link on any
              device and approve access to complete sign-in.
            </Text>

            <Text style={sectionHeading}>No GitHub account?</Text>
            <Section style={codeBlock}>
              <Text style={codeText}>anvil auth login --otp</Text>
            </Section>
            <Text style={muted}>
              A one-time verification code will be sent to this email address.
            </Text>

            <Text style={sectionHeading}>Documentation</Text>
            <Text style={bodyText}>
              Full documentation is available at{' '}
              <Link href="https://docs.eddacraft.ai" style={inlineLink}>
                docs.eddacraft.ai
              </Link>
              .
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

const sectionHeading: React.CSSProperties = {
  margin: '8px 0 8px',
  fontSize: '15px',
  fontWeight: 'bold',
  color: '#EBEBEB',
};

const inlineLink: React.CSSProperties = {
  color: '#CC5500',
  textDecoration: 'underline',
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
