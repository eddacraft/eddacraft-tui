import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
  success,
  info,
  error,
  warning,
  data,
  print,
  blank,
  json,
  debug,
  enableDebug,
  resetDebug,
  isDebugEnabled,
  formatGateResults,
  formatGateResultsJSON,
  formatValidationErrors,
} from './output.js';
import type { GateRunResult, GateRunResultWithCache } from '@eddacraft/anvil-runtime';

const MINIMAL_GATE_RESULT: GateRunResult = {
  overall: true,
  score: 100,
  checks: [],
  summary: { total: 0, passed: 0, failed: 0, skipped: 0 },
};

const MINIMAL_GATE_RESULT_WITH_CACHE: GateRunResultWithCache = {
  ...MINIMAL_GATE_RESULT,
  checks: [],
};

describe('output utilities stream policy', () => {
  let stderrSpy: ReturnType<typeof vi.spyOn>;
  let stdoutSpy: ReturnType<typeof vi.spyOn>;
  let logSpy: ReturnType<typeof vi.spyOn>;
  let warnSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    stderrSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
    logSpy = vi.spyOn(console, 'log').mockImplementation(() => {});
    stdoutSpy = vi.spyOn(process.stdout, 'write').mockImplementation(() => true);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('status helpers write to stderr', () => {
    it('success() writes to stderr', () => {
      success('done');
      expect(stderrSpy).toHaveBeenCalled();
      expect(logSpy).not.toHaveBeenCalled();
    });

    it('info() writes to stderr', () => {
      info('scanning');
      expect(stderrSpy).toHaveBeenCalled();
      expect(logSpy).not.toHaveBeenCalled();
    });

    it('error() writes to stderr', () => {
      error('failed');
      expect(stderrSpy).toHaveBeenCalled();
      expect(logSpy).not.toHaveBeenCalled();
    });

    it('warning() writes to stderr', () => {
      warning('caution');
      expect(warnSpy).toHaveBeenCalled();
      expect(logSpy).not.toHaveBeenCalled();
    });
  });

  describe('data() writes to stdout', () => {
    it('writes content followed by newline to stdout', () => {
      data('{"key":"value"}');
      expect(stdoutSpy).toHaveBeenCalledWith('{"key":"value"}\n');
      expect(stderrSpy).not.toHaveBeenCalled();
    });
  });

  describe('print() writes to stderr', () => {
    it('writes formatted text to stderr', () => {
      print('hello world');
      expect(stderrSpy).toHaveBeenCalledWith('hello world');
      expect(logSpy).not.toHaveBeenCalled();
    });

    it('passes multiple arguments to stderr', () => {
      print('prefix', 'suffix');
      expect(stderrSpy).toHaveBeenCalledWith('prefix', 'suffix');
    });
  });

  describe('blank() writes empty line to stderr', () => {
    it('writes empty string to stderr', () => {
      blank();
      expect(stderrSpy).toHaveBeenCalledWith('');
      expect(logSpy).not.toHaveBeenCalled();
    });
  });

  describe('json() writes to stdout', () => {
    it('writes pretty-printed JSON to stdout', () => {
      json({ key: 'value' });
      expect(stdoutSpy).toHaveBeenCalledWith('{\n  "key": "value"\n}\n');
      expect(stderrSpy).not.toHaveBeenCalled();
    });

    it('writes compact JSON when pretty=false', () => {
      json({ key: 'value' }, false);
      expect(stdoutSpy).toHaveBeenCalledWith('{"key":"value"}\n');
    });
  });

  describe('formatGateResultsJSON writes to stdout', () => {
    it('outputs JSON via stdout.write', () => {
      formatGateResultsJSON(MINIMAL_GATE_RESULT_WITH_CACHE);
      expect(stdoutSpy).toHaveBeenCalled();
      expect(stderrSpy).not.toHaveBeenCalled();

      const output = stdoutSpy.mock.calls[0][0] as string;
      const parsed = JSON.parse(output);
      expect(parsed.version).toBe('1.0.0');
      expect(parsed.overall).toBe(true);
    });

    it('includes engine metadata when provided', () => {
      formatGateResultsJSON(MINIMAL_GATE_RESULT_WITH_CACHE, {
        engine: 'dual',
        engineFallback: true,
      });

      const output = stdoutSpy.mock.calls[0][0] as string;
      const parsed = JSON.parse(output);
      expect(parsed.engine).toBe('dual');
      expect(parsed.engineFallback).toBe(true);
    });

    it('omits engine fields when engineMeta is not provided', () => {
      formatGateResultsJSON(MINIMAL_GATE_RESULT_WITH_CACHE);

      const output = stdoutSpy.mock.calls[0][0] as string;
      const parsed = JSON.parse(output);
      expect(parsed.engine).toBeUndefined();
      expect(parsed.engineFallback).toBeUndefined();
    });

    it('omits engineFallback when undefined in engineMeta', () => {
      formatGateResultsJSON(MINIMAL_GATE_RESULT_WITH_CACHE, {
        engine: 'legacy',
      });

      const output = stdoutSpy.mock.calls[0][0] as string;
      const parsed = JSON.parse(output);
      expect(parsed.engine).toBe('legacy');
      expect(parsed.engineFallback).toBeUndefined();
    });
  });

  describe('formatGateResults writes to stderr', () => {
    it('outputs human-readable gate results via console.error', () => {
      formatGateResults(MINIMAL_GATE_RESULT);
      expect(stderrSpy).toHaveBeenCalled();
      expect(logSpy).not.toHaveBeenCalled();
    });
  });

  describe('formatValidationErrors writes to stderr', () => {
    it('outputs nothing for empty errors', () => {
      formatValidationErrors([]);
      expect(stderrSpy).not.toHaveBeenCalled();
      expect(logSpy).not.toHaveBeenCalled();
    });

    it('outputs validation errors via console.error', () => {
      formatValidationErrors([
        { message: 'missing field', path: 'root.name' },
        { message: 'invalid type' },
      ]);
      expect(stderrSpy).toHaveBeenCalled();
      expect(logSpy).not.toHaveBeenCalled();
    });
  });

  describe('debug() writes to stderr only when enabled', () => {
    const originalEnv = process.env['ANVIL_DEBUG'];

    afterEach(() => {
      resetDebug();
      if (originalEnv === undefined) {
        delete process.env['ANVIL_DEBUG'];
      } else {
        process.env['ANVIL_DEBUG'] = originalEnv;
      }
    });

    it('is silent by default', () => {
      delete process.env['ANVIL_DEBUG'];
      debug('should not appear');
      expect(stderrSpy).not.toHaveBeenCalled();
      expect(logSpy).not.toHaveBeenCalled();
      expect(stdoutSpy).not.toHaveBeenCalled();
    });

    it('writes to stderr after enableDebug()', () => {
      enableDebug();
      debug('test message');
      expect(stderrSpy).toHaveBeenCalled();
      const call = stderrSpy.mock.calls[0];
      expect(call.join(' ')).toContain('[debug]');
      expect(call.join(' ')).toContain('test message');
    });

    it('writes to stderr when ANVIL_DEBUG=1', () => {
      process.env['ANVIL_DEBUG'] = '1';
      debug('env debug');
      expect(stderrSpy).toHaveBeenCalled();
      const call = stderrSpy.mock.calls[0];
      expect(call.join(' ')).toContain('[debug]');
    });

    it('writes to stderr when ANVIL_DEBUG=true (case-insensitive)', () => {
      process.env['ANVIL_DEBUG'] = 'True';
      debug('env debug true');
      expect(stderrSpy).toHaveBeenCalled();
    });

    it('isDebugEnabled() reflects env var state', () => {
      delete process.env['ANVIL_DEBUG'];
      const withoutEnv = isDebugEnabled();
      process.env['ANVIL_DEBUG'] = '1';
      expect(isDebugEnabled()).toBe(true);
      process.env['ANVIL_DEBUG'] = '';
      if (!withoutEnv) {
        expect(isDebugEnabled()).toBe(withoutEnv);
      }
    });
  });
});
