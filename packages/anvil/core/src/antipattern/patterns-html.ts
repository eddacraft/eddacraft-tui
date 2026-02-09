/**
 * HTML Anti-pattern Definitions
 *
 * Detects common HTML anti-patterns:
 * - AP-008: Inline style attributes
 * - AP-009: Inline script blocks
 * - AP-010: Inline event handlers
 * - AP-011: Deprecated HTML tags
 *
 * All patterns are opt-in by default (D-001).
 * All patterns include email template allowlist since HTML emails
 * legitimately use inline styles and scripts.
 *
 * @module antipattern/patterns-html
 */

import type { AntiPattern } from './types.js';

/**
 * AP-008: Inline style attribute
 *
 * Detects `style="..."` attributes in HTML elements.
 * Inline styles hinder maintainability and prevent caching of CSS.
 */
const AP008_INLINE_STYLE: AntiPattern = {
  id: 'AP-008',
  name: 'Inline style attribute',
  category: 'html',
  severity: 'warning',
  confidence: 'high',
  detection: {
    type: 'regex',
    pattern: String.raw`style\s*=\s*["']`,
  },
  title: 'Inline style attribute found',
  explanation:
    'Inline styles mix presentation with structure, making CSS harder to maintain, ' +
    'override, and cache. They also increase HTML file size.',
  suggestion:
    'Move styles to an external CSS file or use CSS classes. ' +
    'For dynamic styles, use CSS custom properties or a CSS-in-JS solution.',
  nudge:
    "Move this inline style to a CSS class. Inline styles can't be overridden " +
    'by stylesheets, break consistency, and make maintenance harder. Define a ' +
    'class in your stylesheet and apply it instead.',
  fileExtensions: ['.html', '.htm'],
  allowlist: ['**/email/**'],
  enabled: true,
  optIn: true,
};

/**
 * AP-009: Inline script block
 *
 * Detects `<script>` blocks with inline code (not just closing tags).
 * Inline scripts bypass CSP and prevent caching.
 */
const AP009_INLINE_SCRIPT: AntiPattern = {
  id: 'AP-009',
  name: 'Inline script block',
  category: 'html',
  severity: 'warning',
  confidence: 'high',
  detection: {
    type: 'regex',
    pattern: String.raw`<script(?:\s[^>]*)?>(?!\s*<\/script>)`,
  },
  title: 'Inline script block found',
  explanation:
    'Inline scripts bypass Content Security Policy (CSP), prevent browser caching, ' +
    'and make code harder to test and maintain.',
  suggestion:
    'Move JavaScript to external .js files referenced with <script src="...">. ' +
    'This enables caching, CSP compliance, and better separation of concerns.',
  nudge:
    'Move this script to an external `.js` file and reference it with ' +
    '`<script src="...">`. Inline scripts cannot be cached, violate CSP ' +
    'policies, and make code harder to test.',
  fileExtensions: ['.html', '.htm'],
  allowlist: ['**/email/**'],
  enabled: true,
  optIn: true,
};

/**
 * AP-010: Inline event handler
 *
 * Detects `on*="..."` attributes like onclick, onload, etc.
 * Inline handlers mix behavior with structure and bypass CSP.
 */
const AP010_INLINE_EVENT_HANDLER: AntiPattern = {
  id: 'AP-010',
  name: 'Inline event handler',
  category: 'html',
  severity: 'warning',
  confidence: 'high',
  detection: {
    type: 'regex',
    pattern: String.raw`\bon\w+\s*=\s*["']`,
  },
  title: 'Inline event handler found',
  explanation:
    'Inline event handlers (onclick, onload, etc.) mix behavior with HTML structure, ' +
    'bypass CSP, and make code harder to debug and maintain.',
  suggestion:
    'Use addEventListener() in external JavaScript files instead. ' +
    'For frameworks, use the framework event binding syntax.',
  nudge:
    'Remove this inline event handler and use `addEventListener()` in an ' +
    'external script instead. Inline handlers mix behaviour with markup and ' +
    'are blocked by strict Content Security Policies.',
  fileExtensions: ['.html', '.htm'],
  allowlist: ['**/email/**'],
  enabled: true,
  optIn: true,
};

/**
 * AP-011: Deprecated HTML tags
 *
 * Detects usage of deprecated HTML tags like <font>, <center>, <marquee>, etc.
 * These tags are obsolete and should be replaced with CSS.
 */
const AP011_DEPRECATED_TAGS: AntiPattern = {
  id: 'AP-011',
  name: 'Deprecated HTML tag',
  category: 'html',
  severity: 'warning',
  confidence: 'high',
  detection: {
    type: 'regex',
    pattern: String.raw`<(?:font|center|marquee|blink|big|strike)\b`,
  },
  title: 'Deprecated HTML tag used',
  explanation:
    'Deprecated HTML tags like <font>, <center>, and <marquee> are obsolete. ' +
    'They may not render correctly in modern browsers and indicate outdated practices.',
  suggestion:
    'Replace deprecated tags with semantic HTML and CSS. ' +
    'For example, use CSS text-align instead of <center>, and CSS font properties instead of <font>.',
  nudge:
    'Replace this deprecated HTML tag with its modern CSS equivalent. Use CSS ' +
    'for visual presentation instead of presentational HTML elements.',
  fileExtensions: ['.html', '.htm'],
  allowlist: ['**/email/**'],
  enabled: true,
  optIn: true,
};

/**
 * All HTML anti-patterns
 */
export const HTML_PATTERNS: readonly AntiPattern[] = [
  AP008_INLINE_STYLE,
  AP009_INLINE_SCRIPT,
  AP010_INLINE_EVENT_HANDLER,
  AP011_DEPRECATED_TAGS,
] as const;
