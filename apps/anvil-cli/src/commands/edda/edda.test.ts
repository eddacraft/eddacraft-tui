import { describe, expect, it, vi } from 'vitest';
import { createEddaCommand } from './index.js';
import { createEddaListCommand, parseConfidence, parseSince } from './list.js';
import { createEddaShowCommand } from './show.js';
import { createEddaPromoteCommand } from './promote.js';
import { createEddaRetireCommand } from './retire.js';
import { createEddaTraceCommand } from './trace.js';

describe('edda command', () => {
  it('createEddaCommand returns edda command', () => {
    const command = createEddaCommand();

    expect(command.name()).toBe('edda');
  });

  it('createEddaListCommand returns list command', () => {
    const command = createEddaListCommand();

    expect(command.name()).toBe('list');
  });

  it('createEddaShowCommand returns show command', () => {
    const command = createEddaShowCommand();

    expect(command.name()).toBe('show');
  });

  it('createEddaPromoteCommand returns promote command', () => {
    const command = createEddaPromoteCommand();

    expect(command.name()).toBe('promote');
  });

  it('createEddaRetireCommand returns retire command', () => {
    const command = createEddaRetireCommand();

    expect(command.name()).toBe('retire');
  });

  it('createEddaTraceCommand returns trace command', () => {
    const command = createEddaTraceCommand();

    expect(command.name()).toBe('trace');
  });

  it('parent edda command registers list, show, promote, retire, and trace subcommands', () => {
    const command = createEddaCommand();
    const subcommands = command.commands.map((entry) => entry.name());

    expect(subcommands).toEqual(
      expect.arrayContaining(['list', 'show', 'promote', 'retire', 'trace'])
    );
    expect(subcommands).toHaveLength(5);
  });

  describe('edda list options', () => {
    it('registers --confidence option', () => {
      const command = createEddaListCommand();
      const option = command.options.find((o) => o.long === '--confidence');
      expect(option).toBeDefined();
      expect(option?.description).toContain('confidence');
    });

    it('registers --since option', () => {
      const command = createEddaListCommand();
      const option = command.options.find((o) => o.long === '--since');
      expect(option).toBeDefined();
      expect(option?.description).toContain('age');
    });

    it('registers --type option', () => {
      const command = createEddaListCommand();
      const option = command.options.find((o) => o.long === '--type');
      expect(option).toBeDefined();
    });

    it('registers --status option with active default', () => {
      const command = createEddaListCommand();
      const option = command.options.find((o) => o.long === '--status');
      expect(option).toBeDefined();
      expect(option?.defaultValue).toBe('active');
    });

    it('registers --limit option with default 20', () => {
      const command = createEddaListCommand();
      const option = command.options.find((o) => o.long === '--limit');
      expect(option).toBeDefined();
    });
  });

  describe('parseConfidence', () => {
    it('returns empty array for undefined', () => {
      expect(parseConfidence()).toEqual([]);
    });

    it('parses single confidence level', () => {
      expect(parseConfidence('high')).toEqual(['high']);
    });

    it('parses comma-separated confidence levels', () => {
      expect(parseConfidence('low,medium')).toEqual(['low', 'medium']);
    });

    it('trims whitespace around values', () => {
      expect(parseConfidence(' high , low ')).toEqual(['high', 'low']);
    });

    it('throws for invalid confidence level', () => {
      expect(() => parseConfidence('invalid')).toThrow('Invalid confidence level');
    });
  });

  describe('parseSince', () => {
    it('returns undefined for undefined', () => {
      expect(parseSince()).toBeUndefined();
    });

    it('parses days duration', () => {
      vi.useFakeTimers({ now: new Date('2026-03-21T00:00:00Z') });
      const result = parseSince('7d');
      expect(result).toBe(new Date('2026-03-14T00:00:00Z').toISOString());
      vi.useRealTimers();
    });

    it('parses hours duration', () => {
      vi.useFakeTimers({ now: new Date('2026-03-21T12:00:00Z') });
      const result = parseSince('24h');
      expect(result).toBe(new Date('2026-03-20T12:00:00Z').toISOString());
      vi.useRealTimers();
    });

    it('parses minutes duration', () => {
      vi.useFakeTimers({ now: new Date('2026-03-21T12:00:00Z') });
      const result = parseSince('30m');
      expect(result).toBe(new Date('2026-03-21T11:30:00Z').toISOString());
      vi.useRealTimers();
    });

    it('throws for invalid format', () => {
      expect(() => parseSince('abc')).toThrow('Invalid --since format');
    });

    it('throws for unsupported unit', () => {
      expect(() => parseSince('7w')).toThrow('Invalid --since format');
    });
  });
});
