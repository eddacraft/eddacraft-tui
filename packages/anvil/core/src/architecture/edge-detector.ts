/**
 * Edge detector for architecture boundary analysis
 *
 * Extracts import edges from source files and compares them against
 * the baseline to identify NEW vs existing violations.
 *
 * **Limitations**:
 * - Import extraction is line-based regex. Multi-line imports split across
 *   lines (e.g., imports with many named exports) may not be detected.
 * - For accurate detection, use TypeScript compiler API in production.
 */

import { readFileSync } from 'node:fs';
import { join, dirname, normalize, extname } from 'node:path';
import { createHash } from 'node:crypto';
import type { DependencyEdge, BaselineViolation } from './types.js';
import { createViolationId } from './types.js';
import { extractHtmlEdges, extractCssEdges } from './edge-detector-web.js';

/**
 * Import edge extracted from source code
 */
export interface ImportEdge {
  /** Source file path (relative to workspace) */
  from: string;
  /** Target file path (resolved, relative to workspace) - used for baseline comparison */
  to: string;
  /** Line number of import statement (1-based) */
  line: number;
  /** Type of import */
  type: 'import' | 'require' | 'dynamic';
  /** Raw import specifier as written in code */
  specifier: string;
}

/**
 * Result of baseline comparison
 */
export interface BaselineComparison {
  /** Edges that exist in baseline (existing violations) */
  existing: ImportEdge[];
  /** Edges NOT in baseline (new violations) */
  new: ImportEdge[];
  /** Edges in baseline but not in current (fixed violations) */
  fixed: BaselineViolation[];
}

/**
 * Options for import extraction
 */
export interface ExtractOptions {
  /** Include dynamic imports (default: true) */
  includeDynamic?: boolean;
  /** Include require() calls (default: true) */
  includeRequire?: boolean;
}

const IMPORT_FROM_REGEX = /import\s+(?:[\w\s{},*]+\s+from\s+)?['"]([^'"]+)['"]/g;
const DYNAMIC_IMPORT_REGEX = /import\s*\(\s*['"]([^'"]+)['"]\s*\)/g;
const REQUIRE_REGEX = /require\s*\(\s*['"]([^'"]+)['"]\s*\)/g;
const EXPORT_FROM_REGEX = /export\s+(?:[\w\s{},*]+\s+from\s+)['"]([^'"]+)['"]/g;

/**
 * Resolve an import specifier to a workspace-relative path
 *
 * Relative imports (./foo, ../bar) are resolved against the importing file.
 * Absolute/package imports are returned as-is (external dependencies).
 */
export function resolveImportPath(specifier: string, fromFile: string): string {
  if (specifier.startsWith('.')) {
    const fromDir = dirname(fromFile);
    const resolved = normalize(join(fromDir, specifier));
    return resolved.replace(/\\/g, '/');
  }
  return specifier;
}

/**
 * Create a stable fingerprint for an edge
 *
 * Used for deduplication and baseline comparison.
 * Format: SHA-256 hash of "from:to:line"
 */
export function createEdgeFingerprint(from: string, to: string, line: number): string {
  const input = `${from}:${to}:${line}`;
  return createHash('sha256').update(input).digest('hex').slice(0, 16);
}

/**
 * Create a fingerprint for an ImportEdge
 */
export function fingerprintEdge(edge: ImportEdge): string {
  return createEdgeFingerprint(edge.from, edge.to, edge.line);
}

/**
 * Extract all import edges from a source file
 *
 * @param filePath - Path to source file (relative to workspace)
 * @param workspaceRoot - Workspace root directory
 * @param options - Extraction options
 * @returns Array of import edges found in the file
 */
