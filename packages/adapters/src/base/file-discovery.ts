/**
 * File Discovery Utility
 *
 * Discovers planning documents in repositories by searching for common patterns.
 */

import { readdir, stat } from 'node:fs/promises';
import { join, basename } from 'node:path';

/**
 * Discovered planning file
 */
export interface DiscoveredFile {
  /** Full path to file */
  path: string;
  /** File name */
  name: string;
  /** File size in bytes */
  size: number;
  /** Last modified timestamp */
  modified: Date;
  /** Confidence score that this is a planning document (0-100) */
  confidence: number;
  /** Reason for detection */
  reason: string;
}

/**
 * Search options
 */
export interface SearchOptions {
  /** Root directory to search from */
  rootPath: string;
  /** Maximum depth to search */
  maxDepth?: number;
  /** Directories to exclude */
  excludeDirs?: string[];
  /** File patterns to search for */
  patterns?: string[];
}

/**
 * Default directories to exclude
 */
const DEFAULT_EXCLUDE_DIRS = [
  'node_modules',
  '.git',
  'dist',
  'build',
  'coverage',
  '.next',
  '.nuxt',
  'out',
  'target',
  'vendor',
];

/**
 * Default file patterns for planning documents
 */
const DEFAULT_PATTERNS = [
  'prd',
  'plan',
  'todo',
  'tasks',
  'spec',
  'requirements',
  'rfc',
  'adr',
  'design',
  'proposal',
  'roadmap',
];

/**
 * Calculate confidence score for a file name
 */
function calculateFileConfidence(filename: string): { confidence: number; reason: string } {
  const lower = filename.toLowerCase();
  let confidence = 0;
  const reasons: string[] = [];

  // Exact matches (high confidence)
  const exactMatches = ['prd.md', 'plan.md', 'todo.md', 'tasks.md', 'spec.md', 'requirements.md'];
  if (exactMatches.some((pattern) => lower === pattern)) {
    confidence += 80;
    reasons.push('exact-match');
  }

  // Pattern matches (medium-high confidence)
  if (lower.includes('prd')) {
    confidence += 60;
    reasons.push('prd-pattern');
  }
  if (lower.includes('plan')) {
    confidence += 55;
    reasons.push('plan-pattern');
  }
  if (lower.includes('todo')) {
    confidence += 50;
    reasons.push('todo-pattern');
  }
  if (lower.includes('spec')) {
    confidence += 50;
    reasons.push('spec-pattern');
  }
  if (lower.includes('requirements')) {
    confidence += 55;
    reasons.push('requirements-pattern');
  }
  if (lower.includes('task')) {
    confidence += 45;
    reasons.push('task-pattern');
  }
  if (lower.includes('rfc')) {
    confidence += 50;
    reasons.push('rfc-pattern');
  }
  if (lower.includes('adr')) {
    confidence += 50;
    reasons.push('adr-pattern');
  }
  if (lower.includes('design')) {
    confidence += 40;
    reasons.push('design-pattern');
  }
  if (lower.includes('proposal')) {
    confidence += 45;
    reasons.push('proposal-pattern');
  }

  // Check for common planning directories
  if (lower.includes('docs/') || lower.includes('/docs/')) {
    confidence += 10;
    reasons.push('in-docs-dir');
  }
  if (lower.includes('.anvil/') || lower.includes('/.anvil/')) {
    confidence += 15;
    reasons.push('in-anvil-dir');
  }

  // Markdown extension
  if (lower.endsWith('.md') || lower.endsWith('.markdown')) {
    confidence += 5;
    reasons.push('markdown');
  }

  return {
    confidence: Math.min(100, confidence),
    reason: reasons.join(', ') || 'no-match',
  };
}

/**
 * Search directory recursively for planning files
 */
