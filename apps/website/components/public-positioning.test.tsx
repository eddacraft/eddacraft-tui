import type { ReactNode } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';

import Home from '../app/page';
import { SocialCard } from '../app/social-card';
import { CLIFooter } from './cli-footer';
import { CompanyBand } from './company-band';
import { DecisionIntegrityFlywheel } from './decision-integrity-flywheel';
import { DecisionModel } from './decision-model';
import { DeliveryBoundary } from './delivery-boundary';
import { HeroSection, InstallCommandRow } from './hero-section';
import { ProductStages } from './product-stages';
import { ShippingProof } from './shipping-proof';
import { TrustGap } from './trust-gap';
import { WriteConsequence } from './write-consequence';

function textOf(element: ReactNode): string {
  return renderToStaticMarkup(element)
    .replace(/<[^>]+>/g, ' ')
    .replace(/\s+/g, ' ')
    .trim();
}

describe('rendered website positioning', () => {
  it('renders every required section through the Home composition', () => {
    const rendered = textOf(<Home />);

    for (const claim of [
      'TRUST THE CODE',
      '12 MCP CLIENTS',
      'PROTECTION IS THE ENTRY POINT.',
      'DECISION INTEGRITY FLYWHEEL',
      '// FOUR_STAGES',
      'THE CONTROL POINT SHIPS TODAY.',
      'THE SYSTEM THAT CREATES WORK SHOULD NOT JUDGE IT ALONE.',
      'TRUST INFRASTRUCTURE FOR AI-ASSISTED WORK.',
      'LET THE AGENT RUN. KEEP THE JUDGE.',
      'REVIEW_CAPACITY',
      'THE WRITE EITHER HAS A JUDGE,',
    ]) {
      expect(rendered).toContain(claim);
    }
  });

  it('renders each public claim in its owning component', () => {
    expect(textOf(<HeroSection />)).toContain('TRUST THE CODE');
    expect(textOf(<ShippingProof />)).toContain('12 MCP CLIENTS');
    expect(textOf(<TrustGap />)).toContain('PROTECTION IS THE ENTRY POINT.');
    expect(textOf(<TrustGap />)).toContain('DECISION INTEGRITY IS THE SYSTEM AROUND IT.');
    expect(textOf(<WriteConsequence />)).toContain('THE WRITE EITHER HAS A JUDGE,');
    expect(textOf(<DecisionIntegrityFlywheel />)).toContain('DECISION INTEGRITY FLYWHEEL');
    expect(textOf(<ProductStages />)).toContain('// FOUR_STAGES');
    expect(textOf(<ProductStages />)).toContain('DECIDE');
    expect(textOf(<DeliveryBoundary />)).toContain('THE CONTROL POINT SHIPS TODAY.');
    expect(textOf(<DecisionModel />)).toContain(
      'THE SYSTEM THAT CREATES WORK SHOULD NOT JUDGE IT ALONE.'
    );
    expect(textOf(<CompanyBand />)).toContain('TRUST INFRASTRUCTURE FOR AI-ASSISTED WORK.');
    expect(textOf(<CLIFooter />)).toContain('LET THE AGENT RUN. KEEP THE JUDGE.');
    expect(textOf(<SocialCard />)).toContain('TRUST THE CODE');
    expect(textOf(<SocialCard />)).toContain('MCP REQUEST :: anvil_validate_write');
  });

  it('names the ANVIL autonomy state in the trust gap', () => {
    expect(textOf(<TrustGap />)).toContain('ANVIL');
    expect(textOf(<TrustGap />)).toContain('BABYSIT');
    expect(textOf(<TrustGap />)).toContain('UNSUPERVISED');
  });

  it('exposes an accessible copy action for the unlocked install command', () => {
    const markup = renderToStaticMarkup(
      <InstallCommandRow command="brew install eddacraft/tap/anvil" />
    );
    expect(markup).toContain('aria-label="Copy install command"');
    expect(markup).toContain('[ COPY ]');
    expect(markup).toContain('brew install eddacraft/tap/anvil');
  });

  it('puts the waitlist form in the hero and keeps the key-unlock path', () => {
    const markup = renderToStaticMarkup(<HeroSection />);
    expect(markup).toContain('id="hero-waitlist-email"');
    expect(markup).toContain('type="email"');
    expect(markup).toContain('[ = ] request early access');
    expect(markup).toContain('have an access key? unlock install');
    // The redacted install row no longer occupies hero real estate.
    expect(markup).not.toContain('[EARLY-ACCESS]');
  });

  it('keeps the footer waitlist form alongside the hero form', () => {
    const markup = renderToStaticMarkup(<Home />);
    expect(markup).toContain('id="hero-waitlist-email"');
    expect(markup).toContain('id="waitlist-email"');
    expect(markup.match(/type="email"/g)).toHaveLength(2);
  });

  it('does not mistake unreachable JSX for rendered content', () => {
    const HiddenClaim = ({ show }: { show: boolean }) => (
      <section>
        {/* TRUST INFRASTRUCTURE */}
        {show ? <span>TRUST THE CODE</span> : null}
      </section>
    );

    const rendered = textOf(<HiddenClaim show={false} />);
    expect(rendered).not.toContain('TRUST INFRASTRUCTURE');
    expect(rendered).not.toContain('TRUST THE CODE');
  });
});
