/**
 * BMAD Adapter Utilities
 *
 * Helper functions for BMAD format parsing and serialization.
 */

import {
  BMADFrontMatter,
  BMADRequirement,
  BMADUserStory,
  BMADChangeLogEntry,
  RequirementType,
  BMADDocumentType,
  DetectionIndicators,
  BMAD_FOLDERS,
  type BMADAgentYaml,
  type BMADAgentMetadata,
  type BMADAgentPersona,
  type BMADMenuItem,
  type BMADAgentPrompt,
  type BMADWorkflowYaml,
  type BMADTeamYaml,
  type BMADModuleYaml,
} from './types.js';
import type { PathDetectionHint } from '../base/types.js';

/**
 * Analyze a file path for BMAD folder structure indicators
 *
 * Detects both v6 (`_bmad`, `_config`) and legacy (`.bmad`, `_cfg`) paths.
 *
 * @param hint - Path detection hint with file path and directory info
 * @returns Object indicating which BMAD path patterns were found
 */
export function analyzePath(hint: PathDetectionHint): {
  isBmadFolder: boolean;
  isConfigFolder: boolean;
} {
  const { filePath, parentDirs } = hint;
  const normalizedPath = filePath.replace(/\\/g, '/');
  const allDirs = parentDirs ?? [];

  const bmadDirs: readonly string[] = [BMAD_FOLDERS.PROJECT, BMAD_FOLDERS.PROJECT_LEGACY];
  const configDirs: readonly string[] = [BMAD_FOLDERS.CONFIG, BMAD_FOLDERS.CONFIG_LEGACY];

  const isBmadFolder =
    bmadDirs.some((d) => {
      const pattern = new RegExp(`(?:^|/)${escapeRegExp(d)}(?:/|$)`);
      return pattern.test(normalizedPath);
    }) || allDirs.some((d) => bmadDirs.includes(d));

  const isConfigFolder =
    configDirs.some((d) => {
      const pattern = new RegExp(`(?:^|/)${escapeRegExp(d)}(?:/|$)`);
      return pattern.test(normalizedPath);
    }) || allDirs.some((d) => configDirs.includes(d));

  return { isBmadFolder, isConfigFolder };
}

/**
 * Expand BMAD template variables in content
 *
 * Supports both legacy underscore syntax `{project_root}` and
 * v6 hyphenated syntax `{project-root}`.
 *
 * @param content - Content with variable placeholders
 * @param variables - Variable values to substitute
 * @returns Content with variables expanded
 */
export function expandVariables(content: string, variables: Record<string, string>): string {
  let result = content;

  for (const [key, value] of Object.entries(variables)) {
    // Support both underscore and hyphenated forms
    const underscoreKey = key.replace(/-/g, '_');
    const hyphenKey = key.replace(/_/g, '-');

    result = result
      .replace(new RegExp(`\\{${escapeRegExp(underscoreKey)}\\}`, 'g'), () => value)
      .replace(new RegExp(`\\{${escapeRegExp(hyphenKey)}\\}`, 'g'), () => value);
  }

  return result;
}

/**
 * Check if content contains hyphenated variable syntax (v6)
 *
 * @param content - Content to check
 * @returns True if hyphenated variables are found
 */
export function hasHyphenatedVariables(content: string): boolean {
  return /\{[a-z]+-[a-z]+(?:-[a-z]+)*\}/i.test(content);
}

