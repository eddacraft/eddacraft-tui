import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
  success,
  info,
  error,
  warning,
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
  let logSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    stderrSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    logSpy = vi.spyOn(console, 'log').mockImplementation(() => {});
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
      expect(stderrSpy).toHaveBeenCalled();
      expect(logSpy).not.toHaveBeenCalled();
    });
  });

  describe('formatGateResultsJSON writes to stdout', () => {
    it('outputs JSON via console.log', () => {
      formatGateResultsJSON(MINIMAL_GATE_RESULT_WITH_CACHE);
      expect(logSpy).toHaveBeenCalled();
      expect(stderrSpy).not.toHaveBeenCalled();

      const output = logSpy.mock.calls[0][0] as string;
      const parsed = JSON.parse(output);
      expect(parsed.version).toBe('1.0.0');
      expect(parsed.overall).toBe(true);
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
});
