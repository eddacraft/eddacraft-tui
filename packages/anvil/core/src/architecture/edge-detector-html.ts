/**
 * HTML/CSS Edge Detector
 *
 * Extracts dependency edges from HTML and CSS files.
 * Detects:
 * - <script src="..."> references
 * - <link href="..."> stylesheet references
 * - CSS @import url("...") references
 * - CSS url() references
 *
 * External URLs (http/https, data:, //) are skipped per D-002.
 *
 * @module architecture/edge-detector-html
 */

import type { ImportEdge } from './edge-detector.js';
import { resolveImportPath } from './edge-detector.js';

// HTML regexes
const SCRIPT_SRC_REGEX = /<script[^>]+src\s*=\s*["']([^"']+)["']/g;
const LINK_HREF_REGEX = /<link[^>]+href\s*=\s*["']([^"']+)["']/g;

// CSS regexes
const CSS_IMPORT_REGEX = /@import\s+(?:url\(\s*)?["']([^"']+)["']/g;
const CSS_URL_REGEX = /url\(\s*["']?([^"')]+)["']?\s*\)/g;

/**
 * Check if a URL is external (http/https, data:, or protocol-relative //)
 */
function isExternalUrl(url: string): boolean {
  return (
    url.startsWith('http://') ||
    url.startsWith('https://') ||
    url.startsWith('data:') ||
    url.startsWith('//')
  );
}

/**
 * Check if a <link> tag is a stylesheet reference
 * Matches rel="stylesheet" or .css extension in href
 */
function isStylesheetLink(fullTag: string, href: string): boolean {
  return /rel\s*=\s*["']stylesheet["']/i.test(fullTag) || href.endsWith('.css');
}

/**
 * Extract dependency edges from HTML file content
 *
 * @param filePath - Path to HTML file (relative to workspace)
 * @param content - File content
 * @returns Array of import edges found in the file
 */
export function extractHtmlEdges(filePath: string, content: string): ImportEdge[] {
  const edges: ImportEdge[] = [];
  const lines = content.split('\n');

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const lineNumber = i + 1;

    // <script src="...">
    let match: RegExpExecArray | null;
    const scriptRegex = new RegExp(SCRIPT_SRC_REGEX.source, 'g');
    while ((match = scriptRegex.exec(line)) !== null) {
      const specifier = match[1];
      if (!isExternalUrl(specifier)) {
        edges.push({
          from: filePath,
          to: resolveImportPath(specifier, filePath),
          line: lineNumber,
          type: 'import',
          specifier,
        });
      }
    }

    // <link href="..."> (stylesheet only)
    const linkRegex = new RegExp(LINK_HREF_REGEX.source, 'g');
    while ((match = linkRegex.exec(line)) !== null) {
      const specifier = match[1];
      if (!isExternalUrl(specifier) && isStylesheetLink(match[0], specifier)) {
        edges.push({
          from: filePath,
          to: resolveImportPath(specifier, filePath),
          line: lineNumber,
          type: 'import',
          specifier,
        });
      }
    }
  }

  return edges;
}

/**
 * Extract dependency edges from CSS file content
 *
 * @param filePath - Path to CSS file (relative to workspace)
 * @param content - File content
 * @returns Array of import edges found in the file
 */
export function extractCssEdges(filePath: string, content: string): ImportEdge[] {
  const edges: ImportEdge[] = [];
  const lines = content.split('\n');

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const lineNumber = i + 1;

    // @import url("...") or @import "..."
    let match: RegExpExecArray | null;
    const importRegex = new RegExp(CSS_IMPORT_REGEX.source, 'g');
    while ((match = importRegex.exec(line)) !== null) {
      const specifier = match[1];
      if (!isExternalUrl(specifier)) {
        edges.push({
          from: filePath,
          to: resolveImportPath(specifier, filePath),
          line: lineNumber,
          type: 'import',
          specifier,
        });
      }
    }

    // url() references (skip external and data: URIs)
    const urlRegex = new RegExp(CSS_URL_REGEX.source, 'g');
    while ((match = urlRegex.exec(line)) !== null) {
      const specifier = match[1];
      // Skip if already captured by @import, external, or data: URI
      if (!isExternalUrl(specifier) && !line.includes('@import')) {
        edges.push({
          from: filePath,
          to: resolveImportPath(specifier, filePath),
          line: lineNumber,
          type: 'import',
          specifier,
        });
      }
    }
  }

  return edges;
}
