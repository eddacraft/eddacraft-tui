import React from 'react';
import { describe, it, expect } from 'vitest';
import { render } from 'ink-testing-library';
import { LogPanel } from '../LogPanel.js';
import {
  type LogEntry,
  createLogEntry,
  filterEntries,
  formatTimestamp,
  DEFAULT_LOG_FILTER,
} from '../types.js';

function createTestEntries(): LogEntry[] {
  return [
    { id: '1', timestamp: new Date('2025-01-01T10:00:00'), level: 'info', message: 'Starting up' },
    {
      id: '2',
      timestamp: new Date('2025-01-01T10:00:01'),
      level: 'debug',
      message: 'Loading config',
    },
    {
      id: '3',
      timestamp: new Date('2025-01-01T10:00:02'),
      level: 'warn',
      message: 'Config missing optional field',
    },
    {
      id: '4',
      timestamp: new Date('2025-01-01T10:00:03'),
      level: 'error',
      message: 'Failed to connect',
    },
    {
      id: '5',
      timestamp: new Date('2025-01-01T10:00:04'),
      level: 'info',
      message: 'Retrying connection',
    },
  ];
}

describe('LogPanel', () => {
  it('renders log entries', () => {
    const entries = createTestEntries();
    const { lastFrame } = render(<LogPanel entries={entries} focused={false} />);

    expect(lastFrame()).toContain('Starting up');
    expect(lastFrame()).toContain('Loading config');
    expect(lastFrame()).toContain('Failed to connect');
  });

  it('shows entry count', () => {
    const entries = createTestEntries();
    const { lastFrame } = render(<LogPanel entries={entries} focused={false} />);

    expect(lastFrame()).toContain('(5/5)');
  });

  it('shows title', () => {
    const entries = createTestEntries();
    const { lastFrame } = render(<LogPanel entries={entries} title="Test Logs" focused={false} />);

    expect(lastFrame()).toContain('Test Logs');
  });

  it('shows empty state when no entries', () => {
    const { lastFrame } = render(<LogPanel entries={[]} focused={false} />);

    expect(lastFrame()).toContain('No log entries');
  });

  it('shows log levels with icons', () => {
    const entries = createTestEntries();
    const { lastFrame } = render(<LogPanel entries={entries} focused={false} />);

    expect(lastFrame()).toContain('ERROR');
    expect(lastFrame()).toContain('WARN');
    expect(lastFrame()).toContain('INFO');
    expect(lastFrame()).toContain('DEBUG');
  });

  it('shows timestamps', () => {
    const entries = createTestEntries();
    const { lastFrame } = render(<LogPanel entries={entries} focused={false} />);

    expect(lastFrame()).toContain('10:00:00');
  });

  it('shows filter bar when enabled', () => {
    const entries = createTestEntries();
    const { lastFrame } = render(<LogPanel entries={entries} showFilter={true} focused={false} />);

    expect(lastFrame()).toContain('Filter:');
  });

  it('hides filter bar when disabled', () => {
    const entries = createTestEntries();
    const { lastFrame } = render(<LogPanel entries={entries} showFilter={false} focused={false} />);

    expect(lastFrame()).not.toContain('Filter:');
  });

  it('shows search bar when enabled', () => {
    const entries = createTestEntries();
    const { lastFrame } = render(<LogPanel entries={entries} showSearch={true} focused={false} />);

    expect(lastFrame()).toContain('Search:');
  });

  it('hides search bar when disabled', () => {
    const entries = createTestEntries();
    const { lastFrame } = render(<LogPanel entries={entries} showSearch={false} focused={false} />);

    expect(lastFrame()).not.toContain('Search:');
  });

  it('shows keyboard shortcuts', () => {
    const entries = createTestEntries();
    const { lastFrame } = render(<LogPanel entries={entries} focused={false} />);

    expect(lastFrame()).toContain('j/k scroll');
    expect(lastFrame()).toContain('g/G jump');
  });

  it('shows focused indicator when focused', () => {
    const entries = createTestEntries();
    const { lastFrame } = render(<LogPanel entries={entries} focused={true} />);

    expect(lastFrame()).toContain('(focused)');
  });

  it('limits visible entries to maxVisible', () => {
    const entries = Array.from({ length: 20 }, (_, i) => createLogEntry('info', `Message ${i}`));
    const { lastFrame } = render(<LogPanel entries={entries} maxVisible={5} focused={false} />);

    const frame = lastFrame();
    const messageMatches = frame.match(/Message \d+/g) || [];
    expect(messageMatches.length).toBeLessThanOrEqual(5);
  });

  it('shows scroll indicators when content overflows', () => {
    const entries = Array.from({ length: 20 }, (_, i) => createLogEntry('info', `Message ${i}`));
    const { lastFrame } = render(<LogPanel entries={entries} maxVisible={5} focused={false} />);

    expect(lastFrame()).toMatch(/[↑↓]/);
  });

  it('shows source when provided', () => {
    const entries: LogEntry[] = [
      { id: '1', timestamp: new Date(), level: 'info', message: 'Test', source: 'MyModule' },
    ];
    const { lastFrame } = render(<LogPanel entries={entries} focused={false} />);

    expect(lastFrame()).toContain('[MyModule]');
  });
});

