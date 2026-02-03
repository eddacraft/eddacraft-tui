/**
 * Tests for entry point detection
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { existsSync, mkdirSync, writeFileSync, rmSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { EntryPointDetector, createEntryPointDetector } from './entry-detector.js';

describe('EntryPointDetector', () => {
  let testDir: string;
  let detector: EntryPointDetector;

  beforeEach(() => {
    // Create a temporary test directory
    testDir = join(tmpdir(), `anvil-test-${Date.now()}`);
    mkdirSync(testDir, { recursive: true });
    detector = createEntryPointDetector(testDir);
  });

  afterEach(() => {
    // Clean up test directory
    if (existsSync(testDir)) {
      rmSync(testDir, { recursive: true, force: true });
    }
  });

  describe('detectEntryPoint', () => {
    describe('package entries', () => {
      it('should detect index.ts as package entry', () => {
        const result = detector.detectEntryPoint('src/index.ts');

        expect(result).not.toBeNull();
        expect(result?.type).toBe('package');
        expect(result?.confidence).toBe('high');
      });

      it('should detect index.js as package entry', () => {
        const result = detector.detectEntryPoint('src/index.js');

        expect(result).not.toBeNull();
        expect(result?.type).toBe('package');
      });

      it('should detect index.mjs as package entry', () => {
        const result = detector.detectEntryPoint('lib/index.mjs');

        expect(result).not.toBeNull();
        expect(result?.type).toBe('package');
      });
    });

    describe('application entries', () => {
      it('should detect main.ts as application entry', () => {
        const result = detector.detectEntryPoint('src/main.ts');

        expect(result).not.toBeNull();
        expect(result?.type).toBe('application');
        expect(result?.confidence).toBe('high');
      });

      it('should detect app.ts as application entry', () => {
        const result = detector.detectEntryPoint('src/app.ts');

        expect(result).not.toBeNull();
        expect(result?.type).toBe('application');
      });

      it('should detect server.ts as application entry', () => {
        const result = detector.detectEntryPoint('src/server.ts');

        expect(result).not.toBeNull();
        // Could be application or http depending on context
        expect(['application', 'http']).toContain(result?.type);
      });
    });

    describe('HTTP handlers', () => {
      it('should detect files in routes directory', () => {
        const result = detector.detectEntryPoint('src/routes/users.ts');

        expect(result).not.toBeNull();
        expect(result?.type).toBe('http');
      });

      it('should detect files in controllers directory', () => {
        const result = detector.detectEntryPoint('src/controllers/user.controller.ts');

        expect(result).not.toBeNull();
        expect(result?.type).toBe('http');
      });

      it('should detect files in api directory', () => {
        const result = detector.detectEntryPoint('src/api/v1/users.ts');

        expect(result).not.toBeNull();
        expect(['http', 'api']).toContain(result?.type);
      });
    });

    describe('CLI entries', () => {
      it('should detect cli.ts as CLI entry', () => {
        const result = detector.detectEntryPoint('src/cli.ts');

        expect(result).not.toBeNull();
        expect(result?.type).toBe('cli');
      });

      it('should detect files in bin directory', () => {
        const result = detector.detectEntryPoint('bin/anvil.js');

        expect(result).not.toBeNull();
        expect(result?.type).toBe('cli');
      });

      it('should detect files in commands directory', () => {
        const result = detector.detectEntryPoint('src/commands/init.ts');

        expect(result).not.toBeNull();
        expect(result?.type).toBe('cli');
      });
    });

    describe('worker entries', () => {
      it('should detect worker.ts as worker entry', () => {
        const result = detector.detectEntryPoint('src/worker.ts');

        expect(result).not.toBeNull();
        expect(result?.type).toBe('worker');
      });

      it('should detect files in workers directory', () => {
        const result = detector.detectEntryPoint('src/workers/email-worker.ts');

        expect(result).not.toBeNull();
        expect(result?.type).toBe('worker');
      });

      it('should detect files in jobs directory', () => {
        const result = detector.detectEntryPoint('src/jobs/cleanup.ts');

        expect(result).not.toBeNull();
        expect(result?.type).toBe('worker');
      });
    });

    describe('test entries', () => {
      it('should detect .test.ts files as test entries', () => {
        const result = detector.detectEntryPoint('src/utils/helpers.test.ts');

        expect(result).not.toBeNull();
        expect(result?.type).toBe('test');
      });

      it('should detect .spec.ts files as test entries', () => {
        const result = detector.detectEntryPoint('src/services/user.spec.ts');

        expect(result).not.toBeNull();
        expect(result?.type).toBe('test');
      });

      it('should detect files in __tests__ directory', () => {
        const result = detector.detectEntryPoint('src/__tests__/integration.ts');

        expect(result).not.toBeNull();
        expect(result?.type).toBe('test');
      });
    });

    describe('package.json detection', () => {
      it('should detect main entry from package.json', () => {
        // Create package.json
        writeFileSync(
          join(testDir, 'package.json'),
          JSON.stringify({
            name: 'test-package',
            main: 'dist/index.js',
          })
        );

        const result = detector.detectEntryPoint('dist/index.js');

        expect(result).not.toBeNull();
        expect(result?.type).toBe('package');
        expect(result?.confidence).toBe('high');
      });

      it('should detect bin entry from package.json', () => {
        writeFileSync(
          join(testDir, 'package.json'),
          JSON.stringify({
            name: 'test-cli',
            bin: {
              'test-cli': './bin/cli.js',
            },
          })
        );

        const result = detector.detectEntryPoint('bin/cli.js');

        expect(result).not.toBeNull();
        expect(result?.type).toBe('cli');
      });

      it('should detect string bin entry from package.json', () => {
        writeFileSync(
          join(testDir, 'package.json'),
          JSON.stringify({
            name: 'simple-cli',
            bin: './bin/index.js',
          })
        );

        const result = detector.detectEntryPoint('bin/index.js');

        expect(result).not.toBeNull();
        expect(result?.type).toBe('cli');
      });

      it('should detect exports entry from package.json', () => {
        writeFileSync(
          join(testDir, 'package.json'),
          JSON.stringify({
            name: 'test-package',
            exports: {
              '.': './dist/index.js',
              './utils': './dist/utils.js',
            },
          })
        );

        const result = detector.detectEntryPoint('dist/utils.js');

        expect(result).not.toBeNull();
        expect(result?.type).toBe('package');
        expect(result?.exports).toContain('./utils');
      });
    });

    describe('non-entry points', () => {
      it('should return null for regular source files', () => {
        const result = detector.detectEntryPoint('src/utils/helpers.ts');

        expect(result).toBeNull();
      });

      it('should return null for type definition files', () => {
        const result = detector.detectEntryPoint('src/types/user.d.ts');

        expect(result).toBeNull();
      });
    });
  });

  describe('detectEntryPoints', () => {
    it('should detect multiple entry points', () => {
      const files = [
        'src/index.ts',
        'src/main.ts',
        'src/controllers/user.ts',
        'src/utils/helpers.ts',
        'src/cli.ts',
      ];

      const entryPoints = detector.detectEntryPoints(files);

      expect(entryPoints.length).toBeGreaterThanOrEqual(3);

      const types = entryPoints.map((e) => e.type);
      expect(types).toContain('package');
      expect(types).toContain('application');
      expect(types).toContain('cli');
    });

    it('should deduplicate entry points by path', () => {
      const files = ['src/index.ts', 'src/index.ts', 'src/index.ts'];

      const entryPoints = detector.detectEntryPoints(files);

      expect(entryPoints).toHaveLength(1);
    });

    it('should sort by confidence and type', () => {
      const files = [
        'src/workers/job.ts', // worker
        'src/index.ts', // package
        'src/routes/api.ts', // http
      ];

      const entryPoints = detector.detectEntryPoints(files);

      // Package entries should come first
      expect(entryPoints[0].type).toBe('package');
    });
  });

  describe('filterNonTestEntryPoints', () => {
    it('should filter out test entry points', () => {
      const entryPoints = [
        { path: 'src/index.ts', type: 'package' as const, confidence: 'high' as const },
        { path: 'src/index.test.ts', type: 'test' as const, confidence: 'high' as const },
        { path: 'src/main.ts', type: 'application' as const, confidence: 'high' as const },
        { path: 'src/__tests__/int.ts', type: 'test' as const, confidence: 'high' as const },
      ];

      const filtered = detector.filterNonTestEntryPoints(entryPoints);

      expect(filtered).toHaveLength(2);
      expect(filtered.every((e) => e.type !== 'test')).toBe(true);
    });

    it('should return empty array if all are tests', () => {
      const entryPoints = [
        { path: 'src/index.test.ts', type: 'test' as const, confidence: 'high' as const },
        { path: 'src/main.spec.ts', type: 'test' as const, confidence: 'high' as const },
      ];

      const filtered = detector.filterNonTestEntryPoints(entryPoints);

      expect(filtered).toHaveLength(0);
    });
  });
});

describe('createEntryPointDetector', () => {
  it('should create functional detector with workspace root', () => {
    const detector = createEntryPointDetector('/test/workspace');

    expect(detector).toBeInstanceOf(EntryPointDetector);
    // Verify the detector is functional by detecting a known entry point
    const result = detector.detectEntryPoint('/test/workspace/src/index.ts');
    expect(result).not.toBeNull();
    expect(result?.type).toBe('package');
  });
});
