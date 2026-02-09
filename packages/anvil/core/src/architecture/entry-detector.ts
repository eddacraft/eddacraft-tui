/**
 * Entry point detection heuristics
 *
 * Detects application entry points from file patterns and exports.
 */

import { existsSync, readFileSync } from 'node:fs';
import { join, basename, dirname } from 'node:path';
import type { EntryPoint, EntryPointType, DetectionConfidence } from './types.js';

/**
 * Entry point pattern with detection rules
 */
interface EntryPointPattern {
  type: EntryPointType;
  filePatterns: RegExp[];
  directoryPatterns: RegExp[];
  confidence: DetectionConfidence;
}

/**
 * Default entry point patterns
 *
 * Order matters: more specific patterns (application, cli, worker) come before
 * generic ones (package). File name matches take priority over directory matches.
 */
const ENTRY_POINT_PATTERNS: EntryPointPattern[] = [
  // Tests (check first - test files should always be identified as tests)
  {
    type: 'test',
    filePatterns: [/\.(test|spec)\.(ts|js)$/, /^test\.(ts|js)$/],
    directoryPatterns: [/^__tests__$/, /^tests?$/, /^spec$/],
    confidence: 'high',
  },
  // Application entries (specific file names)
  {
    type: 'application',
    filePatterns: [
      /^main\.(ts|js|mjs|cjs)$/,
      /^app\.(ts|js|mjs|cjs)$/,
      /^server\.(ts|js|mjs|cjs)$/,
      /^start\.(ts|js|mjs|cjs)$/,
    ],
    directoryPatterns: [/^app$/, /^server$/], // Removed /^src$/ - too generic
    confidence: 'high',
  },
  // CLI commands (specific file names and directories)
  {
    type: 'cli',
    filePatterns: [/^cli\.(ts|js)$/, /^bin\.(ts|js)$/, /^command\.(ts|js)$/],
    directoryPatterns: [/^cli$/, /^bin$/, /^commands$/],
    confidence: 'high',
  },
  // Workers (specific file names and directories)
  {
    type: 'worker',
    filePatterns: [/^worker\.(ts|js)$/, /^job\.(ts|js)$/, /^consumer\.(ts|js)$/],
    directoryPatterns: [/^workers$/, /^jobs$/, /^consumers$/, /^queues$/],
    confidence: 'high',
  },
  // HTTP handlers (specific directories)
  {
    type: 'http',
    filePatterns: [/^routes\.(ts|js)$/, /^router\.(ts|js)$/],
    directoryPatterns: [/^routes$/, /^controllers$/, /^handlers$/],
    confidence: 'high',
  },
  // API handlers (specific directories)
  {
    type: 'api',
    filePatterns: [/^api\.(ts|js)$/, /^handler\.(ts|js)$/],
    directoryPatterns: [/^api$/, /^endpoints$/],
    confidence: 'medium',
  },
  // Package entries (generic - check last)
  {
    type: 'package',
    filePatterns: [/^index\.(ts|js|mjs|cjs)$/],
    directoryPatterns: [], // Removed /^src$/ - too generic, causes false positives
    confidence: 'high',
  },
];

/**
 * Entry point detector
 */
export class EntryPointDetector {
  private workspaceRoot: string;

  constructor(workspaceRoot: string) {
    this.workspaceRoot = workspaceRoot;
  }

  /**
   * Detect entry point type for a file
   *
   * Detection priority:
   * 1. package.json bin entries (highest - explicit CLI declaration)
   * 2. File name matches (specific file names like main.ts, cli.ts)
   * 3. Directory matches (files in specific directories like routes/, workers/)
   * 4. package.json main/exports (fallback for package entries)
   */
  detectEntryPoint(filePath: string): EntryPoint | null {
    const fileName = basename(filePath);
    const relativePath = filePath.startsWith(this.workspaceRoot)
      ? filePath.slice(this.workspaceRoot.length + 1)
      : filePath;

    // Get all directory segments for matching (e.g., src/api/v1 -> ['src', 'api', 'v1'])
    const pathSegments = dirname(relativePath).split(/[/\\]/).filter(Boolean);

    // First: check package.json bin entries (explicit CLI declaration takes priority)
    const binEntry = this.checkPackageJsonBin(relativePath);
    if (binEntry) {
      return binEntry;
    }

    // Second: check file name patterns (highest priority for pattern matching)
    for (const pattern of ENTRY_POINT_PATTERNS) {
      const fileMatch = pattern.filePatterns.some((p) => p.test(fileName));

      if (fileMatch) {
        const dirMatch = pathSegments.some((segment) =>
          pattern.directoryPatterns.some((p) => p.test(segment))
        );
        return {
          path: relativePath,
          type: pattern.type,
          confidence: fileMatch && dirMatch ? 'high' : pattern.confidence,
        };
      }
    }

    // Third: check directory patterns (any segment in path)
    for (const pattern of ENTRY_POINT_PATTERNS) {
      const dirMatch = pathSegments.some((segment) =>
        pattern.directoryPatterns.some((p) => p.test(segment))
      );

      if (dirMatch) {
        return {
          path: relativePath,
          type: pattern.type,
          confidence: pattern.confidence,
        };
      }
    }

    // Fourth: check package.json main/exports (fallback)
    const pkgEntry = this.checkPackageJsonMainExports(relativePath);
    if (pkgEntry) {
      return pkgEntry;
    }

    return null;
  }

