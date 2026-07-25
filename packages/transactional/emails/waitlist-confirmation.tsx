import { Body, Container, Head, Hr, Html, Link, Preview, Row, Section, Text } from 'react-email';
import * as React from 'react';

interface WaitlistConfirmationProps {
  email: string;
  unsubscribeMailto: string;
}

export function WaitlistConfirmation({
  email = 'you@example.com',
  unsubscribeMailto = 'mailto:anvil@updates.eddacraft.ai',
}: WaitlistConfirmationProps) {
  return (
    <Html>
      <Head />
      <Preview>You&apos;re on the anvil waitlist</Preview>
      <Body style={body}>
        <Container style={container}>
          <Section style={header}>
            <Row>
              <Text style={headerText}>
                <span style={prompt}>$ </span>
                <span style={headerBold}>anvil</span>
                <span style={prompt}> :: waitlist confirm</span>
              </Text>
            </Row>
          </Section>

          <Hr style={divider} />

          <Section style={content}>
            <Text style={okBadge}>[ OK ] Access request received</Text>
            <Text style={bodyText}>
              Your email <strong style={emailHighlight}>{email}</strong> has been added to the anvil
              waitlist.
            </Text>
            <Text style={muted}>
              We&apos;re onboarding engineering teams in controlled cohorts. You&apos;ll hear from
              us when your slot opens.
            </Text>
            <Text style={muted}>
              If you have any questions or feedback, just reply to this email &mdash; I personally
              respond to each one.
            </Text>
          </Section>

          <Hr style={divider} />

          <Section style={footer}>
            <Text style={footerSignoff}>
              — Josh
              <br />
              <span style={footerRole}>Founder, eddacraft</span>
            </Text>
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

export default WaitlistConfirmation;

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
  fontSize: '14px',
  color: '#EBEBEB',
};

const emailHighlight: React.CSSProperties = {
  color: '#f5f5f5',
};

const muted: React.CSSProperties = {
  margin: '0 0 12px',
  fontSize: '13px',
  lineHeight: '1.6',
  color: '#a3a3a3',
};

const footer: React.CSSProperties = {
  paddingTop: '24px',
};

const footerSignoff: React.CSSProperties = {
  margin: '0 0 20px',
  fontSize: '13px',
  lineHeight: '1.6',
  color: '#a3a3a3',
};

const footerRole: React.CSSProperties = {
  color: '#525252',
  fontSize: '12px',
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
