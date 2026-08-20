import { describe, expect, it } from 'vitest';

import { findPublicTrustFailures } from './check-public-trust.mjs';

function securityPage(reportingGuidance) {
  return `
    <p>Verify releases against our public key.</p>
    {/* RESPONSIBLE DISCLOSURE */}
    <a href="mailto:security@eddacraft.ai">security@eddacraft.ai</a>
    ${reportingGuidance}
    {/* SEE ALSO */}
  `;
}

describe('public trust contract', () => {
  it('accepts the working reporting email without encryption guidance', () => {
    expect(findPublicTrustFailures(securityPage(''), false)).toEqual([]);
  });

  it.each([
    'For encrypted communications, download our key.',
    'Encrypt reports with PGP.',
    'Use our OpenPGP certificate.',
    'Fingerprint: 1234 5678',
    'Download /security-key.asc before reporting.',
  ])('rejects missing-key reporting guidance: %s', (guidance) => {
    expect(findPublicTrustFailures(securityPage(guidance), false)).toContain(
      'PGP guidance is published without the advertised key'
    );
  });

  it('rejects a placeholder fingerprint even when a key file exists', () => {
    expect(
      findPublicTrustFailures(
        securityPage('Fingerprint: XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX'),
        true
      )
    ).toContain('placeholder PGP fingerprint is published');
  });
});
