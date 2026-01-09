import { describe, it, expect, vi, beforeEach } from 'vitest';
import { withSpinner, createSpinner } from './spinner.js';
import ora from 'ora';

vi.mock('ora');

describe('spinner utilities', () => {
  let mockSpinner: any;

  beforeEach(() => {
    mockSpinner = {
      start: vi.fn().mockReturnThis(),
      succeed: vi.fn().mockReturnThis(),
      fail: vi.fn().mockReturnThis(),
      text: '',
    };
    vi.mocked(ora).mockReturnValue(mockSpinner);
  });

  describe('withSpinner', () => {
    it('should start and succeed spinner on successful execution', async () => {
      const mockFn = vi.fn().mockResolvedValue('result');

      const result = await withSpinner({ text: 'Loading...' }, mockFn);

      expect(ora).toHaveBeenCalledWith('Loading...');
      expect(mockSpinner.start).toHaveBeenCalled();
      expect(mockSpinner.succeed).toHaveBeenCalledWith('Loading...');
      expect(result).toBe('result');
    });

    it('should use custom success text when provided', async () => {
      const mockFn = vi.fn().mockResolvedValue('result');

      await withSpinner({ text: 'Loading...', successText: 'Loaded successfully!' }, mockFn);

      expect(mockSpinner.succeed).toHaveBeenCalledWith('Loaded successfully!');
    });

    it('should fail spinner and rethrow error on failure', async () => {
      const mockError = new Error('Test error');
      const mockFn = vi.fn().mockRejectedValue(mockError);

      await expect(withSpinner({ text: 'Processing...' }, mockFn)).rejects.toThrow('Test error');

      expect(mockSpinner.start).toHaveBeenCalled();
      expect(mockSpinner.fail).toHaveBeenCalledWith('Test error');
    });

    it('should use custom fail text when provided', async () => {
      const mockError = new Error('Test error');
      const mockFn = vi.fn().mockRejectedValue(mockError);

      await expect(
        withSpinner({ text: 'Processing...', failText: 'Processing failed!' }, mockFn)
      ).rejects.toThrow('Test error');

      expect(mockSpinner.fail).toHaveBeenCalledWith('Processing failed!');
    });

    it('should handle non-Error failures', async () => {
      const mockFn = vi.fn().mockRejectedValue('string error');

      await expect(withSpinner({ text: 'Processing...' }, mockFn)).rejects.toBe('string error');

      expect(mockSpinner.fail).toHaveBeenCalledWith('Operation failed');
    });
  });

  describe('createSpinner', () => {
    it('should create and start a spinner', () => {
      const spinner = createSpinner('Working...');

      expect(ora).toHaveBeenCalledWith('Working...');
      expect(mockSpinner.start).toHaveBeenCalled();
      expect(spinner).toBe(mockSpinner);
    });
  });
});