function escapeRegExp(str: string): string {
  return str.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

/**
 * Parse a YAML boolean value
 *
 * Handles YAML 1.1 boolean forms: true/false, yes/no, on/off.
 *
 * @param value - String value from YAML
 * @returns Boolean or undefined if not a boolean value
 */
export function parseYamlBoolean(value: string): boolean | undefined {
  const lower = value.toLowerCase().trim();
  if (['true', 'yes', 'on'].includes(lower)) return true;
  if (['false', 'no', 'off'].includes(lower)) return false;
  return undefined;
}

/**
 * Extract YAML front-matter from markdown content
 *
 * @param content - Markdown content
 * @returns Parsed front-matter or null
 */
export function extractFrontMatter(content: string): BMADFrontMatter | null {
  const frontMatterRegex = /^---[^\S\n]*\n([\s\S]*?)\n---[^\S\n]*\n/;
  const match = content.match(frontMatterRegex);

  if (!match) {
    return null;
  }

  const yamlContent = match[1];
  const frontMatter: BMADFrontMatter = {};

  // Simple YAML parser (handles basic key: value pairs)
  const lines = yamlContent.split('\n');

  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith('#')) continue;

    // Handle key: value
    const keyValueMatch = trimmed.match(/^(\w+):[ \t]*(\S.*)?$/);
    if (keyValueMatch) {
      const [, key, rawValue] = keyValueMatch;
      const value = rawValue ?? '';

      // Remove quotes
      let cleanValue = value.replace(/^["']|["']$/g, '').trim();
      // Handle template variables
      cleanValue = cleanValue.replace(/\{\{.*?\}\}/g, '');

      if (key === 'variables') {
        frontMatter[key] = {};
      } else if (key === 'hasSidecar') {
        // Parse boolean value
        const boolVal = parseYamlBoolean(cleanValue);
        if (boolVal !== undefined) {
          frontMatter.hasSidecar = boolVal;
        }
      } else {
        frontMatter[key as keyof BMADFrontMatter] = cleanValue as never;
      }
    }
  }

  return frontMatter;
}

/**
 * Extract requirements from content
 *
 * @param content - Document content
 * @returns Array of requirements
 */
export function extractRequirements(content: string): BMADRequirement[] {
  const requirements: BMADRequirement[] = [];
  const lines = content.split('\n');

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const trimmed = line.trim();

    // Match FR-01, NFR-01, US-01 patterns (optionally prefixed by list markers)
    const reqMatch = trimmed.match(/^(?:[-*+][ \t]+)?(FR|NFR|US)-(\d{2}):[ \t]*(\S.*)$/);
    if (reqMatch) {
      const [, typeStr, numStr, description] = reqMatch;
      requirements.push({
        type: typeStr as RequirementType,
        id: `${typeStr}-${numStr}`,
        number: parseInt(numStr, 10),
        description,
        line: i + 1,
      });
    }
  }

  return requirements;
}

/**
 * Extract user stories from content
 *
 * @param content - Document content
 * @returns Array of user stories
 */
export function extractUserStories(content: string): BMADUserStory[] {
  const stories: BMADUserStory[] = [];
  const lines = content.split('\n');

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];

    // Match US-01: Title format
    const storyMatch = line.trim().match(/^(?:[-*+][ \t]+)?(US-\d{2}):[ \t]*(\S.*)$/);
    if (!storyMatch) continue;

    const [, id, title] = storyMatch;
    const story: BMADUserStory = {
      id,
      title,
      line: i + 1,
    };

    // Look for "As a... I want... so that..." pattern in following lines
    let j = i + 1;
    while (j < lines.length && j < i + 10) {
      const storyLine = lines[j].trim();

      const asPrefix = storyLine.match(/^As an?[ \t]+/i);
      if (asPrefix) {
        story.userType = storyLine.slice(asPrefix[0].length).replace(/,\s*$/, '');
      }

      const wantPrefix = storyLine.match(/^I want[ \t]+/i);
      if (wantPrefix) {
        story.action = storyLine.slice(wantPrefix[0].length).replace(/,\s*$/, '');
      }

      const soThatPrefix = storyLine.match(/^so that[ \t]+/i);
      if (soThatPrefix) {
        story.benefit = storyLine.slice(soThatPrefix[0].length).replace(/\.\s*$/, '');
      }

      // Look for acceptance criteria
      if (storyLine.match(/^acceptance criteria:?$/i)) {
        const criteria: string[] = [];
        let k = j + 1;
        while (k < lines.length && k < j + 20) {
          const criteriaLine = lines[k].trim();
          const criteriaMatch = criteriaLine.match(/^\d+\.[ \t]+(\S.+)$/);
          if (criteriaMatch) {
            criteria.push(criteriaMatch[1]);
          } else if (criteriaLine && !criteriaLine.match(/^\d+\./)) {
            break;
          }
          k++;
        }
        if (criteria.length > 0) {
          story.acceptanceCriteria = criteria;
        }
        break;
      }

      j++;
    }

    stories.push(story);
  }

  return stories;
}

/**
 * Extract change log entries from content
 *
 * @param content - Document content
 * @returns Array of change log entries
 */
export function extractChangeLog(content: string): BMADChangeLogEntry[] {
  const entries: BMADChangeLogEntry[] = [];
  const lines = content.split('\n');

  let inTable = false;
  for (const line of lines) {
    // Detect table header
    if (line.match(/\|\s*Date\s*\|\s*Version\s*\|\s*Description\s*\|\s*Author\s*\|/i)) {
      inTable = true;
      continue;
    }

    // Skip separator row
    if (inTable && line.match(/\|[\s:-]+\|/)) {
      continue;
    }

    // Parse table rows
    if (inTable) {
      const rowMatch = line.match(/\|([^|]+)\|([^|]+)\|([^|]+)\|([^|]+)\|/);
      if (rowMatch) {
        const [, date, version, description, author] = rowMatch;
        entries.push({
          date: date.trim(),
          version: version.trim(),
          description: description.trim(),
          author: author.trim(),
        });
      } else if (line.trim() && !line.startsWith('|')) {
        // End of table
        break;
      }
    }
  }

  return entries;
}