  /**
   * Check if file is a bin entry in package.json
   */
  private checkPackageJsonBin(filePath: string): EntryPoint | null {
    const pkgPath = join(this.workspaceRoot, 'package.json');

    if (!existsSync(pkgPath)) {
      return null;
    }

    try {
      const pkg = JSON.parse(readFileSync(pkgPath, 'utf-8'));
      const normalisedPath = filePath.replace(/\\/g, '/');

      if (pkg.bin) {
        const binEntries = typeof pkg.bin === 'string' ? { [pkg.name]: pkg.bin } : pkg.bin;

        for (const binPath of Object.values(binEntries)) {
          const normalisedBin = (binPath as string).replace(/^\.\//, '');
          if (normalisedPath === normalisedBin || normalisedPath.endsWith(normalisedBin)) {
            return {
              path: filePath,
              type: 'cli',
              confidence: 'high',
            };
          }
        }
      }
    } catch {
      // Ignore parse errors
    }

    return null;
  }

  /**
   * Check if file is referenced in package.json main or exports
   */
  private checkPackageJsonMainExports(filePath: string): EntryPoint | null {
    const pkgPath = join(this.workspaceRoot, 'package.json');

    if (!existsSync(pkgPath)) {
      return null;
    }

    try {
      const pkg = JSON.parse(readFileSync(pkgPath, 'utf-8'));
      const normalisedPath = filePath.replace(/\\/g, '/');

      // Check main entry
      if (pkg.main) {
        const mainPath = pkg.main.replace(/^\.\//, '');
        if (normalisedPath === mainPath || normalisedPath.endsWith(mainPath)) {
          return {
            path: filePath,
            type: 'package',
            confidence: 'high',
          };
        }
      }

      // Check exports (depth-limited to prevent abuse via deeply nested exports)
      if (pkg.exports) {
        const MAX_EXPORTS_DEPTH = 10;
        const checkExports = (
          exports: Record<string, unknown>,
          prefix = '',
          depth = 0
        ): EntryPoint | null => {
          if (depth >= MAX_EXPORTS_DEPTH) return null;
          for (const [key, value] of Object.entries(exports)) {
            if (typeof value === 'string') {
              const exportPath = value.replace(/^\.\//, '');
              if (normalisedPath === exportPath || normalisedPath.endsWith(exportPath)) {
                return {
                  path: filePath,
                  type: 'package',
                  confidence: 'high',
                  exports: [prefix + key],
                };
              }
            } else if (typeof value === 'object' && value !== null) {
              const result = checkExports(
                value as Record<string, unknown>,
                prefix + key + '/',
                depth + 1
              );
              if (result) return result;
            }
          }
          return null;
        };

        const exportEntry = checkExports(pkg.exports);
        if (exportEntry) return exportEntry;
      }
    } catch {
      // Ignore parse errors
    }

    return null;
  }

  /**
   * Detect all entry points from a list of files
   */
  detectEntryPoints(filePaths: string[]): EntryPoint[] {
    const entryPoints: EntryPoint[] = [];
    const seen = new Set<string>();

    for (const filePath of filePaths) {
      const entry = this.detectEntryPoint(filePath);
      if (entry && !seen.has(entry.path)) {
        seen.add(entry.path);
        entryPoints.push(entry);
      }
    }

    // Sort by confidence and type
    return entryPoints.sort((a, b) => {
      const confOrder = { high: 0, medium: 1, low: 2 };
      const confDiff = confOrder[a.confidence] - confOrder[b.confidence];
      if (confDiff !== 0) return confDiff;

      const typeOrder: Record<EntryPointType, number> = {
        package: 0,
        application: 1,
        http: 2,
        api: 3,
        cli: 4,
        worker: 5,
        test: 6,
        unknown: 7,
      };
      return typeOrder[a.type] - typeOrder[b.type];
    });
  }

  /**
   * Filter to only non-test entry points
   */
  filterNonTestEntryPoints(entryPoints: EntryPoint[]): EntryPoint[] {
    return entryPoints.filter((e) => e.type !== 'test');
  }
}

/**
 * Create an entry point detector
 */
export function createEntryPointDetector(workspaceRoot: string): EntryPointDetector {
  return new EntryPointDetector(workspaceRoot);
}
