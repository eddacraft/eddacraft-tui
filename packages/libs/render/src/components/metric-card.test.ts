import { describe, expect, it } from 'vitest';

import { formatValue } from './metric-card.js';

describe('formatValue', () => {
  it('normalises rounded duration seconds before splitting minutes and seconds', () => {
    expect(formatValue('119.6', 'duration')).toBe('2m 0s');
  });
});
