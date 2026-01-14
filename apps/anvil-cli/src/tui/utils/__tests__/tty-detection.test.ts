import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { isTUIAvailable, getTerminalSize, supportsColour } from '../tty-detection.js';

describe('TTY Detection', () => {
  const originalEnv = { ...process.env };

  beforeEach(() => {
    vi.resetModules();
    process.env = { ...originalEnv };
    delete process.env['NO_TUI'];
    delete process.env['CI'];
    delete process.env['GITHUB_ACTIONS'];
    delete process.env['NO_COLOR'];
    delete process.env['FORCE_COLOR'];
  });

  afterEach(() => {
    process.env = originalEnv;
  });

  describe('isTUIAvailable', () => {
    describe('explicit flags', () => {
      it('should return false when --no-tui flag is set', () => {
        Object.defineProperty(process.stdout, 'isTTY', { value: true, configurable: true });

        const result = isTUIAvailable({ noTui: true });

        expect(result).toBe(false);
      });

      it('should return false when tui option is explicitly false (Commander.js --no-tui behaviour)', () => {
        Object.defineProperty(process.stdout, 'isTTY', { value: true, configurable: true });

        // Commander.js sets options.tui = false for --no-tui, not options.noTui = true
        const result = isTUIAvailable({ tui: false });

        expect(result).toBe(false);
      });

      it('should return true when --tui flag is set and stdout is TTY', () => {
        Object.defineProperty(process.stdout, 'isTTY', { value: true, configurable: true });

        const result = isTUIAvailable({ tui: true });

        expect(result).toBe(true);
      });

      it('should return false when --tui flag is set but stdout is not TTY', () => {
        Object.defineProperty(process.stdout, 'isTTY', { value: false, configurable: true });
        const consoleSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});

        const result = isTUIAvailable({ tui: true });

        expect(result).toBe(false);
        expect(consoleSpy).toHaveBeenCalledWith('Warning: --tui requested but stdout is not a TTY');

        consoleSpy.mockRestore();
      });

      it('should prioritise --no-tui over --tui', () => {
        Object.defineProperty(process.stdout, 'isTTY', { value: true, configurable: true });

        const result = isTUIAvailable({ noTui: true, tui: true });

        expect(result).toBe(false);
      });
    });

    describe('output mode flags', () => {
      it('should return false when --json flag is set', () => {
        Object.defineProperty(process.stdout, 'isTTY', { value: true, configurable: true });

        const result = isTUIAvailable({ json: true });

        expect(result).toBe(false);
      });

      it('should return false when --quiet flag is set', () => {
        Object.defineProperty(process.stdout, 'isTTY', { value: true, configurable: true });

        const result = isTUIAvailable({ quiet: true });

        expect(result).toBe(false);
      });
    });

    describe('environment variables', () => {
      it('should return false when NO_TUI=1', () => {
        Object.defineProperty(process.stdout, 'isTTY', { value: true, configurable: true });
        process.env['NO_TUI'] = '1';

        const result = isTUIAvailable({});

        expect(result).toBe(false);
      });

      it('should return false when NO_TUI=true', () => {
        Object.defineProperty(process.stdout, 'isTTY', { value: true, configurable: true });
        process.env['NO_TUI'] = 'true';

        const result = isTUIAvailable({});

        expect(result).toBe(false);
      });

      it('should return false when CI=true', () => {
        Object.defineProperty(process.stdout, 'isTTY', { value: true, configurable: true });
        process.env['CI'] = 'true';

        const result = isTUIAvailable({});

        expect(result).toBe(false);
      });

      it('should return false when CI=1', () => {
        Object.defineProperty(process.stdout, 'isTTY', { value: true, configurable: true });
        process.env['CI'] = '1';

        const result = isTUIAvailable({});

        expect(result).toBe(false);
      });

      it('should return false when GITHUB_ACTIONS=true', () => {
        Object.defineProperty(process.stdout, 'isTTY', { value: true, configurable: true });
        process.env['GITHUB_ACTIONS'] = 'true';

        const result = isTUIAvailable({});

        expect(result).toBe(false);
      });
    });

    describe('TTY detection', () => {
      it('should return false when stdout is not a TTY', () => {
        Object.defineProperty(process.stdout, 'isTTY', { value: false, configurable: true });

        const result = isTUIAvailable({});

        expect(result).toBe(false);
      });

      it('should return true when stdout is a TTY and no disabling conditions', () => {
        Object.defineProperty(process.stdout, 'isTTY', { value: true, configurable: true });

        const result = isTUIAvailable({});

        expect(result).toBe(true);
      });

      it('should return false when stdout.isTTY is undefined', () => {
        Object.defineProperty(process.stdout, 'isTTY', { value: undefined, configurable: true });

        const result = isTUIAvailable({});

        expect(result).toBe(false);
      });
    });

    describe('priority order', () => {
      it('should check --no-tui before checking environment variables', () => {
        Object.defineProperty(process.stdout, 'isTTY', { value: true, configurable: true });
        process.env['NO_TUI'] = '1';

        expect(isTUIAvailable({ noTui: true })).toBe(false);
      });

      it('should check --tui before checking environment variables', () => {
        Object.defineProperty(process.stdout, 'isTTY', { value: true, configurable: true });
        process.env['CI'] = 'true';

        expect(isTUIAvailable({ tui: true })).toBe(true);
      });

      it('should check json/quiet flags before environment variables', () => {
        Object.defineProperty(process.stdout, 'isTTY', { value: true, configurable: true });

        expect(isTUIAvailable({ json: true })).toBe(false);
        expect(isTUIAvailable({ quiet: true })).toBe(false);
      });
    });

    describe('default behaviour', () => {
      it('should use empty options object when not provided', () => {
        Object.defineProperty(process.stdout, 'isTTY', { value: true, configurable: true });

        const result = isTUIAvailable();

        expect(result).toBe(true);
      });
    });
  });

  describe('getTerminalSize', () => {
    it('should return stdout dimensions when available', () => {
      Object.defineProperty(process.stdout, 'columns', { value: 120, configurable: true });
      Object.defineProperty(process.stdout, 'rows', { value: 40, configurable: true });

      const size = getTerminalSize();

      expect(size).toEqual({ columns: 120, rows: 40 });
    });

    it('should return default dimensions when stdout dimensions are undefined', () => {
      Object.defineProperty(process.stdout, 'columns', { value: undefined, configurable: true });
      Object.defineProperty(process.stdout, 'rows', { value: undefined, configurable: true });

      const size = getTerminalSize();

      expect(size).toEqual({ columns: 80, rows: 24 });
    });

    it('should return default dimensions when stdout dimensions are 0', () => {
      Object.defineProperty(process.stdout, 'columns', { value: 0, configurable: true });
      Object.defineProperty(process.stdout, 'rows', { value: 0, configurable: true });

      const size = getTerminalSize();

      expect(size).toEqual({ columns: 80, rows: 24 });
    });
  });

  describe('supportsColour', () => {
    it('should return false when NO_COLOR is set', () => {
      process.env['NO_COLOR'] = '1';

      const result = supportsColour();

      expect(result).toBe(false);
    });

    it('should return false when NO_COLOR is empty string (still defined)', () => {
      process.env['NO_COLOR'] = '';

      const result = supportsColour();

      expect(result).toBe(false);
    });

    it('should return true when FORCE_COLOR is set', () => {
      process.env['FORCE_COLOR'] = '1';

      const result = supportsColour();

      expect(result).toBe(true);
    });

    it('should return true when FORCE_COLOR is empty string (still defined)', () => {
      process.env['FORCE_COLOR'] = '';

      const result = supportsColour();

      expect(result).toBe(true);
    });

    it('should return true when stdout is TTY', () => {
      Object.defineProperty(process.stdout, 'isTTY', { value: true, configurable: true });

      const result = supportsColour();

      expect(result).toBe(true);
    });

    it('should return false when stdout is not TTY', () => {
      Object.defineProperty(process.stdout, 'isTTY', { value: false, configurable: true });

      const result = supportsColour();

      expect(result).toBe(false);
    });

    it('should return false when stdout.isTTY is undefined', () => {
      Object.defineProperty(process.stdout, 'isTTY', { value: undefined, configurable: true });

      const result = supportsColour();

      expect(result).toBe(false);
    });

    it('should prioritise NO_COLOR over FORCE_COLOR', () => {
      process.env['NO_COLOR'] = '1';
      process.env['FORCE_COLOR'] = '1';

      const result = supportsColour();

      expect(result).toBe(false);
    });

    it('should prioritise FORCE_COLOR over TTY detection', () => {
      Object.defineProperty(process.stdout, 'isTTY', { value: false, configurable: true });
      process.env['FORCE_COLOR'] = '1';

      const result = supportsColour();

      expect(result).toBe(true);
    });
  });
});
