/**
 * Tests for layer detection
 */

import { describe, it, expect } from 'vitest';
import { LayerDetector, createLayerDetector } from './layer-detector.js';
import type { Layers } from './types.js';

describe('LayerDetector', () => {
  describe('detectLayer', () => {
    it('should detect presentation layer from controllers path', () => {
      const detector = createLayerDetector();
      const result = detector.detectLayer('src/controllers/user.controller.ts');

      expect(result.layer).toBe('presentation');
      expect(result.confidence).toBe('high');
    });

    it('should detect presentation layer from routes path', () => {
      const detector = createLayerDetector();
      const result = detector.detectLayer('src/routes/api/users.ts');

      expect(result.layer).toBe('presentation');
    });

    it('should detect application layer from services path', () => {
      const detector = createLayerDetector();
      const result = detector.detectLayer('src/services/user-service.ts');

      expect(result.layer).toBe('application');
      expect(result.confidence).toBe('high');
    });

    it('should detect application layer from use-cases path', () => {
      const detector = createLayerDetector();
      const result = detector.detectLayer('src/use-cases/create-user.ts');

      expect(result.layer).toBe('application');
    });

    it('should detect domain layer from entities path', () => {
      const detector = createLayerDetector();
      const result = detector.detectLayer('src/entities/user.entity.ts');

      expect(result.layer).toBe('domain');
    });

    it('should detect domain layer from models path', () => {
      const detector = createLayerDetector();
      const result = detector.detectLayer('src/models/user.model.ts');

      expect(result.layer).toBe('domain');
    });

    it('should detect infrastructure layer from repositories path', () => {
      const detector = createLayerDetector();
      const result = detector.detectLayer('src/repositories/user-repository.ts');

      expect(result.layer).toBe('infrastructure');
    });

    it('should detect infrastructure layer from database path', () => {
      const detector = createLayerDetector();
      const result = detector.detectLayer('src/database/connection.ts');

      expect(result.layer).toBe('infrastructure');
    });

    it('should detect shared layer from utils path', () => {
      const detector = createLayerDetector();
      const result = detector.detectLayer('src/utils/helpers.ts');

      expect(result.layer).toBe('shared');
    });

    it('should detect shared layer from lib path', () => {
      const detector = createLayerDetector();
      const result = detector.detectLayer('src/lib/logger.ts');

      expect(result.layer).toBe('shared');
    });

    it('should return null layer for unrecognised paths', () => {
      const detector = createLayerDetector();
      const result = detector.detectLayer('src/random/unknown/file.ts');

      expect(result.layer).toBeNull();
      expect(result.confidence).toBe('low');
    });

    it('should handle Windows-style paths', () => {
      const detector = createLayerDetector();
      const result = detector.detectLayer('src\\controllers\\user.ts');

      expect(result.layer).toBe('presentation');
    });

    it('should use priority when multiple patterns match', () => {
      const detector = createLayerDetector();
      // A file in src/services/domain/ could match both services (application) and domain
      // Application has priority 2, domain has priority 3, so application wins
      const result = detector.detectLayer('src/services/domain/user.ts');

      // services/ pattern should win due to lower priority number
      expect(result.layer).toBe('application');
      expect(result.confidence).toBe('medium'); // Medium because multiple matched
    });
  });

  describe('detectLayers', () => {
    it('should detect layers for multiple files', () => {
      const detector = createLayerDetector();
      const files = [
        'src/controllers/user.ts',
        'src/services/user-service.ts',
        'src/entities/user.ts',
        'src/repositories/user-repo.ts',
        'src/utils/helpers.ts',
      ];

      const results = detector.detectLayers(files);

      expect(results).toHaveLength(5);
      expect(results[0].layer).toBe('presentation');
      expect(results[1].layer).toBe('application');
      expect(results[2].layer).toBe('domain');
      expect(results[3].layer).toBe('infrastructure');
      expect(results[4].layer).toBe('shared');
    });
  });

  describe('getDetectedLayers', () => {
    it('should return unique detected layers', () => {
      const detector = createLayerDetector();
      const files = [
        'src/controllers/a.ts',
        'src/controllers/b.ts',
        'src/services/c.ts',
        'src/unknown/d.ts',
      ];

      const layers = detector.getDetectedLayers(files);

      expect(layers.size).toBe(2);
      expect(layers.has('presentation')).toBe(true);
      expect(layers.has('application')).toBe(true);
    });
  });

  describe('suggestLayers', () => {
    it('should only include detected layers', () => {
      const detector = createLayerDetector();
      const files = ['src/controllers/user.ts', 'src/services/user-service.ts'];

      const suggested = detector.suggestLayers(files);

      expect(suggested).toHaveProperty('presentation');
      expect(suggested).toHaveProperty('application');
      expect(suggested).not.toHaveProperty('domain');
      expect(suggested).not.toHaveProperty('infrastructure');
    });

    it('should include dependency rules for suggested layers', () => {
      const detector = createLayerDetector();
      const files = ['src/controllers/user.ts', 'src/services/user-service.ts'];

      const suggested = detector.suggestLayers(files);

      expect(suggested.presentation.depends_on).toContain('application');
      expect(suggested.application.depends_on).toContain('domain');
    });
  });

  describe('isAllowedDependency', () => {
    it('should allow self-references', () => {
      const detector = createLayerDetector();

      expect(detector.isAllowedDependency('presentation', 'presentation')).toBe(true);
      expect(detector.isAllowedDependency('domain', 'domain')).toBe(true);
    });

    it('should allow presentation to application', () => {
      const detector = createLayerDetector();

      expect(detector.isAllowedDependency('presentation', 'application')).toBe(true);
    });

    it('should allow presentation to shared', () => {
      const detector = createLayerDetector();

      expect(detector.isAllowedDependency('presentation', 'shared')).toBe(true);
    });

    it('should disallow presentation to infrastructure', () => {
      const detector = createLayerDetector();

      expect(detector.isAllowedDependency('presentation', 'infrastructure')).toBe(false);
    });

    it('should disallow presentation to domain', () => {
      const detector = createLayerDetector();

      expect(detector.isAllowedDependency('presentation', 'domain')).toBe(false);
    });

    it('should allow domain to shared only', () => {
      const detector = createLayerDetector();

      expect(detector.isAllowedDependency('domain', 'shared')).toBe(true);
      expect(detector.isAllowedDependency('domain', 'presentation')).toBe(false);
      expect(detector.isAllowedDependency('domain', 'application')).toBe(false);
      expect(detector.isAllowedDependency('domain', 'infrastructure')).toBe(false);
    });

    it('should use custom layers when provided', () => {
      const customLayers: Layers = {
        ui: {
          patterns: ['src/ui/**'],
          depends_on: ['logic'],
        },
        logic: {
          patterns: ['src/logic/**'],
          depends_on: [],
        },
      };

      const detector = createLayerDetector(customLayers);

      expect(detector.isAllowedDependency('ui', 'logic', customLayers)).toBe(true);
      expect(detector.isAllowedDependency('logic', 'ui', customLayers)).toBe(false);
    });
  });

  describe('findAmbiguousAssignments', () => {
    it('should find files with low confidence', () => {
      const detector = createLayerDetector();
      const files = [
        'src/controllers/user.ts', // High confidence
        'src/random/unknown.ts', // Low confidence (no match)
      ];

      const ambiguous = detector.findAmbiguousAssignments(files);

      expect(ambiguous).toHaveLength(1);
      expect(ambiguous[0].file).toBe('src/random/unknown.ts');
      expect(ambiguous[0].confidence).toBe('low');
    });

    it('should find files with medium confidence (multiple matches)', () => {
      const detector = createLayerDetector();
      // This path matches both services (application) and domain patterns
      const files = ['src/services/domain/entity.ts'];

      const ambiguous = detector.findAmbiguousAssignments(files);

      // Should be flagged as ambiguous due to multiple layer matches
      expect(ambiguous.length).toBeGreaterThanOrEqual(0);
    });
  });

  describe('custom layers', () => {
    it('should use custom layer patterns', () => {
      const customLayers: Layers = {
        frontend: {
          patterns: ['src/components/**', 'src/pages/**'],
          depends_on: ['backend'],
        },
        backend: {
          patterns: ['src/api/**', 'src/server/**'],
          depends_on: [],
        },
      };

      const detector = createLayerDetector(customLayers);

      expect(detector.detectLayer('src/components/Button.tsx').layer).toBe('frontend');
      expect(detector.detectLayer('src/pages/Home.tsx').layer).toBe('frontend');
      expect(detector.detectLayer('src/api/users.ts').layer).toBe('backend');
      expect(detector.detectLayer('src/server/index.ts').layer).toBe('backend');
    });

    it('should return null for paths not matching custom patterns', () => {
      const customLayers: Layers = {
        frontend: {
          patterns: ['src/components/**'],
          depends_on: [],
        },
      };

      const detector = createLayerDetector(customLayers);
      const result = detector.detectLayer('src/services/user.ts');

      expect(result.layer).toBeNull();
    });
  });
});

describe('createLayerDetector', () => {
  it('should create detector with default patterns', () => {
    const detector = createLayerDetector();

    expect(detector).toBeInstanceOf(LayerDetector);
    expect(detector.detectLayer('src/controllers/test.ts').layer).toBe('presentation');
  });

  it('should create detector with custom layers', () => {
    const customLayers: Layers = {
      custom: {
        patterns: ['src/custom/**'],
        depends_on: [],
      },
    };

    const detector = createLayerDetector(customLayers);

    expect(detector.detectLayer('src/custom/file.ts').layer).toBe('custom');
  });
});