export function extractImports(
  filePath: string,
  workspaceRoot: string,
  options: ExtractOptions = {}
): ImportEdge[] {
  const { includeDynamic = true, includeRequire = true } = options;
  const edges: ImportEdge[] = [];

  let content: string;
  try {
    const fullPath = workspaceRoot ? join(workspaceRoot, filePath) : filePath;
    content = readFileSync(fullPath, 'utf-8');
  } catch {
    return edges;
  }

  // Delegate to HTML/CSS extractors based on file extension
  const ext = extname(filePath).toLowerCase();
  if (ext === '.html' || ext === '.htm') {
    return extractHtmlEdges(filePath, content);
  }
  if (ext === '.css' || ext === '.scss' || ext === '.less') {
    return extractCssEdges(filePath, content);
  }

  const lines = content.split('\n');

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const lineNumber = i + 1;

    for (const match of line.matchAll(IMPORT_FROM_REGEX)) {
      const specifier = match[1];
      edges.push({
        from: filePath,
        to: resolveImportPath(specifier, filePath),
        line: lineNumber,
        type: 'import',
        specifier,
      });
    }

    for (const match of line.matchAll(EXPORT_FROM_REGEX)) {
      const specifier = match[1];
      edges.push({
        from: filePath,
        to: resolveImportPath(specifier, filePath),
        line: lineNumber,
        type: 'import',
        specifier,
      });
    }

    if (includeDynamic) {
      for (const match of line.matchAll(DYNAMIC_IMPORT_REGEX)) {
        const specifier = match[1];
        edges.push({
          from: filePath,
          to: resolveImportPath(specifier, filePath),
          line: lineNumber,
          type: 'dynamic',
          specifier,
        });
      }
    }

    if (includeRequire) {
      for (const match of line.matchAll(REQUIRE_REGEX)) {
        const specifier = match[1];
        edges.push({
          from: filePath,
          to: resolveImportPath(specifier, filePath),
          line: lineNumber,
          type: 'require',
          specifier,
        });
      }
    }
  }

  return edges;
}

/**
 * Extract imports from multiple files
 *
 * @param filePaths - Array of file paths (relative to workspace)
 * @param workspaceRoot - Workspace root directory
 * @param options - Extraction options
 * @returns Array of all import edges
 */
export function extractImportsFromFiles(
  filePaths: string[],
  workspaceRoot: string,
  options: ExtractOptions = {}
): ImportEdge[] {
  const allEdges: ImportEdge[] = [];

  for (const filePath of filePaths) {
    const edges = extractImports(filePath, workspaceRoot, options);
    allEdges.push(...edges);
  }

  return allEdges;
}

function edgeToViolationId(edge: ImportEdge): string {
  return createViolationId(edge.from, edge.to, edge.line);
}

/**
 * Compare current edges against baseline violations
 *
 * @param currentEdges - Edges extracted from current code
 * @param baselineViolations - Violations recorded in baseline
 * @returns Comparison result with new, existing, and fixed violations
 */
export function compareToBaseline(
  currentEdges: ImportEdge[],
  baselineViolations: BaselineViolation[]
): BaselineComparison {
  const baselineIds = new Set(baselineViolations.map((v) => v.id));
  const currentIds = new Set(currentEdges.map((e) => edgeToViolationId(e)));

  const existingEdges: ImportEdge[] = [];
  const newEdges: ImportEdge[] = [];

  for (const edge of currentEdges) {
    const violationId = edgeToViolationId(edge);
    if (baselineIds.has(violationId)) {
      existingEdges.push(edge);
    } else {
      newEdges.push(edge);
    }
  }

  const fixedViolations = baselineViolations.filter((v) => !currentIds.has(v.id));

  return {
    existing: existingEdges,
    new: newEdges,
    fixed: fixedViolations,
  };
}

/**
 * Convert ImportEdge to DependencyEdge
 *
 * @param edge - Import edge to convert
 * @param fromLayer - Source layer (null if unknown)
 * @param toLayer - Target layer (null if unknown)
 * @returns DependencyEdge for use in violation tracking
 */
export function toDependencyEdge(
  edge: ImportEdge,
  fromLayer: string | null = null,
  toLayer: string | null = null
): DependencyEdge {
  return {
    from: edge.from,
    to: edge.to,
    from_layer: fromLayer,
    to_layer: toLayer,
    line: edge.line,
    type: edge.type,
  };
}

/**
 * Deduplicate edges by fingerprint
 *
 * @param edges - Array of edges (may contain duplicates)
 * @returns Deduplicated array
 */
export function deduplicateEdges(edges: ImportEdge[]): ImportEdge[] {
  const seen = new Set<string>();
  const unique: ImportEdge[] = [];

  for (const edge of edges) {
    const fingerprint = fingerprintEdge(edge);
    if (!seen.has(fingerprint)) {
      seen.add(fingerprint);
      unique.push(edge);
    }
  }

  return unique;
}

/**
 * Filter edges to only include those crossing layer boundaries
 *
 * @param edges - All import edges
 * @param getLayer - Function to determine layer for a file path
 * @returns Edges that cross layer boundaries
 */
export function filterCrossLayerEdges(
  edges: ImportEdge[],
  getLayer: (filePath: string) => string | null
): ImportEdge[] {
  return edges.filter((edge) => {
    const fromLayer = getLayer(edge.from);
    const toLayer = getLayer(edge.to);

    // Only include if both have layers and they're different
    return fromLayer !== null && toLayer !== null && fromLayer !== toLayer;
  });
}
