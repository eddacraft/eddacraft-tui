/**
 * BMAD Format Types
 *
 * Type definitions for BMAD (Breakthrough Method for Agile AI-Driven Development) format.
 */

/**
 * YAML front-matter metadata
 */
export interface BMADFrontMatter {
  name?: string;
  version?: string;
  description?: string;
  output_file?: string;
  variables?: Record<string, string>;
  template?: string;
  date?: string;
  author?: string;
}

/**
 * BMAD document types
 */
export enum BMADDocumentType {
  PRD = 'prd',
  ARCHITECTURE = 'architecture',
  EPIC = 'epic',
  STORY = 'story',
  UNKNOWN = 'unknown',
}

/**
 * Requirement types
 */
export enum RequirementType {
  FUNCTIONAL = 'FR',
  NON_FUNCTIONAL = 'NFR',
  USER_STORY = 'US',
}

/**
 * Parsed requirement
 */
export interface BMADRequirement {
  type: RequirementType;
  id: string;
  number: number;
  description: string;
  line?: number;
}

/**
 * User story structure
 */
export interface BMADUserStory {
  id: string;
  title: string;
  userType?: string;
  action?: string;
  benefit?: string;
  acceptanceCriteria?: string[];
  line?: number;
}

/**
 * Change log entry
 */
export interface BMADChangeLogEntry {
  date: string;
  version: string;
  description: string;
  author: string;
}

/**
 * Parsed BMAD document
 */
export interface BMADDocument {
  type: BMADDocumentType;
  frontMatter?: BMADFrontMatter;
  title?: string;
  intent?: string;
  requirements: BMADRequirement[];
  userStories: BMADUserStory[];
  changeLog: BMADChangeLogEntry[];
  sections: Map<string, string>;
  raw: string;
}

/**
 * Detection indicators for confidence scoring
 */
export interface DetectionIndicators {
  hasYamlFrontMatter: boolean;
  hasFunctionalRequirements: boolean;
  hasNonFunctionalRequirements: boolean;
  hasUserStories: boolean;
  hasUserStoryFormat: boolean;
  hasChangeLogTable: boolean;
  hasDocumentTitle: boolean;
  requirementCount: number;
}