/**
 * Identify document type from content
 *
 * @param content - Document content
 * @param frontMatter - Parsed front-matter
 * @returns Document type
 */
export function identifyDocumentType(
  content: string,
  frontMatter?: BMADFrontMatter | null
): BMADDocumentType {
  // v6: Check for structured YAML document types first
  if (isAgentYamlContent(content)) {
    return BMADDocumentType.AGENT;
  }
  if (isWorkflowYamlContent(content)) {
    return BMADDocumentType.WORKFLOW;
  }
  if (isTeamYamlContent(content)) {
    return BMADDocumentType.TEAM;
  }
  if (isModuleYamlContent(content)) {
    return BMADDocumentType.MODULE;
  }

  // Check front-matter
  if (frontMatter?.name) {
    if (/product requirements/i.test(frontMatter.name)) {
      return BMADDocumentType.PRD;
    }
    if (/architecture/i.test(frontMatter.name)) {
      return BMADDocumentType.ARCHITECTURE;
    }
    if (/\bagent\b/i.test(frontMatter.name)) {
      return BMADDocumentType.AGENT;
    }
  }

  // v6: hasSidecar field is a strong agent indicator
  if (frontMatter?.hasSidecar !== undefined) {
    return BMADDocumentType.AGENT;
  }

  // Check content
  if (/(Product Requirements Document|PRD)/i.test(content)) {
    return BMADDocumentType.PRD;
  }
  if (/Architecture Document/i.test(content)) {
    return BMADDocumentType.ARCHITECTURE;
  }
  if (/Epic:/i.test(content) || /Epic Goal/i.test(content)) {
    return BMADDocumentType.EPIC;
  }
  if (/As an?\s+[^\n]{1,200},\s*\nI want\s+[^\n]{1,200},\s*\nso that/i.test(content)) {
    return BMADDocumentType.STORY;
  }

  return BMADDocumentType.UNKNOWN;
}

/**
 * Analyze content for detection indicators
 *
 * @param content - Document content
 * @param hint - Optional path detection hint
 * @returns Detection indicators
 */
export function analyzeContent(content: string, hint?: PathDetectionHint): DetectionIndicators {
  const frontMatter = extractFrontMatter(content);
  const requirements = extractRequirements(content);

  const pathAnalysis = hint ? analyzePath(hint) : { isBmadFolder: false, isConfigFolder: false };

  return {
    hasYamlFrontMatter: frontMatter !== null && Object.keys(frontMatter).length > 0,
    hasFunctionalRequirements: requirements.some((r) => r.type === RequirementType.FUNCTIONAL),
    hasNonFunctionalRequirements: requirements.some(
      (r) => r.type === RequirementType.NON_FUNCTIONAL
    ),
    hasUserStories: requirements.some((r) => r.type === RequirementType.USER_STORY),
    hasUserStoryFormat:
      /As an?[ \t]+(?:(?![,\s]*I want)[^\n]){1,200}[,\s]+I want[ \t]+(?:(?![,\s]*so that)[^\n]){1,200}[,\s]+so that/i.test(
        content
      ),
    hasChangeLogTable: /\|\s*Date\s*\|\s*Version\s*\|\s*Description\s*\|\s*Author\s*\|/i.test(
      content
    ),
    hasDocumentTitle: /(Product Requirements|Architecture) Document/i.test(content),
    requirementCount: requirements.length,
    hasBmadFolderPath: pathAnalysis.isBmadFolder,
    hasBmadConfigPath: pathAnalysis.isConfigFolder,
    hasHasSidecar: frontMatter?.hasSidecar !== undefined,
    hasHyphenatedVariables: hasHyphenatedVariables(content),
    // v6 structured YAML detection
    isAgentYaml: isAgentYamlContent(content),
    isWorkflowYaml: isWorkflowYamlContent(content),
    isTeamYaml: isTeamYamlContent(content),
    isModuleYaml: isModuleYamlContent(content),
  };
}

/**
 * Calculate detection confidence score
 *
 * @param indicators - Detection indicators
 * @returns Confidence score (0-100)
 */
