import type { ReactNode } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';

import { SocialCard } from '../app/social-card';
import { CompanyBand } from './company-band';
import { HeroSection } from './hero-section';
import { TrustGap } from './trust-gap';

function textOf(element: ReactNode): string {
  return renderToStaticMarkup(element)
    .replace(/<[^>]+>/g, ' ')
    .replace(/\s+/g, ' ')
    .trim();
}

describe('rendered website positioning', () => {
  it('renders each public claim in its owning component', () => {
    expect(textOf(<HeroSection />)).toContain('TRUST THE CODE');
    expect(textOf(<TrustGap />)).toContain('PROTECTION IS THE ENTRY POINT.');
    expect(textOf(<TrustGap />)).toContain('DECISION INTEGRITY IS THE SYSTEM AROUND IT.');
    expect(textOf(<CompanyBand />)).toContain('TRUST INFRASTRUCTURE');
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
