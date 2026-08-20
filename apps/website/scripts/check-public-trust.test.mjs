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

  it.each([
    'Fingerprint: XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX',
    'PGP Fingerprint: XXXX-XXXX-XXXX-XXXX',
    'PGP Fingerprint: XXXX:XXXX:XXXX:XXXX',
    'PGP Fingerprint: XXXX_XXXX_XXXX_XXXX',
    'OpenPGP Fingerprint: TO BE PUBLISHED',
    'OpenPGP Fingerprint: NOT YET PUBLISHED',
    'OpenPGP Fingerprint: TODO',
    'OpenPGP Fingerprint: TBA',
    'OpenPGP Fingerprint: ???? ???? ???? ????',
    'OpenPGP Fingerprint: [redacted]',
    '<p>PGP fingerprint</p><code>XXXX XXXX XXXX XXXX</code>',
    '<p>OpenPGP fingerprint</p>\n<pre>NOT YET PUBLISHED</pre>',
    `GPG Fingerprint: ${Array.from({ length: 4 }, () => '0'.repeat(4)).join(' ')}`,
    `GPG Fingerprint: ${Array.from({ length: 4 }, () => '0'.repeat(4)).join(':')}`,
    `GPG Fingerprint: ${Array.from({ length: 10 }, () => '0'.repeat(4)).join(' ')}`,
    `GPG Fingerprint: ${Array.from({ length: 10 }, () => '0'.repeat(4)).join(':')}`,
    `GPG Fingerprint: ${Array.from({ length: 10 }, () => '0'.repeat(4)).join('-')}`,
    `GPG Fingerprint: ${Array.from({ length: 16 }, () => '0'.repeat(4)).join(' ')}`,
    `GPG Fingerprint: ${Array.from({ length: 10 }, () => 'F'.repeat(4)).join(' ')}`,
  ])('rejects a placeholder fingerprint even when a key exists: %s', (guidance) => {
    expect(findPublicTrustFailures(securityPage(guidance), true)).toContain(
      'placeholder PGP fingerprint is published'
    );
  });

  it.each(['Z', '_TBD', '-TBD', ':TBD'])(
    'rejects an invalid adjacent fingerprint suffix: %s',
    (suffix) => {
      const fingerprint = Array.from({ length: 10 }, (_, index) =>
        (0xa000 + index).toString(16).toUpperCase()
      ).join('');

      expect(
        findPublicTrustFailures(
          securityPage(`<p>PGP fingerprint</p><code>${fingerprint}${suffix}</code>`),
          true
        )
      ).toContain('placeholder PGP fingerprint is published');
    }
  );

  it.each([10, 16])(
    'accepts a structurally valid %d-group fingerprint when the published key exists',
    (groupCount) => {
      const fingerprint = Array.from({ length: groupCount }, (_, index) =>
        (0xa000 + index).toString(16).toUpperCase()
      ).join(' ');

      expect(
        findPublicTrustFailures(securityPage(`PGP Fingerprint: ${fingerprint}`), true)
      ).toEqual([]);
    }
  );
});
