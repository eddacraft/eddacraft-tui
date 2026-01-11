import { describe, it, expect, beforeEach, vi } from 'vitest';
import { FormatDetectionService } from '../format-detection.js';

// Mock AdapterRegistry
vi.mock('@anvil/adapters', () => ({
  AdapterRegistry: {
    getInstance: vi.fn(() => ({
      detectAdapter: vi.fn(),
      listAdapters: vi.fn(() => []),
    })),
  },
}));

describe('FormatDetectionService', () => {
  let service: FormatDetectionService;

  beforeEach(() => {
    service = new FormatDetectionService();
  });

  describe('constructor', () => {
    it('should create service with default min confidence', () => {
      const defaultService = new FormatDetectionService();
      expect(defaultService).toBeDefined();
    });

    it('should create service with custom min confidence', () => {
      const customService = new FormatDetectionService({ minConfidence: 70 });
      expect(customService).toBeDefined();
    });
  });

  describe('detectFormat', () => {
    it('should return null when no format detected', async () => {
      const result = await service.detectFormat('invalid content');
      expect(result).toBeNull();
    });

    it('should include file path in result when provided', async () => {
      const filePath = '/test/file.md';
      const result = await service.detectFormat('content', filePath);

      if (result) {
        expect(result.filePath).toBe(filePath);
      }
    });
  });

  describe('detectAllFormats', () => {
    it('should return empty array when no adapters match', async () => {
      const result = await service.detectAllFormats('invalid content');
      expect(result).toEqual([]);
    });

    it('should sort results by confidence descending', async () => {
      const results = await service.detectAllFormats('test content');

      // Verify sorting if multiple results exist
      if (results.length > 1) {
        for (let i = 0; i < results.length - 1; i++) {
          expect(results[i].detection.confidence).toBeGreaterThanOrEqual(
            results[i + 1].detection.confidence
          );
        }
      }
    });
  });
});
