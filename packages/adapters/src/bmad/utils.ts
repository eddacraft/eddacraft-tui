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
} from './types.js';

/**
 * Extract YAML front-matter from markdown content
 *
 * @param content - Markdown content
 * @returns Parsed front-matter or null
 */
export function extractFrontMatter(content: string): BMADFrontMatter | null {
  const frontMatterRegex = /^---\s*\n([\s\S]*?)\n---\s*\n/;
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
    const keyValueMatch = trimmed.match(/^(\w+):\s*(.*)$/);
    if (keyValueMatch) {
      const [, key, value] = keyValueMatch;

      // Remove quotes
      let cleanValue = value.replace(/^["']|["']$/g, '').trim();
      // Handle template variables
      cleanValue = cleanValue.replace(/\{\{.*?\}\}/g, '');

      if (key === 'variables') {
        frontMatter[key] = {};
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

    // Match FR-01, NFR-01, US-01 patterns
    const reqMatch = trimmed.match(/^(FR|NFR|US)-(\d{2}):\s*(.+)$/);
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
    const storyMatch = line.match(/^(US-\d{2}):\s*(.+)$/);
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

      const asMatch = storyLine.match(/^As an?\s+(.+?),?\s*$/i);
      if (asMatch) {
        story.userType = asMatch[1];
      }

      const wantMatch = storyLine.match(/^I want\s+(.+?),?\s*$/i);
      if (wantMatch) {
        story.action = wantMatch[1];
      }

      const soThatMatch = storyLine.match(/^so that\s+(.+?)\.?\s*$/i);
      if (soThatMatch) {
        story.benefit = soThatMatch[1];
      }

      // Look for acceptance criteria
      if (storyLine.match(/^acceptance criteria:?$/i)) {
        const criteria: string[] = [];
        let k = j + 1;
        while (k < lines.length && k < j + 20) {
          const criteriaLine = lines[k].trim();
          const criteriaMatch = criteriaLine.match(/^\d+\.\s+(.+)$/);
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
      const rowMatch = line.match(/\|\s*([^|]+)\s*\|\s*([^|]+)\s*\|\s*([^|]+)\s*\|\s*([^|]+)\s*\|/);
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
  // Check front-matter
  if (frontMatter?.name) {
    if (/product requirements/i.test(frontMatter.name)) {
      return BMADDocumentType.PRD;
    }
    if (/architecture/i.test(frontMatter.name)) {
      return BMADDocumentType.ARCHITECTURE;
    }
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
  if (/As an?\s+.+,\s*\nI want\s+.+,\s*\nso that/i.test(content)) {
    return BMADDocumentType.STORY;
  }

  return BMADDocumentType.UNKNOWN;
}

/**
 * Analyze content for detection indicators
 *
 * @param content - Document content
 * @returns Detection indicators
 */
export function analyzeContent(content: string): DetectionIndicators {
  const frontMatter = extractFrontMatter(content);
  const requirements = extractRequirements(content);

  return {
    hasYamlFrontMatter: frontMatter !== null && Object.keys(frontMatter).length > 0,
    hasFunctionalRequirements: requirements.some((r) => r.type === RequirementType.FUNCTIONAL),
    hasNonFunctionalRequirements: requirements.some(
      (r) => r.type === RequirementType.NON_FUNCTIONAL
    ),
    hasUserStories: requirements.some((r) => r.type === RequirementType.USER_STORY),
    hasUserStoryFormat: /As an?\s+.+[,\s]+I want\s+.+[,\s]+so that/i.test(content),
    hasChangeLogTable: /\|\s*Date\s*\|\s*Version\s*\|\s*Description\s*\|\s*Author\s*\|/i.test(
      content
    ),
    hasDocumentTitle: /(Product Requirements|Architecture) Document/i.test(content),
    requirementCount: requirements.length,
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
    const h1Match = trimmed.match(/^#\s+(.+)$/);
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
  const targetSections = {
    [BMADDocumentType.PRD]: ['Executive Summary', 'Product Vision', 'Overview'],
    [BMADDocumentType.ARCHITECTURE]: ['Technical Summary', 'Overview'],
    [BMADDocumentType.EPIC]: ['Epic Goal', 'Goal', 'Overview'],
    [BMADDocumentType.STORY]: ['Description', 'Story'],
    [BMADDocumentType.UNKNOWN]: ['Overview', 'Summary'],
  };

  const sections = targetSections[docType];

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