export function calculateConfidenceScore(indicators: DetectionIndicators): number {
  let score = 0;

  // YAML front-matter (30 points)
  if (indicators.hasYamlFrontMatter) {
    score += 30;
  }

  // Requirement identifiers (25 points)
  if (
    indicators.hasFunctionalRequirements ||
    indicators.hasNonFunctionalRequirements ||
    indicators.hasUserStories
  ) {
    score += 25;
  }

  // User story format (20 points)
  if (indicators.hasUserStoryFormat) {
    score += 20;
  }

  // Change log table (15 points)
  if (indicators.hasChangeLogTable) {
    score += 15;
  }

  // Document title (10 points)
  if (indicators.hasDocumentTitle) {
    score += 10;
  }

  // v6: BMAD folder path (20 points bonus)
  if (indicators.hasBmadFolderPath) {
    score += 20;
  }

  // v6: Config folder path (5 points bonus)
  if (indicators.hasBmadConfigPath) {
    score += 5;
  }

  // v6: hasSidecar field (15 points bonus)
  if (indicators.hasHasSidecar) {
    score += 15;
  }

  // v6: Hyphenated variables like {var-name} (10 points bonus)
  if (indicators.hasHyphenatedVariables) {
    score += 10;
  }

  // v6: Structured YAML document types (high confidence — these are definitive)
  if (indicators.isAgentYaml) {
    score += 80;
  }
  if (indicators.isWorkflowYaml) {
    score += 80;
  }
  if (indicators.isTeamYaml) {
    score += 80;
  }
  if (indicators.isModuleYaml) {
    score += 70;
  }

  return Math.min(100, score);
}

/**
 * Build detection reason message
 *
 * @param indicators - Detection indicators
 * @returns Reason message
 */
export function buildDetectionReason(indicators: DetectionIndicators): string {
  const reasons: string[] = [];

  if (indicators.hasYamlFrontMatter) {
    reasons.push('yaml-frontmatter');
  }
  if (
    indicators.hasFunctionalRequirements ||
    indicators.hasNonFunctionalRequirements ||
    indicators.hasUserStories
  ) {
    reasons.push(`${indicators.requirementCount} requirements`);
  }
  if (indicators.hasUserStoryFormat) {
    reasons.push('user-story-format');
  }
  if (indicators.hasChangeLogTable) {
    reasons.push('change-log-table');
  }
  if (indicators.hasDocumentTitle) {
    reasons.push('document-title');
  }
  if (indicators.hasBmadFolderPath) {
    reasons.push('bmad-folder');
  }
  if (indicators.hasBmadConfigPath) {
    reasons.push('bmad-config');
  }
  if (indicators.hasHasSidecar) {
    reasons.push('has-sidecar');
  }
  if (indicators.hasHyphenatedVariables) {
    reasons.push('hyphenated-variables');
  }
  if (indicators.isAgentYaml) {
    reasons.push('agent-yaml');
  }
  if (indicators.isWorkflowYaml) {
    reasons.push('workflow-yaml');
  }
  if (indicators.isTeamYaml) {
    reasons.push('team-yaml');
  }
  if (indicators.isModuleYaml) {
    reasons.push('module-yaml');
  }

  return reasons.length > 0 ? reasons.join(', ') : 'no strong indicators';
}

/**
 * Extract document title from content
 *
 * @param content - Document content
 * @returns Title or null
 */
