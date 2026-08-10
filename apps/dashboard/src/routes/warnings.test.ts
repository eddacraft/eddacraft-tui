import { describe, expect, it } from 'vitest';

import { protectionOverviewFixture } from '@/api/fixtures';
import { resolveWarningSelection } from '@/routes/warnings';

describe('warnings route selection', () => {
  it('does not reuse local selection for stale URL evidence', () => {
    const retained = protectionOverviewFixture.warnings[0]!;

    expect(resolveWarningSelection([retained], 'stale-evidence')).toBeUndefined();
  });

  it('does not reuse local selection when URL evidence is absent', () => {
    const retained = protectionOverviewFixture.warnings[0]!;

    expect(resolveWarningSelection([retained], undefined)).toBeUndefined();
  });

  it('resolves a current warning once its evidence id is in the URL', () => {
    const warning = protectionOverviewFixture.warnings[0]!;

    expect(resolveWarningSelection([warning], warning.evidence_id)).toBe(warning);
  });
});
