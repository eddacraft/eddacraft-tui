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
import { HeroSection } from './hero-section';
import { ProductStages } from './product-stages';
import { ShippingProof } from './shipping-proof';
import { TrustGap } from './trust-gap';

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
      '// FOUR_STAGE_CONTROL_PLANE',
      'THE CONTROL POINT SHIPS TODAY.',
      'THE SYSTEM THAT CREATES WORK SHOULD NOT JUDGE IT ALONE.',
      'TRUST INFRASTRUCTURE FOR AI-ASSISTED WORK.',
      'BUILD WITH SPEED. SHIP WITH INTEGRITY.',
    ]) {
      expect(rendered).toContain(claim);
    }
  });

  it('renders each public claim in its owning component', () => {
    expect(textOf(<HeroSection />)).toContain('TRUST THE CODE');
    expect(textOf(<ShippingProof />)).toContain('12 MCP CLIENTS');
    expect(textOf(<TrustGap />)).toContain('PROTECTION IS THE ENTRY POINT.');
    expect(textOf(<TrustGap />)).toContain('DECISION INTEGRITY IS THE SYSTEM AROUND IT.');
    expect(textOf(<DecisionIntegrityFlywheel />)).toContain('DECISION INTEGRITY FLYWHEEL');
    expect(textOf(<ProductStages />)).toContain('// FOUR_STAGE_CONTROL_PLANE');
    expect(textOf(<DeliveryBoundary />)).toContain('THE CONTROL POINT SHIPS TODAY.');
    expect(textOf(<DecisionModel />)).toContain(
      'THE SYSTEM THAT CREATES WORK SHOULD NOT JUDGE IT ALONE.'
    );
    expect(textOf(<CompanyBand />)).toContain('TRUST INFRASTRUCTURE FOR AI-ASSISTED WORK.');
    expect(textOf(<CLIFooter />)).toContain('BUILD WITH SPEED. SHIP WITH INTEGRITY.');
    expect(textOf(<SocialCard />)).toContain('TRUST THE CODE');
    expect(textOf(<SocialCard />)).toContain('MCP REQUEST :: anvil_validate_write');
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
