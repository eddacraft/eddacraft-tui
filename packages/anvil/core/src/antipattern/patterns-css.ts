/**
 * CSS Anti-pattern Definitions
 *
 * Detects common CSS anti-patterns:
 * - AP-012: !important usage
 * - AP-013: CSS @import (performance concern)
 *
 * All patterns are opt-in by default (D-001).
 *
 * @module antipattern/patterns-css
 */

import type { AntiPattern } from './types.js';

/**
 * AP-012: !important in CSS
 *
 * Detects usage of `!important` in CSS declarations.
 * Overuse of !important creates specificity wars and makes CSS harder to maintain.
 */
const AP012_IMPORTANT: AntiPattern = {
  id: 'AP-012',
  name: '!important in CSS',
  category: 'css',
  severity: 'warning',
  confidence: 'high',
  detection: {
    type: 'regex',
    pattern: String.raw`!\s*important`,
  },
  title: '!important used in CSS',
  explanation:
    'Using !important overrides all other specificity rules, creating maintenance headaches. ' +
    'It often indicates specificity wars or architectural issues in CSS.',
  suggestion:
    'Increase selector specificity naturally, restructure CSS to avoid conflicts, ' +
    'or use CSS layers (@layer) for better cascade control.',
  fileExtensions: ['.css', '.scss', '.less'],
  allowlist: ['**/reset.css', '**/normalize.css'],
  enabled: true,
  optIn: true,
};

/**
 * AP-013: CSS @import
 *
 * Detects usage of CSS `@import` which causes sequential loading.
 * This is a performance concern, not a bug.
 */
const AP013_CSS_IMPORT: AntiPattern = {
  id: 'AP-013',
  name: 'CSS @import',
  category: 'css',
  severity: 'info',
  confidence: 'high',
  detection: {
    type: 'regex',
    pattern: String.raw`@import\s+(?:url\()?["']`,
  },
  title: 'CSS @import causes sequential loading',
  explanation:
    'CSS @import causes browsers to load stylesheets sequentially rather than in parallel, ' +
    'which increases page load time. Each @import blocks rendering until the imported file loads.',
  suggestion:
    'Use <link> tags in HTML for parallel loading, or use a CSS bundler ' +
    '(PostCSS, Sass, etc.) to inline imports at build time.',
  fileExtensions: ['.css', '.scss', '.less'],
  enabled: true,
  optIn: true,
};

/**
 * All CSS anti-patterns
 */
export const CSS_PATTERNS: readonly AntiPattern[] = [AP012_IMPORTANT, AP013_CSS_IMPORT] as const;
