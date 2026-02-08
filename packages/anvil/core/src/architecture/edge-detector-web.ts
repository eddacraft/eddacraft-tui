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
 * @module architecture/edge-detector-web
 */

import type { ImportEdge } from './edge-detector.js';
import { resolveImportPath } from './edge-detector.js';

// HTML regexes
const SCRIPT_SRC_REGEX = /<script[^>]+src\s*=\s*["']([^"']+)["']/g;
const LINK_TAG_REGEX = /<link\s[^>]*>/g;
const HREF_ATTR_REGEX = /href\s*=\s*["']([^"']+)["']/;

// CSS regexes
const CSS_IMPORT_REGEX =
  /@import\s+(?:url\(\s*(?:["']([^"']+)["']|([^)]+?))\s*\)|["']([^"']+)["'])/g;
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
    SCRIPT_SRC_REGEX.lastIndex = 0;
    while ((match = SCRIPT_SRC_REGEX.exec(line)) !== null) {
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
    LINK_TAG_REGEX.lastIndex = 0;
    while ((match = LINK_TAG_REGEX.exec(line)) !== null) {
      const fullTag = match[0];
      const hrefMatch = fullTag.match(HREF_ATTR_REGEX);
      if (hrefMatch) {
        const specifier = hrefMatch[1];
        if (!isExternalUrl(specifier) && isStylesheetLink(fullTag, specifier)) {
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

    // @import url("...") or @import "..." or @import url(...)
    let match: RegExpExecArray | null;
    CSS_IMPORT_REGEX.lastIndex = 0;
    while ((match = CSS_IMPORT_REGEX.exec(line)) !== null) {
      const specifier = (match[1] ?? match[2] ?? match[3])?.trim();
      if (specifier && !isExternalUrl(specifier)) {
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
    CSS_URL_REGEX.lastIndex = 0;
    while ((match = CSS_URL_REGEX.exec(line)) !== null) {
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