export function extractTitle(content: string): string | null {
  const lines = content.split('\n');

  for (const line of lines) {
    const trimmed = line.trim();
    // Match h1 header
    const h1Match = trimmed.match(/^#[ \t]+(\S.+)$/);
    if (h1Match) {
      return h1Match[1];
    }
  }

  return null;
}

/**
 * Extract intent/summary from document
 *
 * @param content - Document content
 * @param docType - Document type
 * @returns Intent description
 */
export function extractIntent(content: string, docType: BMADDocumentType): string {
  const lines = content.split('\n');
  let inTargetSection = false;
  const intentLines: string[] = [];

  // Section headers to look for based on document type
  const targetSections: Record<string, string[]> = {
    [BMADDocumentType.PRD]: ['Executive Summary', 'Product Vision', 'Overview'],
    [BMADDocumentType.ARCHITECTURE]: ['Technical Summary', 'Overview'],
    [BMADDocumentType.EPIC]: ['Epic Goal', 'Goal', 'Overview'],
    [BMADDocumentType.STORY]: ['Description', 'Story'],
    [BMADDocumentType.AGENT]: ['Purpose', 'Role', 'Overview'],
    [BMADDocumentType.UNKNOWN]: ['Overview', 'Summary'],
  };

  const sections = targetSections[docType] ?? targetSections[BMADDocumentType.UNKNOWN];

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const trimmed = line.trim();

    // Check if this is a target section header
    for (const section of sections) {
      if (trimmed.match(new RegExp(`^#{1,3}\\s+${section}\\s*$`, 'i'))) {
        inTargetSection = true;
        continue;
      }
    }

    // If in target section, collect lines until next header
    if (inTargetSection) {
      if (trimmed.match(/^#{1,3}\s+/)) {
        // Hit another header, stop
        break;
      }
      if (trimmed && !trimmed.startsWith('#')) {
        intentLines.push(trimmed);
      }
    }
  }

  // Return first paragraph or first 2 sentences
  const intent = intentLines.join(' ').trim();
  if (intent) {
    const sentences = intent.match(/[^.!?]+[.!?]+/g);
    if (sentences && sentences.length > 0) {
      return sentences.slice(0, 2).join(' ').trim();
    }
    return intent.substring(0, 200).trim() + (intent.length > 200 ? '...' : '');
  }

  // Fallback: use title or first non-empty line
  const title = extractTitle(content);
  return title || 'BMAD document';
}

// ---------------------------------------------------------------------------
// v6 YAML structure detection helpers
// ---------------------------------------------------------------------------

/**
 * Detect whether content is structured agent YAML (agent.metadata + agent.persona)
 */
export function isAgentYamlContent(content: string): boolean {
  return (
    /^agent:\s*$/m.test(content) &&
    /^\s+metadata:\s*$/m.test(content) &&
    /^\s+persona:\s*$/m.test(content)
  );
}

/**
 * Detect whether content is a workflow YAML (has name + description)
 */
export function isWorkflowYamlContent(content: string): boolean {
  return (
    /^name:\s+\S/m.test(content) &&
    /^description:\s*(?:[|>]\s*)?.*/m.test(content) &&
    (/^instructions:\s+/m.test(content) || /^config_source:\s+/m.test(content))
  );
}

/**
 * Detect whether content is a team bundle YAML
 */
export function isTeamYamlContent(content: string): boolean {
  return /^bundle:\s*$/m.test(content) && /^agents:\s*$/m.test(content);
}

/**
 * Detect whether content is a module config YAML.
 * Requires `code:` + `name:` + (`default_selected:` or `directories:`) to
 * avoid false positives on generic config files that also have code/name keys.
 */
export function isModuleYamlContent(content: string): boolean {
  return (
    /^code:\s+\S/m.test(content) &&
    /^name:\s+/m.test(content) &&
    (/^default_selected:\s+/m.test(content) || /^directories:\s*$/m.test(content))
  );
}

// ---------------------------------------------------------------------------
// v6 Agent YAML Parser (lightweight — no js-yaml dependency)
//
// NOTE: The parser assumes BMAD's standard 2-space indentation convention.
// Agent YAML files using 4-space or tab indentation will not parse correctly.
// ---------------------------------------------------------------------------

/**
 * Parse a simple YAML value, handling quotes and multi-line pipe syntax.
 */
function parseSimpleYamlValue(raw: string): string {
  const trimmed = raw.trim();
  // Remove surrounding quotes
  if (
    (trimmed.startsWith('"') && trimmed.endsWith('"')) ||
    (trimmed.startsWith("'") && trimmed.endsWith("'"))
  ) {
    return trimmed.slice(1, -1);
  }
  return trimmed;
}

/**
 * Get indentation level of a line (number of leading spaces)
 */
function indentLevel(line: string): number {
  const match = line.match(/^(\s*)/);
  return match ? match[1].length : 0;
}

/**
 * Collect a plain multi-line scalar (continuation lines indented deeper than the key).
 * Used when a YAML key has no value on the same line and the value is on the next indented lines.
 * Returns the joined text and the index of the last consumed line.
 */
function collectPlainScalar(
  lines: string[],
  startIndex: number,
  keyIndent: number
): { text: string; endIndex: number } {
  const parts: string[] = [];
  let i = startIndex;
  while (i < lines.length) {
    const line = lines[i];
    if (line.trim() === '') break;
    if (indentLevel(line) <= keyIndent) break;
    parts.push(line.trim());
    i++;
  }
  const joined = parts.join(' ');
  return { text: parseSimpleYamlValue(joined), endIndex: i - 1 };
}

/**
 * Collect a multi-line pipe (|) block starting after a `key: |` line.
 * Returns the block text and the index of the last consumed line.
 */
function collectPipeBlock(
  lines: string[],
  startIndex: number,
  baseIndent: number
): { text: string; endIndex: number } {
  const blockLines: string[] = [];
  let i = startIndex;
  while (i < lines.length) {
    const line = lines[i];
    if (line.trim() === '') {
      blockLines.push('');
      i++;
      continue;
    }
    if (indentLevel(line) <= baseIndent) break;
    blockLines.push(line.trimStart());
    i++;
  }
  // Trim trailing empty lines
  while (blockLines.length > 0 && blockLines[blockLines.length - 1] === '') {
    blockLines.pop();
  }
  return { text: blockLines.join('\n'), endIndex: i - 1 };
}

/**
 * Collect a YAML list (lines starting with "- ") at a given indent level.
 * Each item can be a simple string or a multi-line string (quoted).
 */
function collectYamlList(
  lines: string[],
  startIndex: number,
  baseIndent: number
): { items: string[]; endIndex: number } {
  const items: string[] = [];
  let i = startIndex;
  while (i < lines.length) {
    const line = lines[i];
    if (line.trim() === '') {
      i++;
      continue;
    }
    const currentIndent = indentLevel(line);
    if (currentIndent <= baseIndent && line.trim() !== '') break;
    const listMatch = line.match(/^(\s*)- (.*)$/);
    if (listMatch) {
      let value = listMatch[2].trim();
      // Handle multi-line quoted strings (double or single quotes)
      const quoteChar = value.charAt(0);
      if ((quoteChar === '"' || quoteChar === "'") && !value.endsWith(quoteChar)) {
        i++;
        while (i < lines.length) {
          value += ' ' + lines[i].trim();
          if (lines[i].trimEnd().endsWith(quoteChar)) break;
          i++;
        }
      }
      items.push(parseSimpleYamlValue(value));
    }
    i++;
  }
  return { items, endIndex: i - 1 };
}

/**
 * Parse a v6 agent YAML file into a BMADAgentYaml structure.
 *
 * This is a lightweight parser that handles the specific YAML schema
 * used by BMAD agent definitions without requiring a full YAML library.
 */
export function parseAgentYaml(content: string): BMADAgentYaml | null {
  if (!isAgentYamlContent(content)) return null;

  const lines = content.split('\n');
  const metadata: Partial<BMADAgentMetadata> = {};
  const persona: Partial<BMADAgentPersona> = {};
  let critical_actions: string[] | undefined;
  let menu: BMADMenuItem[] | undefined;
  let prompts: BMADAgentPrompt[] | undefined;

  // Typed as string to prevent incorrect CFA narrowing across loop iterations —
  // TypeScript eliminates values assigned before `continue` in earlier iterations.
  let section = 'root' as string;

  // Detect section indent level from first section header (metadata/persona)
  let sectionIndent = 2; // default BMAD convention
  for (const l of lines) {
    const m = l.match(/^(\s+)(metadata|persona):\s*$/);
    if (m) {
      sectionIndent = m[1].length;
      break;
    }
  }

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const trimmed = line.trim();
    if (trimmed === '' || trimmed.startsWith('#')) continue;

    // Top-level section detection (flexible indent)
    const indent = indentLevel(line);
    if (indent === sectionIndent && trimmed.endsWith(':')) {
      const sectionName = trimmed.slice(0, -1);
      if (sectionName === 'metadata') {
        section = 'metadata';
        continue;
      }
      if (sectionName === 'persona') {
        section = 'persona';
        continue;
      }
      if (sectionName === 'critical_actions') {
        section = 'critical_actions';
        const list = collectYamlList(lines, i + 1, sectionIndent);
        critical_actions = list.items;
        i = list.endIndex;
        continue;
      }
      if (sectionName === 'menu') {
        section = 'menu';
        menu = parseMenuItems(lines, i + 1);
        i = skipSection(lines, i + 1, sectionIndent * 2);
        continue;
      }
      if (sectionName === 'prompts') {
        section = 'prompts';
        prompts = parsePromptItems(lines, i + 1);
        i = skipSection(lines, i + 1, sectionIndent * 2);
        continue;
      }
    }

    // Field indent = 2x section indent (e.g. section at 2 → fields at 4)
    const fieldIndent = sectionIndent * 2;

    // Parse metadata fields (dynamic indent based on section indent)
    if (section === 'metadata' && indentLevel(line) >= fieldIndent) {
      const kvMatch = trimmed.match(/^(\w+):\s*(.*)$/);
      if (kvMatch) {
        const [, key, value] = kvMatch;
        if (value.trim() === '') continue;
        const val = parseSimpleYamlValue(value);
        switch (key) {
          case 'id':
            metadata.id = val;
            break;
          case 'name':
            metadata.name = val;
            break;
          case 'title':
            metadata.title = val;
            break;
          case 'icon':
            metadata.icon = val;
            break;
          case 'module':
            metadata.module = val;
            break;
          case 'capabilities':
            metadata.capabilities = val;
            break;
          case 'hasSidecar': {
            const bool = parseYamlBoolean(val);
            metadata.hasSidecar = bool ?? false;
            break;
          }
        }
      }
    }

    // Parse persona fields (dynamic indent based on section indent)
    if (section === 'persona' && indentLevel(line) >= fieldIndent) {
      const kvMatch = trimmed.match(/^(\w[\w_]*):\s*(.*)$/);
      if (kvMatch) {
        const [, key, rawValue] = kvMatch;
        if (rawValue.trim() === '|') {
          // Multi-line pipe block
          const block = collectPipeBlock(lines, i + 1, indentLevel(line));
          (persona as Record<string, string>)[key] = block.text;
          i = block.endIndex;
        } else if (rawValue.trim() === '') {
          // Plain multi-line scalar (value on next indented lines)
          const scalar = collectPlainScalar(lines, i + 1, indentLevel(line));
          (persona as Record<string, string>)[key] = scalar.text;
          i = scalar.endIndex;
        } else {
          (persona as Record<string, string>)[key] = parseSimpleYamlValue(rawValue);
        }
      }
    }
  }

  // Validate required fields
  if (!metadata.id || !metadata.name || !metadata.title) return null;
  if (!persona.role || !persona.identity) return null;

  return {
    metadata: {
      id: metadata.id,
      name: metadata.name,
      title: metadata.title,
      icon: metadata.icon,
      module: metadata.module,
      capabilities: metadata.capabilities,
      hasSidecar: metadata.hasSidecar ?? false,
    },
    persona: {
      role: persona.role,
      identity: persona.identity,
      communication_style: persona.communication_style ?? '',
      principles: persona.principles ?? '',
    },
    critical_actions,
    menu,
    prompts,
  };
}

