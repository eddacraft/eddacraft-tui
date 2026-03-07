/**
 * Generic Adapter Utilities
 *
 * Helper functions for parsing generic markdown documents.
 */

import type { GenericDocument, GenericIndicators } from './types.js';

/**
 * Extract document title from first heading
 */
export function extractTitle(content: string): string | undefined {
  const match = content.match(/^#\s+(\S[^\n]*)$/m);
  return match ? match[1].trim() : undefined;
}

/**
 * Extract intent from common sections
 */
export function extractIntent(content: string): string | undefined {
  // Look for sections like: Purpose, Intent, Objective, Goal, Summary
  const patterns = [
    /##[ \t]+(?:Purpose|Intent|Objective|Goal)[ \t]*\n+([^\n#]+)/i,
    /##[ \t]+(?:Executive[ \t]+)?Summary[ \t]*\n+([^\n#]+)/i,
    /^([^#\n]+?)(?=\n##|\n#|$)/m, // First paragraph as fallback
  ];

  for (const pattern of patterns) {
    const match = content.match(pattern);
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
  const patterns = [
    /##[ \t]+(?:Overview|Description|Background|Context)[ \t]*\n+([\s\S]+?)(?=\n##|$)/i,
  ];

  for (const pattern of patterns) {
    const match = content.match(pattern);
    if (match && match[1]) {
      return match[1].trim();
    }
  }

  return undefined;
}

/**
 * Extract goals from lists under Goals/Objectives sections
 */
export function extractGoals(content: string): string[] {
  const goals: string[] = [];

  // Match Goals/Objectives section
  const sectionMatch = content.match(/##[ \t]+(?:Goals?|Objectives?)\s*\n+([\s\S]+?)(?=\n##|$)/i);

  if (sectionMatch) {
    const section = sectionMatch[1];
    // Extract list items
    const listItems = section.match(/^[-*][ \t]+(\S.*)$/gm);
    if (listItems) {
      goals.push(...listItems.map((item) => item.replace(/^[-*]\s+/, '').trim()));
    }
  }

  return goals;
}

/**
 * Extract requirements from various sections
 */
export function extractRequirements(content: string): string[] {
  const requirements: string[] = [];

  // Match Requirements/Needs/Must Have sections
  const patterns = [
    /##[ \t]+(?:Requirements?|Needs?|Must[ \t]+Have)[ \t]*\n+([\s\S]+?)(?=\n##|$)/i,
    /##[ \t]+(?:Functional[ \t]+)?Requirements?[ \t]*\n+([\s\S]+?)(?=\n##|$)/i,
  ];

  for (const pattern of patterns) {
    const match = content.match(pattern);
    if (match) {
      const section = match[1];
      // Extract list items
      const listItems = section.match(/^[-*][ \t]+(\S.*)$/gm);
      if (listItems) {
        requirements.push(...listItems.map((item) => item.replace(/^[-*]\s+/, '').trim()));
      }
      // Extract numbered items
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
  const tasks: string[] = [];

  const patterns = [
    /##[ \t]+(?:Tasks?|Action[ \t]+Items?|To[ \t]+Do|TODO)[ \t]*\n+([\s\S]+?)(?=\n##|$)/i,
  ];

  for (const pattern of patterns) {
    const match = content.match(pattern);
    if (match) {
      const section = match[1];
      // Extract list items (including checkboxes)
      const listItems = section.match(/^[-*][ \t]+(?:\[[ x]\][ \t]+)?(\S.*)$/gm);
      if (listItems) {
        tasks.push(...listItems.map((item) => item.replace(/^[-*]\s+(?:\[[ x]\]\s+)?/, '').trim()));
      }
    }
  }

  return tasks;
}

/**
 * Extract features from Features/Capabilities sections
 */
export function extractFeatures(content: string): string[] {
  const features: string[] = [];

  const patterns = [
    /##[ \t]+(?:Features?|Capabilities?|Functionality)[ \t]*\n+([\s\S]+?)(?=\n##|$)/i,
  ];

  for (const pattern of patterns) {
    const match = content.match(pattern);
    if (match) {
      const section = match[1];
      const listItems = section.match(/^[-*][ \t]+(\S.*)$/gm);
      if (listItems) {
        features.push(...listItems.map((item) => item.replace(/^[-*]\s+/, '').trim()));
      }
    }
  }

  return features;
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
