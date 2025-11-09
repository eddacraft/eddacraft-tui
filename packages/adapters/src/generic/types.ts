/**
 * Generic Adapter Types
 *
 * Types for the generic markdown adapter that handles common planning document formats.
 */

/**
 * Parsed generic document structure
 */
export interface GenericDocument {
  /** Document title (from first # heading) */
  title?: string;
  /** Intent/purpose extracted from intro or objective sections */
  intent?: string;
  /** Overview/description */
  overview?: string;
  /** Goals or objectives */
  goals?: string[];
  /** Requirements extracted from various sections */
  requirements?: string[];
  /** Tasks or action items */
  tasks?: string[];
  /** Features or capabilities */
  features?: string[];
  /** Metadata from document */
  metadata?: Record<string, unknown>;
  /** Raw content */
  raw: string;
}

/**
 * Detection indicators for generic markdown
 */
export interface GenericIndicators {
  /** Has markdown headings */
  hasHeadings: boolean;
  /** Has lists (bullet or numbered) */
  hasLists: boolean;
  /** Has requirements-like content */
  hasRequirementsSection: boolean;
  /** Has goals/objectives section */
  hasGoalsSection: boolean;
  /** Has tasks/action items */
  hasTasksSection: boolean;
  /** Has features section */
  hasFeaturesSection: boolean;
  /** Has overview/description */
  hasOverviewSection: boolean;
  /** Document length */
  wordCount: number;
  /** Number of list items */
  listItemCount: number;
}