/**
 * Parse menu items from agent YAML lines.
 */
function parseMenuItems(lines: string[], startIndex: number): BMADMenuItem[] {
  const items: BMADMenuItem[] = [];
  let current: Partial<BMADMenuItem> | null = null;

  for (let i = startIndex; i < lines.length; i++) {
    const line = lines[i];
    if (line.trim() === '') continue;
    const indent = indentLevel(line);
    // Stop when we leave the menu section
    if (indent < 4 && line.trim() !== '') break;

    const trimmed = line.trim();
    // New menu item
    if (trimmed.startsWith('- trigger:')) {
      if (current?.trigger && current.description) {
        items.push(current as BMADMenuItem);
      }
      current = { trigger: parseSimpleYamlValue(trimmed.replace(/^- trigger:\s*/, '')) };
      continue;
    }

    if (current) {
      // Match key: value (value may be empty for multi-line)
      const kvMatch = trimmed.match(/^(\w+):\s*(.*)$/);
      if (kvMatch) {
        const [, key, rawValue] = kvMatch;
        let val: string;
        if (rawValue.trim() === '') {
          // Plain multi-line scalar (value on next indented lines)
          const scalar = collectPlainScalar(lines, i + 1, indent);
          val = scalar.text;
          i = scalar.endIndex;
        } else {
          val = parseSimpleYamlValue(rawValue);
        }
        switch (key) {
          case 'exec':
            current.exec = val;
            break;
          case 'workflow':
            current.workflow = val;
            break;
          case 'action':
            current.action = val;
            break;
          case 'data':
            current.data = val;
            break;
          case 'description':
            current.description = val;
            break;
        }
      }
    }
  }

  // Push last item
  if (current?.trigger && current.description) {
    items.push(current as BMADMenuItem);
  }

  return items;
}

