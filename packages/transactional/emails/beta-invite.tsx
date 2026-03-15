import {
  Body,
  Container,
  Head,
  Hr,
  Html,
  Link,
  Preview,
  Row,
  Section,
  Text,
} from '@react-email/components';
import * as React from 'react';

interface BetaInviteProps {
  email: string;
  userCode: string;
  activateUrl: string;
  unsubscribeMailto: string;
}

export function BetaInvite({
  email = 'you@example.com',
  userCode = 'ANVIL-7F3A',
  activateUrl = 'https://eddacraft.ai/auth/activate?code=ANVIL-7F3A',
  unsubscribeMailto = 'mailto:anvil@updates.eddacraft.ai',
}: BetaInviteProps) {
  return (
    <Html>
      <Head />
      <Preview>You&apos;re in &mdash; Anvil beta access</Preview>
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
              You&apos;ve been approved for the Anvil beta.
            </Text>

            <Text style={sectionLabel}>Activate in your browser:</Text>
            <Text style={linkBlock}>
              <Link href={activateUrl} style={activateLink}>
                {activateUrl}
              </Link>
            </Text>

            <Text style={sectionLabel}>Or run in your terminal:</Text>
            <Section style={codeBlock}>
              <Text style={codeText}>$ anvil auth login</Text>
            </Section>

            <Text style={activationCode}>{userCode}</Text>
            <Text style={codeLabel}>Your activation code</Text>

            <Text style={muted}>This code expires in 48 hours.</Text>
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

const fontFamily = "'Courier New', Courier, monospace";

const body: React.CSSProperties = {
  margin: 0,
  padding: 0,
  backgroundColor: '#0a0a0a',
  fontFamily,
  color: '#d4d4d4',
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
};

const prompt: React.CSSProperties = {
  color: '#737373',
};

const headerBold: React.CSSProperties = {
  color: '#d4d4d4',
  fontWeight: 'bold',
};

const divider: React.CSSProperties = {
  borderColor: '#262626',
  borderTop: '1px solid #262626',
  margin: 0,
};

const content: React.CSSProperties = {
  padding: '32px 0',
};

const okBadge: React.CSSProperties = {
  margin: '0 0 16px',
  fontSize: '14px',
  color: '#22c55e',
};

const bodyText: React.CSSProperties = {
  margin: '0 0 24px',
  fontSize: '14px',
  color: '#d4d4d4',
};

const sectionLabel: React.CSSProperties = {
  margin: '0 0 8px',
  fontSize: '13px',
  color: '#737373',
};

const linkBlock: React.CSSProperties = {
  margin: '0 0 24px',
  fontSize: '14px',
};

const activateLink: React.CSSProperties = {
  color: '#22c55e',
  textDecoration: 'underline',
};

const codeBlock: React.CSSProperties = {
  backgroundColor: '#1a1a1a',
  border: '1px solid #262626',
  borderRadius: '4px',
  padding: '12px 16px',
  marginBottom: '24px',
};

const codeText: React.CSSProperties = {
  margin: 0,
  fontSize: '14px',
  color: '#d4d4d4',
  fontFamily,
};

const activationCode: React.CSSProperties = {
  margin: '0 0 4px',
  fontSize: '24px',
  fontWeight: 'bold',
  color: '#ffffff',
  letterSpacing: '2px',
  textAlign: 'center' as const,
};

const codeLabel: React.CSSProperties = {
  margin: '0 0 16px',
  fontSize: '12px',
  color: '#737373',
  textAlign: 'center' as const,
};

const muted: React.CSSProperties = {
  margin: '0 0 8px',
  fontSize: '13px',
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
