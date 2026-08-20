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

  it('rejects a reporting email outside responsible disclosure', () => {
    const page = `
      <a href="mailto:security@eddacraft.ai">security@eddacraft.ai</a>
      {/* RESPONSIBLE DISCLOSURE */}
      <p>No reporting channel is rendered here.</p>
      {/* SEE ALSO */}
    `;

    expect(findPublicTrustFailures(page, false)).toContain(
      'missing working security reporting email'
    );
  });

  it('rejects placeholder PGP guidance outside responsible disclosure', () => {
    const page = `
      {/* RESPONSIBLE DISCLOSURE */}
      <a href="mailto:security@eddacraft.ai">security@eddacraft.ai</a>
      {/* SEE ALSO */}
      <p>PGP fingerprint: XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX</p>
    `;

    expect(findPublicTrustFailures(page, false)).toEqual(
      expect.arrayContaining([
        'placeholder PGP fingerprint is published',
        'PGP guidance is published without the advertised key',
      ])
    );
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
