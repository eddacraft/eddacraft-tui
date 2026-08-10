import { describe, expect, it } from 'vitest';

import { protectionOverviewFixture } from '@/api/fixtures';
import { resolveWarningSelection } from '@/routes/warnings';

describe('warnings route selection', () => {
  it('does not reuse local selection for stale URL evidence', () => {
    const retained = protectionOverviewFixture.warnings[0]!;

    expect(resolveWarningSelection([retained], 'stale-evidence', retained)).toBeUndefined();
    expect(resolveWarningSelection([retained], undefined, retained)).toBe(retained);
  });
});