describe('LogPanel types', () => {
  describe('createLogEntry', () => {
    it('creates entry with unique id', () => {
      const entry1 = createLogEntry('info', 'Test 1');
      const entry2 = createLogEntry('info', 'Test 2');

      expect(entry1.id).not.toBe(entry2.id);
    });

    it('creates entry with current timestamp', () => {
      const before = new Date();
      const entry = createLogEntry('info', 'Test');
      const after = new Date();

      expect(entry.timestamp.getTime()).toBeGreaterThanOrEqual(before.getTime());
      expect(entry.timestamp.getTime()).toBeLessThanOrEqual(after.getTime());
    });

    it('sets level and message', () => {
      const entry = createLogEntry('error', 'Something failed');

      expect(entry.level).toBe('error');
      expect(entry.message).toBe('Something failed');
    });

    it('includes source when provided', () => {
      const entry = createLogEntry('info', 'Test', 'TestSource');

      expect(entry.source).toBe('TestSource');
    });
  });

  describe('filterEntries', () => {
    it('filters by level', () => {
      const entries = createTestEntries();
      const filter = { ...DEFAULT_LOG_FILTER, levels: new Set(['error' as const]) };

      const filtered = filterEntries(entries, filter);

      expect(filtered).toHaveLength(1);
      expect(filtered[0].level).toBe('error');
    });

    it('filters by multiple levels', () => {
      const entries = createTestEntries();
      const filter = {
        ...DEFAULT_LOG_FILTER,
        levels: new Set(['error' as const, 'warn' as const]),
      };

      const filtered = filterEntries(entries, filter);

      expect(filtered).toHaveLength(2);
      expect(filtered.every((e) => e.level === 'error' || e.level === 'warn')).toBe(true);
    });

    it('filters by search term in message', () => {
      const entries = createTestEntries();
      const filter = { ...DEFAULT_LOG_FILTER, search: 'connect' };

      const filtered = filterEntries(entries, filter);

      expect(filtered).toHaveLength(2);
      expect(filtered.every((e) => e.message.toLowerCase().includes('connect'))).toBe(true);
    });

    it('filters by search term in source', () => {
      const entries: LogEntry[] = [
        { id: '1', timestamp: new Date(), level: 'info', message: 'Test', source: 'AuthModule' },
        { id: '2', timestamp: new Date(), level: 'info', message: 'Test', source: 'DbModule' },
      ];
      const filter = { ...DEFAULT_LOG_FILTER, search: 'Auth' };

      const filtered = filterEntries(entries, filter);

      expect(filtered).toHaveLength(1);
      expect(filtered[0].source).toBe('AuthModule');
    });

    it('is case-insensitive for search', () => {
      const entries = createTestEntries();
      const filter = { ...DEFAULT_LOG_FILTER, search: 'CONNECT' };

      const filtered = filterEntries(entries, filter);

      expect(filtered).toHaveLength(2);
    });

    it('combines level and search filters', () => {
      const entries = createTestEntries();
      const filter = {
        levels: new Set(['info' as const]),
        search: 'connect',
      };

      const filtered = filterEntries(entries, filter);

      expect(filtered).toHaveLength(1);
      expect(filtered[0].level).toBe('info');
      expect(filtered[0].message).toContain('connection');
    });

    it('returns all entries with default filter', () => {
      const entries = createTestEntries();

      const filtered = filterEntries(entries, DEFAULT_LOG_FILTER);

      expect(filtered).toHaveLength(entries.length);
    });
  });

  describe('formatTimestamp', () => {
    it('formats time with leading zeros', () => {
      const date = new Date('2025-01-01T09:05:03');

      const formatted = formatTimestamp(date);

      expect(formatted).toBe('09:05:03');
    });

    it('formats afternoon time correctly', () => {
      const date = new Date('2025-01-01T14:30:45');

      const formatted = formatTimestamp(date);

      expect(formatted).toBe('14:30:45');
    });
  });
});
