import { Body, Container, Head, Hr, Html, Link, Preview, Row, Section, Text } from 'react-email';
import * as React from 'react';

interface OtpCodeProps {
  code: string;
  unsubscribeMailto: string;
}

export function OtpCode({
  code = '847291',
  unsubscribeMailto = 'mailto:anvil@updates.eddacraft.ai',
}: OtpCodeProps) {
  return (
    <Html>
      <Head />
      <Preview>Your anvil verification code</Preview>
      <Body style={body}>
        <Container style={container}>
          <Section style={header}>
            <Row>
              <Text style={headerText}>
                <span style={prompt}>$ </span>
                <span style={headerBold}>anvil</span>
                <span style={prompt}> :: verify</span>
              </Text>
            </Row>
          </Section>

          <Hr style={divider} />

          <Section style={content}>
            <Text style={codeDisplay}>{code}</Text>
            <Text style={muted}>This code expires in 10 minutes.</Text>
            <Text style={muted}>If you didn&apos;t request this, you can safely ignore it.</Text>
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

export default OtpCode;

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

const codeDisplay: React.CSSProperties = {
  margin: '0 0 24px',
  fontSize: '40px',
  fontFamily: fontMono,
  fontWeight: 'bold',
  color: '#CC5500',
  textAlign: 'center',
  letterSpacing: '8px',
  lineHeight: '1',
  padding: '24px 0',
};

const muted: React.CSSProperties = {
  margin: '0 0 8px',
  fontSize: '14px',
  color: '#a3a3a3',
  textAlign: 'center',
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