async function searchDirectory(
  dirPath: string,
  options: Required<SearchOptions>,
  currentDepth: number = 0,
  results: DiscoveredFile[] = []
): Promise<DiscoveredFile[]> {
  // Stop if max depth reached
  if (currentDepth > options.maxDepth) {
    return results;
  }

  try {
    const entries = await readdir(dirPath, { withFileTypes: true });

    for (const entry of entries) {
      const fullPath = join(dirPath, entry.name);

      if (entry.isDirectory()) {
        // Skip excluded directories
        if (options.excludeDirs.includes(entry.name)) {
          continue;
        }

        // Recurse into directory
        await searchDirectory(fullPath, options, currentDepth + 1, results);
      } else if (entry.isFile()) {
        // Check if file matches patterns
        const lower = entry.name.toLowerCase();
        const matchesPattern = options.patterns.some((pattern) => lower.includes(pattern));

        if (matchesPattern && (lower.endsWith('.md') || lower.endsWith('.markdown'))) {
          try {
            const stats = await stat(fullPath);
            const { confidence, reason } = calculateFileConfidence(fullPath);

            if (confidence >= 40) {
              // Threshold for inclusion
              results.push({
                path: fullPath,
                name: entry.name,
                size: stats.size,
                modified: stats.mtime,
                confidence,
                reason,
              });
            }
          } catch (statError) {
            console.debug(`[FileDiscovery] Failed to stat file ${fullPath}:`, statError);
          }
        }
      }
    }
  } catch (readError) {
    console.debug(`[FileDiscovery] Failed to read directory ${dirPath}:`, readError);
  }

  return results;
}

/**
 * Discover planning documents in a directory
 *
 * Searches for common planning document patterns like prd.md, plan.md, todo.md, etc.
 *
 * @param options - Search options
 * @returns Array of discovered files, sorted by confidence
 */
export async function discoverPlanningFiles(options: SearchOptions): Promise<DiscoveredFile[]> {
  const searchOptions: Required<SearchOptions> = {
    rootPath: options.rootPath,
    maxDepth: options.maxDepth ?? 5,
    excludeDirs: options.excludeDirs ?? DEFAULT_EXCLUDE_DIRS,
    patterns: options.patterns ?? DEFAULT_PATTERNS,
  };

  const results = await searchDirectory(searchOptions.rootPath, searchOptions);

  // Sort by confidence (descending), then by modified date (newest first)
  return results.sort((a, b) => {
    if (a.confidence !== b.confidence) {
      return b.confidence - a.confidence;
    }
    return b.modified.getTime() - a.modified.getTime();
  });
}

/**
 * Find the most likely planning document
 *
 * Returns the single most likely planning document based on confidence and recency.
 *
 * @param options - Search options
 * @returns The most likely planning document, or undefined if none found
 */
export async function findBestPlanningFile(
  options: SearchOptions
): Promise<DiscoveredFile | undefined> {
  const files = await discoverPlanningFiles(options);
  return files[0];
}

/**
 * Group discovered files by name pattern
 *
 * Groups files like "prd.md", "plan.md" etc. for easier selection.
 *
 * @param files - Discovered files
 * @returns Map of pattern to files
 */
export function groupFilesByPattern(files: DiscoveredFile[]): Map<string, DiscoveredFile[]> {
  const groups = new Map<string, DiscoveredFile[]>();

  for (const file of files) {
    const lower = basename(file.name).toLowerCase();

    let pattern = 'other';
    if (lower.includes('prd')) pattern = 'prd';
    else if (lower.includes('plan')) pattern = 'plan';
    else if (lower.includes('todo')) pattern = 'todo';
    else if (lower.includes('spec')) pattern = 'spec';
    else if (lower.includes('requirements')) pattern = 'requirements';
    else if (lower.includes('rfc')) pattern = 'rfc';
    else if (lower.includes('adr')) pattern = 'adr';

    if (!groups.has(pattern)) {
      groups.set(pattern, []);
    }
    groups.get(pattern)!.push(file);
  }

  return groups;
}