/**
 * Parse prompt items from agent YAML lines.
 */
function parsePromptItems(lines: string[], startIndex: number): BMADAgentPrompt[] {
  const prompts: BMADAgentPrompt[] = [];
  let current: Partial<BMADAgentPrompt> | null = null;
  let collectingContent = false;
  const contentLines: string[] = [];

  for (let i = startIndex; i < lines.length; i++) {
    const line = lines[i];
    const indent = indentLevel(line);
    if (indent < 4 && line.trim() !== '') break;

    const trimmed = line.trim();

    if (trimmed.startsWith('- id:')) {
      // Save previous prompt (block-scalar or inline content)
      if (current?.id) {
        if (collectingContent) {
          current.content = contentLines.join('\n').trim();
        }
        if (current.content) {
          prompts.push(current as BMADAgentPrompt);
        }
      }
      current = { id: parseSimpleYamlValue(trimmed.replace(/^- id:\s*/, '')) };
      collectingContent = false;
      contentLines.length = 0;
      continue;
    }

    if (current && trimmed.startsWith('content:')) {
      const remainder = trimmed.replace(/^content:\s*/, '');
      if (remainder === '|' || remainder === '') {
        collectingContent = true;
      } else {
        current.content = parseSimpleYamlValue(remainder);
      }
      continue;
    }

    if (collectingContent && current) {
      contentLines.push(line.trimStart());
    }
  }

  // Push last prompt
  if (current?.id) {
    if (collectingContent) {
      current.content = contentLines.join('\n').trim();
    }
    if (current.content) {
      prompts.push(current as BMADAgentPrompt);
    }
  }

  return prompts;
}

/**
 * Skip past a YAML section at a given indent level.
 */
