/**
 * BMAD Format Types
 *
 * Type definitions for BMAD (Breakthrough Method for Agile AI-Driven Development) format.
 */

/**
 * BMAD v6 folder structure constants
 *
 * v6 changed `.bmad` → `_bmad` and `_cfg` → `_config`.
 * We support both legacy and new paths for backward compatibility.
 */
export const BMAD_FOLDERS = {
  /** New v6 project folder */
  PROJECT: '_bmad',
  /** Legacy project folder */
  PROJECT_LEGACY: '.bmad',
  /** New v6 config folder */
  CONFIG: '_config',
  /** Legacy config folder */
  CONFIG_LEGACY: '_cfg',
  /** Agent memory folder (v6) */
  MEMORY: '_memory',
  /** Module config file (v6) */
  MODULE_CONFIG: 'module.yaml',
} as const;

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
  /** v6: Whether the agent document has a sidecar config */
  hasSidecar?: boolean;
}

/**
 * BMAD document types
 */
export enum BMADDocumentType {
  PRD = 'prd',
  ARCHITECTURE = 'architecture',
  EPIC = 'epic',
  STORY = 'story',
  /** v6: Agent persona/configuration document */
  AGENT = 'agent',
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
  /** v6: Path is inside a BMAD project folder */
  hasBmadFolderPath: boolean;
  /** v6: Path is inside a BMAD config folder */
  hasBmadConfigPath: boolean;
  /** v6: Document has hasSidecar field */
  hasHasSidecar: boolean;
  /** v6: Document uses hyphenated variable syntax */
  hasHyphenatedVariables: boolean;
}
