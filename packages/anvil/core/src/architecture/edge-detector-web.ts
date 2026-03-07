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

// HTML attribute extraction helper — uses indexOf with word-boundary check
// to avoid regex backtracking and substring false-matches (e.g. data-src vs src)
function extractAttr(tag: string, attr: string): string | null {
  let start = 0;
  while (start < tag.length) {
    const idx = tag.indexOf(attr, start);
    if (idx === -1) return null;
    // Ensure attr is preceded by whitespace (word boundary for attribute names)
    if (idx > 0 && !/\s/.test(tag[idx - 1])) {
      start = idx + 1;
      continue;
    }
    // Ensure attr is not a prefix of a longer attribute name (e.g. src vs srcFoo)
    const nextChar = tag[idx + attr.length];
    if (nextChar !== undefined && nextChar !== '=' && !/[ \t]/.test(nextChar)) {
      start = idx + attr.length;
      continue;
    }
    const afterAttr = tag.substring(idx + attr.length);
    const m = afterAttr.match(/^[ \t]*=[ \t]*["']([^"']+)["']/);
    return m ? m[1] : null;
  }
  return null;
}

// CSS regexes — use [ \t] instead of \s to prevent ReDoS backtracking
const CSS_IMPORT_REGEX =
  /@import[ \t]+(?:url\([ \t]*(?:["']([^"']+)["']|([^)\s]+))[ \t]*\)|["']([^"']+)["'])/g;
const CSS_URL_REGEX = /url\([ \t]*(?:"([^")+]+)"|'([^')]+)'|([^"'\s)]+))[ \t]*\)/g;

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
  return /rel[ \t]*=[ \t]*["']stylesheet["']/i.test(fullTag) || href.endsWith('.css');
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
  const TAG_OPEN = /<(script|link)\s/gi;
  const TAG_CLOSE = />/g;

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const lineNumber = i + 1;

    let tagMatch: RegExpExecArray | null;
    TAG_OPEN.lastIndex = 0;
    while ((tagMatch = TAG_OPEN.exec(line)) !== null) {
      TAG_CLOSE.lastIndex = tagMatch.index;
      const closeMatch = TAG_CLOSE.exec(line);
      if (!closeMatch) continue;
      const fullTag = line.substring(tagMatch.index, closeMatch.index + 1);
      const tagName = tagMatch[1].toLowerCase();

      if (tagName === 'script') {
        const specifier = extractAttr(fullTag, 'src');
        if (specifier && !isExternalUrl(specifier)) {
          edges.push({
            from: filePath,
            to: resolveImportPath(specifier, filePath),
            line: lineNumber,
            type: 'import',
            specifier,
          });
        }
      } else if (tagName === 'link') {
        const specifier = extractAttr(fullTag, 'href');
        if (specifier && !isExternalUrl(specifier) && isStylesheetLink(fullTag, specifier)) {
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
      const specifier = match[1] ?? match[2] ?? match[3];
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
