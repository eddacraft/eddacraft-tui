// @vitest-environment node
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { readJsonFileSync } from './file-io.js';

vi.mock('node:fs', async (importOriginal) => {
  const actual = await importOriginal<typeof import('node:fs')>();
  return {
    ...actual,
    default: actual,
    existsSync: vi.fn(),
    readFileSync: vi.fn(),
  };
});

import { existsSync, readFileSync } from 'node:fs';

describe('readJsonFileSync', () => {
  beforeEach(() => {
    vi.resetAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('should parse a valid JSON file', () => {
    vi.mocked(existsSync).mockReturnValue(true);
    vi.mocked(readFileSync).mockReturnValue('{"name": "test", "version": "1.0.0"}');

    const result = readJsonFileSync<{ name: string; version: string }>('/path/to/file.json');

    expect(result).toEqual({ name: 'test', version: '1.0.0' });
    expect(readFileSync).toHaveBeenCalledWith('/path/to/file.json', 'utf-8');
  });

  it('should return null when file does not exist', () => {
    vi.mocked(existsSync).mockReturnValue(false);

    const result = readJsonFileSync('/path/to/missing.json');

    expect(result).toBeNull();
    expect(readFileSync).not.toHaveBeenCalled();
  });

  it('should return null when file contains invalid JSON', () => {
    vi.mocked(existsSync).mockReturnValue(true);
    vi.mocked(readFileSync).mockReturnValue('not valid json {{{');

    const result = readJsonFileSync('/path/to/bad.json');

    expect(result).toBeNull();
  });

  it('should return null when readFileSync throws', () => {
    vi.mocked(existsSync).mockReturnValue(true);
    vi.mocked(readFileSync).mockImplementation(() => {
      throw new Error('EACCES: permission denied');
    });

    const result = readJsonFileSync('/path/to/protected.json');

    expect(result).toBeNull();
  });

  it('should handle empty JSON object', () => {
    vi.mocked(existsSync).mockReturnValue(true);
    vi.mocked(readFileSync).mockReturnValue('{}');

    const result = readJsonFileSync('/path/to/empty.json');

    expect(result).toEqual({});
  });

  it('should handle JSON arrays', () => {
    vi.mocked(existsSync).mockReturnValue(true);
    vi.mocked(readFileSync).mockReturnValue('[1, 2, 3]');

    const result = readJsonFileSync<number[]>('/path/to/array.json');

    expect(result).toEqual([1, 2, 3]);
  });
});
