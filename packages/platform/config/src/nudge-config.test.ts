import { describe, it, expect } from 'vitest';
import {
  DEFAULT_NUDGE_CONFIG,
  meetsNudgeThreshold,
  type NudgeConfig,
  type NudgeSeverityThreshold,
} from './nudge-config.js';

describe('DEFAULT_NUDGE_CONFIG', () => {
  it('should have expected defaults', () => {
    expect(DEFAULT_NUDGE_CONFIG).toEqual({
      enabled: true,
      interactive: false,
      severityThreshold: 'warning',
    });
  });

  it('should be typed as Readonly', () => {
    // Readonly<T> is a compile-time constraint; verify the shape is stable
    const config: Readonly<NudgeConfig> = DEFAULT_NUDGE_CONFIG;
    expect(config.enabled).toBe(true);
    expect(config.interactive).toBe(false);
    expect(config.severityThreshold).toBe('warning');
  });
});

describe('meetsNudgeThreshold', () => {
  it('should return true when severity equals threshold', () => {
    expect(meetsNudgeThreshold('error', 'error')).toBe(true);
    expect(meetsNudgeThreshold('warning', 'warning')).toBe(true);
    expect(meetsNudgeThreshold('info', 'info')).toBe(true);
  });

  it('should return true when severity exceeds threshold', () => {
    expect(meetsNudgeThreshold('error', 'warning')).toBe(true);
    expect(meetsNudgeThreshold('error', 'info')).toBe(true);
    expect(meetsNudgeThreshold('warning', 'info')).toBe(true);
  });

  it('should return false when severity is below threshold', () => {
    expect(meetsNudgeThreshold('info', 'warning')).toBe(false);
    expect(meetsNudgeThreshold('info', 'error')).toBe(false);
    expect(meetsNudgeThreshold('warning', 'error')).toBe(false);
  });

  it('should handle unknown severity as lowest (0)', () => {
    expect(meetsNudgeThreshold('unknown', 'info')).toBe(true);
    expect(meetsNudgeThreshold('unknown', 'warning')).toBe(false);
  });

  describe('with default threshold (warning)', () => {
    const threshold: NudgeSeverityThreshold = DEFAULT_NUDGE_CONFIG.severityThreshold;

    it('should include errors', () => {
      expect(meetsNudgeThreshold('error', threshold)).toBe(true);
    });

    it('should include warnings', () => {
      expect(meetsNudgeThreshold('warning', threshold)).toBe(true);
    });

    it('should exclude info', () => {
      expect(meetsNudgeThreshold('info', threshold)).toBe(false);
    });
  });
});
