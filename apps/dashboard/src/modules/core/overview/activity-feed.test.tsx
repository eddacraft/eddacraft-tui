import { describe, expect, it } from 'vitest';

import { protectionOverviewFixture } from '@/api/fixtures';
import { deriveActivityEvents } from '@/modules/core/overview/activity-feed';

describe('deriveActivityEvents', () => {
  it('builds events from recent runs and warnings without inventing suppressions', () => {
    const events = deriveActivityEvents({
      ...protectionOverviewFixture,
      recent_runs: [
        {
          id: 'latest-gate',
          result: 'fail',
          label: 'FAILED',
          score: 50,
          warning_count: 2,
          duration_seconds: 1,
          started_at: null,
          new_warning_count: null,
          changed_file_count: null,
          checks: [],
        },
      ],
      warnings: [
        {
          id: 'w1',
          severity: 'high',
          category: 'Secrets',
          message: 'secret-detection: Potential secret',
          file_path: 'src/a.ts',
          age_label: 'Latest gate',
          evidence_id: 'w1',
          rule: 'secret-detection',
          line: 1,
          explanation: 'secret',
          matched_pattern: '',
          evidence_excerpt: [],
        },
      ],
    });

    expect(events).toHaveLength(2);
    expect(events[0]?.kind).toBe('gate');
    expect(events[1]?.kind).toBe('warning');
    expect(events.every((event) => event.kind !== ('suppression' as never))).toBe(true);
  });
});