function skipSection(lines: string[], startIndex: number, minIndent: number): number {
  let i = startIndex;
  while (i < lines.length) {
    const line = lines[i];
    if (line.trim() === '') {
      i++;
      continue;
    }
    if (indentLevel(line) < minIndent) break;
    i++;
  }
  return i - 1;
}

/**
 * Parse a v6 workflow YAML file.
 */
export function parseWorkflowYaml(content: string): BMADWorkflowYaml | null {
  if (!isWorkflowYamlContent(content)) return null;

  const result: Record<string, string> = {};
  const lines = content.split('\n');

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    if (line.trim() === '' || line.trim().startsWith('#')) continue;
    // Only parse top-level keys (no indent)
    if (indentLevel(line) !== 0) continue;

    const kvMatch = line.match(/^(\w[\w_]*):[ \t]*(\S.*)?$/);
    if (kvMatch) {
      const [, key, rawValue] = kvMatch;
      const trimmedValue = (rawValue ?? '').trim();
      if (trimmedValue === '|') {
        // Literal block scalar — preserve newlines
        const block = collectPipeBlock(lines, i + 1, 0);
        result[key] = block.text;
        i = block.endIndex;
      } else if (trimmedValue === '' || trimmedValue === '>') {
        // Folded or plain multi-line scalar (newlines collapsed to spaces)
        const scalar = collectPlainScalar(lines, i + 1, 0);
        result[key] = scalar.text;
        i = scalar.endIndex;
      } else {
        result[key] = parseSimpleYamlValue(rawValue);
      }
    }
  }

  if (!result['name'] || !result['description']) return null;

  return {
    name: result['name'],
    description: result['description'],
    config_source: result['config_source'],
    installed_path: result['installed_path'],
    instructions: result['instructions'],
    validation: result['validation'],
    ...result,
  } as BMADWorkflowYaml;
}

/**
 * Parse a v6 team bundle YAML file.
 */
export function parseTeamYaml(content: string): BMADTeamYaml | null {
  if (!isTeamYamlContent(content)) return null;

  const lines = content.split('\n');
  const bundle: Record<string, string> = {};
  const agents: string[] = [];
  let party: string | undefined;
  let section: 'root' | 'bundle' | 'agents' = 'root';

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const trimmed = line.trim();
    if (trimmed === '' || trimmed.startsWith('#')) continue;

    if (trimmed === 'bundle:') {
      section = 'bundle';
      continue;
    }
    if (trimmed === 'agents:') {
      section = 'agents';
      continue;
    }
    if (trimmed.startsWith('party:')) {
      party = parseSimpleYamlValue(trimmed.replace(/^party:\s*/, ''));
      section = 'root';
      continue;
    }

    if (section === 'bundle') {
      const kvMatch = trimmed.match(/^(\w+):\s*(.+)$/);
      if (kvMatch) {
        bundle[kvMatch[1]] = parseSimpleYamlValue(kvMatch[2]);
      }
    }

    if (section === 'agents' && trimmed.startsWith('- ')) {
      agents.push(parseSimpleYamlValue(trimmed.replace(/^-\s*/, '').trim()));
    }
  }

  if (!bundle['name'] || agents.length === 0) return null;

  return {
    bundle: { name: bundle['name'], icon: bundle['icon'], description: bundle['description'] },
    agents,
    party,
  };
}

/**
 * Parse a v6 module.yaml configuration file.
 */
export function parseModuleYaml(content: string): BMADModuleYaml | null {
  if (!isModuleYamlContent(content)) return null;

  const lines = content.split('\n');
  const result: Record<string, unknown> = {};
  let directories: string[] | undefined;

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    if (line.trim() === '' || line.trim().startsWith('#')) continue;
    // Only parse top-level keys (no indent)
    if (indentLevel(line) === 0) {
      // Handle directories list
      if (/^directories:\s*$/.test(line)) {
        const list = collectYamlList(lines, i + 1, 0);
        directories = list.items;
        i = list.endIndex;
        continue;
      }
      const kvMatch = line.match(/^(\w[\w_]*):[ \t]*(\S.*)$/);
      if (kvMatch) {
        const [, key, value] = kvMatch;
        const val = parseSimpleYamlValue(value);
        if (val === 'true') result[key] = true;
        else if (val === 'false') result[key] = false;
        else result[key] = val;
      }
    }
  }

  if (!result['code'] || !result['name']) return null;

  return {
    code: result['code'] as string,
    name: result['name'] as string,
    description: result['description'] as string | undefined,
    default_selected: result['default_selected'] as boolean | undefined,
    header: result['header'] as string | undefined,
    subheader: result['subheader'] as string | undefined,
    directories,
  } as BMADModuleYaml;
}
