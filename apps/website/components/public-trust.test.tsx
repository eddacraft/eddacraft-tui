import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';

import SecurityPage from '../app/security/page';

describe('rendered public trust contract', () => {
  it('renders a usable reporting link inside responsible disclosure', () => {
    const markup = renderToStaticMarkup(<SecurityPage />);
    const disclosureStart = markup.indexOf('RESPONSIBLE DISCLOSURE');
    const disclosureEnd = markup.indexOf('SEE ALSO', disclosureStart);

    expect(disclosureStart).toBeGreaterThanOrEqual(0);
    expect(disclosureEnd).toBeGreaterThan(disclosureStart);
    expect(markup.slice(disclosureStart, disclosureEnd)).toContain(
      'href="mailto:security@eddacraft.ai"'
    );
  });
});
