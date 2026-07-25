import { Body, Container, Head, Hr, Html, Link, Preview, Row, Section, Text } from 'react-email';
import * as React from 'react';

interface WaitlistMigrationProps {
  email: string;
  name?: string;
  unsubscribeMailto: string;
}

export function WaitlistMigration({
  email = 'you@example.com',
  name,
  unsubscribeMailto = 'mailto:anvil@updates.eddacraft.ai',
}: WaitlistMigrationProps) {
  const greeting = name ? `${name}, you` : 'You';

  return (
    <Html>
      <Head />
      <Preview>anvil has a new home &mdash; and you&apos;re on the beta waitlist</Preview>
      <Body style={body}>
        <Container style={container}>
          <Section style={header}>
            <Row>
              <Text style={headerText}>
                <span style={prompt}>$ </span>
                <span style={headerBold}>anvil</span>
                <span style={prompt}> :: status update</span>
              </Text>
            </Row>
          </Section>

          <Hr style={divider} />

          <Section style={content}>
            <Text style={infoBadge}>[ INFO ] Platform update</Text>
            <Text style={bodyText}>
              {greeting} signed up for early notifications on anvil. A lot has changed since then
              &mdash; we wanted to bring you up to speed.
            </Text>

            <Text style={sectionLabel}>What&apos;s new:</Text>

            <Text style={bodyText}>
              <strong style={highlight}>New website</strong> &mdash; We&apos;ve rebuilt{' '}
              <Link href="https://eddacraft.ai" style={inlineLink}>
                eddacraft.ai
              </Link>{' '}
              from the ground up. It&apos;s faster, cleaner, and reflects where the product is
              heading.
            </Text>

            <Text style={bodyText}>
              <strong style={highlight}>Documentation</strong> &mdash; Full docs are now live at{' '}
              <Link href="https://docs.eddacraft.ai" style={inlineLink}>
                docs.eddacraft.ai
              </Link>
              . Architecture guides, CLI reference, and getting started walkthroughs are all there.
            </Text>

            <Text style={bodyText}>
              <strong style={highlight}>Beta waitlist</strong> &mdash; Your email{' '}
              <strong style={highlight}>{email}</strong> has been moved to the formal beta waitlist.
              You don&apos;t need to sign up again. When your cohort opens, you&apos;ll receive an
              invite with activation instructions.
            </Text>

            <Text style={muted}>
              We&apos;re onboarding engineering teams in controlled cohorts to keep quality high.
              Capacity is limited &mdash; early signups like yours are prioritised.
            </Text>

            <Text style={muted}>
              Questions or feedback? Just reply to this email &mdash; I personally read and respond
              to every one.
            </Text>

            <Text style={footerSignoff}>
              &mdash; Josh
              <br />
              <span style={footerRole}>Founder, eddacraft</span>
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

export default WaitlistMigration;

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

const infoBadge: React.CSSProperties = {
  margin: '0 0 16px',
  fontSize: '14px',
  fontFamily: fontMono,
  color: '#CC5500',
};

const bodyText: React.CSSProperties = {
  margin: '0 0 24px',
  fontSize: '14px',
  color: '#EBEBEB',
  lineHeight: '1.6',
};

const highlight: React.CSSProperties = {
  color: '#f5f5f5',
};

const sectionLabel: React.CSSProperties = {
  margin: '0 0 16px',
  fontSize: '13px',
  color: '#737373',
  fontFamily: fontMono,
};

const inlineLink: React.CSSProperties = {
  color: '#CC5500',
  textDecoration: 'underline',
};

const muted: React.CSSProperties = {
  margin: '0 0 8px',
  fontSize: '13px',
  color: '#a3a3a3',
  lineHeight: '1.6',
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
