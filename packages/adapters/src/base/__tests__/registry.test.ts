/**
 * Adapter Registry Tests
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { AdapterRegistry } from '../registry.js';
import type {
  FormatAdapter,
  DetectionResult,
  AdapterMetadata,
  PathDetectionHint,
} from '../types.js';
import { createDetection } from '../utils.js';

// Mock adapter for testing
class MockAdapter implements FormatAdapter {
  constructor(
    public readonly metadata: AdapterMetadata,
    private detectionResult: DetectionResult = createDetection(true, 100)
  ) {}

  detect(): DetectionResult {
    return this.detectionResult;
  }

  async parse() {
    return { success: true };
  }

  async serialize() {
    return { success: true };
  }

  async validate() {
    return { valid: true, summary: 'Valid' };
  }

  canImport(format: string): boolean {
    return this.metadata.formats.includes(format) || this.metadata.extensions.includes(format);
  }

  canExport(format: string): boolean {
    return this.canImport(format);
  }
}

describe('AdapterRegistry', () => {
  let registry: AdapterRegistry;

  beforeEach(() => {
    AdapterRegistry.resetInstance();
    registry = AdapterRegistry.getInstance();
  });

  afterEach(() => {
    registry.clear();
  });

  describe('Singleton Pattern', () => {
    it('should return same instance', () => {
      const instance1 = AdapterRegistry.getInstance();
      const instance2 = AdapterRegistry.getInstance();
      expect(instance1).toBe(instance2);
    });

    it('should reset instance', () => {
      const instance1 = AdapterRegistry.getInstance();
      AdapterRegistry.resetInstance();
      const instance2 = AdapterRegistry.getInstance();
      expect(instance1).not.toBe(instance2);
    });
  });

  describe('Registration', () => {
    it('should register an adapter', () => {
      const adapter = new MockAdapter({
        name: 'test',
        version: '1.0.0',
        displayName: 'Test Adapter',
        description: 'Test adapter',
        extensions: ['.test'],
        formats: ['test'],
      });

      registry.register(adapter);
      expect(registry.size).toBe(1);
      expect(registry.getAdapter('test')).toBe(adapter);
    });

    it('should throw error when registering duplicate adapter', () => {
      const adapter1 = new MockAdapter({
        name: 'test',
        version: '1.0.0',
        displayName: 'Test',
        description: 'Test',
        extensions: [],
        formats: [],
      });

      const adapter2 = new MockAdapter({
        name: 'test',
        version: '2.0.0',
        displayName: 'Test 2',
        description: 'Test 2',
        extensions: [],
        formats: [],
      });

      registry.register(adapter1);
      expect(() => registry.register(adapter2)).toThrow("Adapter 'test' is already registered");
    });

    it('should unregister an adapter', () => {
      const adapter = new MockAdapter({
        name: 'test',
        version: '1.0.0',
        displayName: 'Test',
        description: 'Test',
        extensions: [],
        formats: [],
      });

      registry.register(adapter);
      expect(registry.size).toBe(1);

      const removed = registry.unregister('test');
      expect(removed).toBe(true);
      expect(registry.size).toBe(0);
    });

    it('should return false when unregistering non-existent adapter', () => {
      const removed = registry.unregister('nonexistent');
      expect(removed).toBe(false);
    });
  });

  describe('Lookup', () => {
    beforeEach(() => {
      registry.register(
        new MockAdapter({
          name: 'markdown',
          version: '1.0.0',
          displayName: 'Markdown',
          description: 'Markdown adapter',
          extensions: ['.md', '.markdown'],
          formats: ['markdown', 'md'],
        })
      );

      registry.register(
        new MockAdapter({
          name: 'json',
          version: '1.0.0',
          displayName: 'JSON',
          description: 'JSON adapter',
          extensions: ['.json'],
          formats: ['json'],
        })
      );
    });

    it('should get adapter by name', () => {
      const adapter = registry.getAdapter('markdown');
      expect(adapter).toBeDefined();
      expect(adapter?.metadata.name).toBe('markdown');
    });

    it('should return undefined for unknown adapter', () => {
      const adapter = registry.getAdapter('unknown');
      expect(adapter).toBeUndefined();
    });

    it('should get adapter by format', () => {
      const adapter = registry.getAdapterForFormat('markdown');
      expect(adapter).toBeDefined();
      expect(adapter?.metadata.name).toBe('markdown');
    });

    it('should get adapter by extension', () => {
      const adapter = registry.getAdapterForFormat('.md');
      expect(adapter).toBeDefined();
      expect(adapter?.metadata.name).toBe('markdown');
    });

    it('should return undefined for unsupported format', () => {
      const adapter = registry.getAdapterForFormat('yaml');
      expect(adapter).toBeUndefined();
    });
  });

  describe('Detection', () => {
    it('should detect adapter from content', () => {
      const highConfAdapter = new MockAdapter(
        {
          name: 'high',
          version: '1.0.0',
          displayName: 'High',
          description: 'High confidence',
          extensions: [],
          formats: ['high'],
        },
        createDetection(true, 90)
      );

      const lowConfAdapter = new MockAdapter(
        {
          name: 'low',
          version: '1.0.0',
          displayName: 'Low',
          description: 'Low confidence',
          extensions: [],
          formats: ['low'],
        },
        createDetection(true, 50)
      );

      registry.register(highConfAdapter);
      registry.register(lowConfAdapter);

      const result = registry.detectAdapter('test content');
      expect(result).toBeDefined();
      expect(result?.adapter.metadata.name).toBe('high');
      expect(result?.detection.confidence).toBe(90);
    });

    it('should respect minimum confidence threshold', () => {
      const lowConfAdapter = new MockAdapter(
        {
          name: 'low',
          version: '1.0.0',
          displayName: 'Low',
          description: 'Low',
          extensions: [],
          formats: [],
        },
        createDetection(true, 40)
      );

      registry.register(lowConfAdapter);

      const result = registry.detectAdapter('test', 50);
      expect(result).toBeUndefined();
    });

    it('should detect all adapters', () => {
      const adapter1 = new MockAdapter(
        {
          name: 'first',
          version: '1.0.0',
          displayName: 'First',
          description: 'First',
          extensions: [],
          formats: [],
        },
        createDetection(true, 80)
      );

      const adapter2 = new MockAdapter(
        {
          name: 'second',
          version: '1.0.0',
          displayName: 'Second',
          description: 'Second',
          extensions: [],
          formats: [],
        },
        createDetection(true, 60)
      );

      registry.register(adapter1);
      registry.register(adapter2);

      const results = registry.detectAll('test');
      expect(results).toHaveLength(2);
      expect(results[0].adapter.metadata.name).toBe('first');
      expect(results[1].adapter.metadata.name).toBe('second');
    });
  });

  describe('Listing', () => {
    beforeEach(() => {
      registry.register(
        new MockAdapter({
          name: 'adapter1',
          version: '1.0.0',
          displayName: 'Adapter 1',
          description: 'First',
          extensions: ['.a1', '.a'],
          formats: ['a1', 'adapter1'],
        })
      );

      registry.register(
        new MockAdapter({
          name: 'adapter2',
          version: '1.0.0',
          displayName: 'Adapter 2',
          description: 'Second',
          extensions: ['.a2'],
          formats: ['a2'],
        })
      );
    });

    it('should list all adapters', () => {
      const adapters = registry.listAdapters();
      expect(adapters).toHaveLength(2);
    });

    it('should list adapter names', () => {
      const names = registry.listAdapterNames();
      expect(names).toEqual(['adapter1', 'adapter2']);
    });

    it('should list supported formats', () => {
      const formats = registry.listSupportedFormats();
      expect(formats).toContain('a1');
      expect(formats).toContain('a2');
      expect(formats).toContain('adapter1');
    });

    it('should list supported extensions', () => {
      const extensions = registry.listSupportedExtensions();
      expect(extensions).toContain('.a1');
      expect(extensions).toContain('.a2');
      expect(extensions).toContain('.a');
    });
  });

  describe('Import/Export Adapters', () => {
    beforeEach(() => {
      registry.register(
        new MockAdapter({
          name: 'markdown',
          version: '1.0.0',
          displayName: 'Markdown',
          description: 'Markdown',
          extensions: ['.md'],
          formats: ['markdown'],
        })
      );

      registry.register(
        new MockAdapter({
          name: 'json',
          version: '1.0.0',
          displayName: 'JSON',
          description: 'JSON',
          extensions: ['.json'],
          formats: ['json'],
        })
      );
    });

    it('should get import adapters for format', () => {
      const adapters = registry.getImportAdapters('markdown');
      expect(adapters).toHaveLength(1);
      expect(adapters[0].metadata.name).toBe('markdown');
    });

    it('should get export adapters for format', () => {
      const adapters = registry.getExportAdapters('.json');
      expect(adapters).toHaveLength(1);
      expect(adapters[0].metadata.name).toBe('json');
    });

    it('should check if format is supported', () => {
      expect(registry.isFormatSupported('markdown')).toBe(true);
      expect(registry.isFormatSupported('yaml')).toBe(false);
    });
  });

  describe('Clear', () => {
    it('should clear all adapters', () => {
      registry.register(
        new MockAdapter({
          name: 'test',
          version: '1.0.0',
          displayName: 'Test',
          description: 'Test',
          extensions: [],
          formats: [],
        })
      );

      expect(registry.size).toBe(1);
      registry.clear();
      expect(registry.size).toBe(0);
    });
  });

  describe('Path-aware Detection (registry)', () => {
    it('should use detectWithPath when available', () => {
      const pathAwareAdapter = new MockAdapter(
        {
          name: 'path-aware',
          version: '1.0.0',
          displayName: 'Path Aware',
          description: 'Has detectWithPath',
          extensions: [],
          formats: [],
        },
        createDetection(true, 60)
      );

      // Add detectWithPath that returns higher confidence
      (pathAwareAdapter as FormatAdapter).detectWithPath = (
        _content: string,
        _hint: PathDetectionHint
      ) => createDetection(true, 90, 'path-boosted');

      registry.register(pathAwareAdapter);

      const hint: PathDetectionHint = {
        filePath: '/project/_bmad/doc.md',
        parentDirs: ['_bmad', 'project'],
      };

      const result = registry.detectAdapterWithPath('test content', hint);
      expect(result).toBeDefined();
      expect(result?.detection.confidence).toBe(90);
      expect(result?.detection.reason).toBe('path-boosted');
    });

    it('should fall back to detect when detectWithPath is not available', () => {
      const standardAdapter = new MockAdapter(
        {
          name: 'standard',
          version: '1.0.0',
          displayName: 'Standard',
          description: 'No detectWithPath',
          extensions: [],
          formats: [],
        },
        createDetection(true, 70)
      );

      registry.register(standardAdapter);

      const hint: PathDetectionHint = {
        filePath: '/project/doc.md',
      };

      const result = registry.detectAdapterWithPath('test content', hint);
      expect(result).toBeDefined();
      expect(result?.detection.confidence).toBe(70);
    });

    it('should select highest confidence from path-aware detection', () => {
      const lowAdapter = new MockAdapter(
        {
          name: 'low-path',
          version: '1.0.0',
          displayName: 'Low',
          description: 'Low confidence',
          extensions: [],
          formats: [],
        },
        createDetection(true, 50)
      );

      const highAdapter = new MockAdapter(
        {
          name: 'high-path',
          version: '1.0.0',
          displayName: 'High',
          description: 'High confidence',
          extensions: [],
          formats: [],
        },
        createDetection(true, 60)
      );

      (highAdapter as FormatAdapter).detectWithPath = (
        _content: string,
        _hint: PathDetectionHint
      ) => createDetection(true, 95, 'path-detected');

      registry.register(lowAdapter);
      registry.register(highAdapter);

      const hint: PathDetectionHint = {
        filePath: '/project/_bmad/doc.md',
        parentDirs: ['_bmad'],
      };

      const result = registry.detectAdapterWithPath('test', hint);
      expect(result?.adapter.metadata.name).toBe('high-path');
      expect(result?.detection.confidence).toBe(95);
    });

    it('should respect minimum confidence threshold for path detection', () => {
      const lowAdapter = new MockAdapter(
        {
          name: 'low-threshold',
          version: '1.0.0',
          displayName: 'Low',
          description: 'Low confidence',
          extensions: [],
          formats: [],
        },
        createDetection(true, 30)
      );

      registry.register(lowAdapter);

      const hint: PathDetectionHint = {
        filePath: '/project/doc.md',
      };

      const result = registry.detectAdapterWithPath('test', hint, 50);
      expect(result).toBeUndefined();
    });
  });
});
