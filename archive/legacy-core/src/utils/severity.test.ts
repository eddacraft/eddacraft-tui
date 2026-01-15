import { describe, it, expect } from 'vitest';
import { parseSeverity } from './severity.js';

describe('parseSeverity', () => {
  it('should parse "error" correctly', () => {
    expect(parseSeverity('error')).toBe('error');
    expect(parseSeverity('ERROR')).toBe('error');
    expect(parseSeverity('Error')).toBe('error');
  });

  it('should parse "warning" and "warn" correctly', () => {
    expect(parseSeverity('warning')).toBe('warning');
    expect(parseSeverity('WARNING')).toBe('warning');
    expect(parseSeverity('warn')).toBe('warning');
    expect(parseSeverity('WARN')).toBe('warning');
  });

  it('should parse "info" correctly', () => {
    expect(parseSeverity('info')).toBe('info');
    expect(parseSeverity('INFO')).toBe('info');
  });

  it('should return default value for invalid strings', () => {
    expect(parseSeverity('invalid')).toBe('info');
    expect(parseSeverity('unknown')).toBe('info');
    expect(parseSeverity('')).toBe('info');
  });

  it('should return default value for non-string types', () => {
    expect(parseSeverity(123)).toBe('info');
    expect(parseSeverity(null)).toBe('info');
    expect(parseSeverity(undefined)).toBe('info');
    expect(parseSeverity({})).toBe('info');
    expect(parseSeverity([])).toBe('info');
  });

  it('should use custom default value when provided', () => {
    expect(parseSeverity('invalid', 'error')).toBe('error');
    expect(parseSeverity(123, 'warning')).toBe('warning');
    expect(parseSeverity(null, 'error')).toBe('error');
  });

  it('should preserve valid values even with custom default', () => {
    expect(parseSeverity('error', 'warning')).toBe('error');
    expect(parseSeverity('info', 'error')).toBe('info');
  });

  it('should preserve valid values with undefined default', () => {
    expect(parseSeverity('error', undefined)).toBe('error');
    expect(parseSeverity('warning', undefined)).toBe('warning');
    expect(parseSeverity('info', undefined)).toBe('info');
  });
});
