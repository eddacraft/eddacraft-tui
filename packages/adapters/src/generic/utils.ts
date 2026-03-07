/**
 * Generic Adapter Utilities
 *
 * Helper functions for parsing generic markdown documents.
 */

import type { GenericDocument, GenericIndicators } from './types.js';

function normaliseLineEndings(content: string): string {
  return content.replace(/\r\n/g, '\n');
}

function extractSectionBody(content: string, headerPattern: RegExp): string | undefined {
  const normalised = normaliseLineEndings(content);
  const match = normalised.match(headerPattern);
  if (!match) return undefined;
  const start = match.index! + match[0].length;
  const nextSection = normalised.indexOf('\n##', start);
  const body = nextSection === -1 ? normalised.slice(start) : normalised.slice(start, nextSection);
  return body.trim() || undefined;
}

/**
 * Extract document title from first heading
 */
export function extractTitle(content: string): string | undefined {
  const match = normaliseLineEndings(content).match(/^#\s+(\S[^\n]*)$/m);
  return match ? match[1].trim() : undefined;
}

/**
 * Extract intent from common sections
 */
export function extractIntent(content: string): string | undefined {
  const normalised = normaliseLineEndings(content);
  // Look for sections like: Purpose, Intent, Objective, Goal, Summary
  const patterns = [
    /##[ \t]+(?:Purpose|Intent|Objective|Goal)[ \t]*\n+([^\n#]+)/i,
    /##[ \t]+(?:Executive[ \t]+)?Summary[ \t]*\n+([^\n#]+)/i,
    /^([^#\n]+?)(?=\n##|\n#|$)/m, // First paragraph as fallback
  ];

  for (const pattern of patterns) {
    const match = normalised.match(pattern);
    if (match && match[1]) {
      return match[1].trim().substring(0, 500); // Limit length
    }
  }

  return undefined;
}

/**
 * Extract overview/description
 */
export function extractOverview(content: string): string | undefined {
  return extractSectionBody(
    content,
    /##[ \t]+(?:Overview|Description|Background|Context)[ \t]*\n/i
  );
}

/**
 * Extract goals from lists under Goals/Objectives sections
 */
export function extractGoals(content: string): string[] {
  const section = extractSectionBody(content, /##[ \t]+(?:Goals?|Objectives?)[ \t]*\n/i);
  if (!section) return [];

  const listItems = section.match(/^[-*][ \t]+(\S.*)$/gm);
  return listItems ? listItems.map((item) => item.replace(/^[-*]\s+/, '').trim()) : [];
}

/**
 * Extract requirements from various sections
 */
export function extractRequirements(content: string): string[] {
  const requirements: string[] = [];

  const headerPatterns = [
    /##[ \t]+(?:Requirements?|Needs?|Must[ \t]+Have)[ \t]*\n/i,
    /##[ \t]+(?:Functional[ \t]+)?Requirements?[ \t]*\n/i,
  ];

  for (const headerPattern of headerPatterns) {
    const section = extractSectionBody(content, headerPattern);
    if (section) {
      const listItems = section.match(/^[-*][ \t]+(\S.*)$/gm);
      if (listItems) {
        requirements.push(...listItems.map((item) => item.replace(/^[-*]\s+/, '').trim()));
      }
      const numberedItems = section.match(/^\d+\.[ \t]+(\S.*)$/gm);
      if (numberedItems) {
        requirements.push(...numberedItems.map((item) => item.replace(/^\d+\.\s+/, '').trim()));
      }
    }
  }

  return requirements;
}

/**
 * Extract tasks from Tasks/Action Items sections
 */
export function extractTasks(content: string): string[] {
  const section = extractSectionBody(
    content,
    /##[ \t]+(?:Tasks?|Action[ \t]+Items?|To[ \t]+Do|TODO)[ \t]*\n/i
  );
  if (!section) return [];

  const listItems = section.match(/^[-*][ \t]+(?:\[[ x]\][ \t]+)?(\S.*)$/gm);
  return listItems
    ? listItems.map((item) => item.replace(/^[-*]\s+(?:\[[ x]\]\s+)?/, '').trim())
    : [];
}

/**
 * Extract features from Features/Capabilities sections
 */
export function extractFeatures(content: string): string[] {
  const section = extractSectionBody(
    content,
    /##[ \t]+(?:Features?|Capabilities?|Functionality)[ \t]*\n/i
  );
  if (!section) return [];

  const listItems = section.match(/^[-*][ \t]+(\S.*)$/gm);
  return listItems ? listItems.map((item) => item.replace(/^[-*]\s+/, '').trim()) : [];
}

/**
 * Analyze content for generic markdown indicators
 */
export function analyzeContent(content: string): GenericIndicators {
  const headings = content.match(/^#{1,6}[ \t]+\S.*$/gm) || [];
  const listItems = content.match(/^[-*][ \t]+\S.*$/gm) || [];
  const words = content.split(/\s+/).filter((w) => w.length > 0);

  return {
    hasHeadings: headings.length > 0,
    hasLists: listItems.length > 0,
    hasRequirementsSection: /##\s+(?:Requirements?|Needs?)/i.test(content),
    hasGoalsSection: /##\s+(?:Goals?|Objectives?)/i.test(content),
    hasTasksSection: /##\s+(?:Tasks?|Action\s+Items?|To\s+Do)/i.test(content),
    hasFeaturesSection: /##\s+(?:Features?|Capabilities?)/i.test(content),
    hasOverviewSection: /##\s+(?:Overview|Description|Background)/i.test(content),
    wordCount: words.length,
    listItemCount: listItems.length,
  };
}

/**
 * Calculate confidence score for generic markdown detection
 */
export function calculateConfidenceScore(indicators: GenericIndicators): number {
  let score = 0;

  // Basic markdown structure (10 points)
  if (indicators.hasHeadings) {
    score += 10;
  }

  // Has lists (10 points)
  if (indicators.hasLists) {
    score += 10;
  }

  // Planning-related sections (5 points each)
  if (indicators.hasRequirementsSection) {
    score += 5;
  }
  if (indicators.hasGoalsSection) {
    score += 5;
  }
  if (indicators.hasTasksSection) {
    score += 5;
  }
  if (indicators.hasFeaturesSection) {
    score += 5;
  }
  if (indicators.hasOverviewSection) {
    score += 5;
  }

  // Content density (up to 10 points)
  if (indicators.wordCount >= 100) {
    score += 5;
  }
  if (indicators.wordCount >= 500) {
    score += 5;
  }

  // List items (up to 5 points)
  if (indicators.listItemCount >= 3) {
    score += 3;
  }
  if (indicators.listItemCount >= 10) {
    score += 2;
  }

  return Math.min(100, score);
}

/**
 * Build detection reason string
 */
export function buildDetectionReason(indicators: GenericIndicators): string {
  const reasons: string[] = [];

  if (indicators.hasHeadings) {
    reasons.push('markdown-headings');
  }
  if (indicators.hasLists) {
    reasons.push(`${indicators.listItemCount} list-items`);
  }
  if (indicators.hasRequirementsSection) {
    reasons.push('requirements-section');
  }
  if (indicators.hasGoalsSection) {
    reasons.push('goals-section');
  }
  if (indicators.hasTasksSection) {
    reasons.push('tasks-section');
  }
  if (indicators.hasFeaturesSection) {
    reasons.push('features-section');
  }
  if (indicators.hasOverviewSection) {
    reasons.push('overview-section');
  }

  return reasons.length > 0 ? reasons.join(', ') : 'generic-markdown';
}

/**
 * Parse generic markdown document
 */
export function parseGenericDocument(content: string): GenericDocument {
  return {
    title: extractTitle(content),
    intent: extractIntent(content),
    overview: extractOverview(content),
    goals: extractGoals(content),
    requirements: extractRequirements(content),
    tasks: extractTasks(content),
    features: extractFeatures(content),
    raw: content,
  };
}
