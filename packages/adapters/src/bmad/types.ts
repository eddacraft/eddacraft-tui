/**
 * BMAD Format Types
 *
 * Type definitions for BMAD (Breakthrough Method for Agile AI-Driven Development) format.
 * Supports BMAD v6.0.3 (latest stable) and legacy formats.
 */

/** Latest supported upstream BMAD version */
export const BMAD_UPSTREAM_VERSION = '6.0.3';

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
  /** Output folder default (v6) */
  OUTPUT: '_bmad-output',
} as const;

/**
 * BMAD v6 module codes
 */
/** Known BMAD module codes. Accept any string for custom modules. */
export type BMADModuleCode = 'bmm' | 'core' | 'cis' | (string & {});

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
  /** v6: Agent persona/configuration document (.agent.yaml or .md) */
  AGENT = 'agent',
  /** v6: Workflow configuration (.yaml or .md) */
  WORKFLOW = 'workflow',
  /** v6: Team bundle configuration (.yaml) */
  TEAM = 'team',
  /** v6: Module configuration (module.yaml) */
  MODULE = 'module',
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

// ---------------------------------------------------------------------------
// v6 Agent YAML Schema
// ---------------------------------------------------------------------------

/**
 * v6 agent metadata block (agent.metadata.*)
 */
export interface BMADAgentMetadata {
  /** Path-based ID, e.g. "_bmad/bmm/agents/pm.md" */
  id: string;
  /** Display name, e.g. "John" */
  name: string;
  /** Role title, e.g. "Product Manager" */
  title: string;
  /** Emoji icon */
  icon?: string;
  /** Module code (bmm, core, cis) */
  module?: BMADModuleCode;
  /** Comma-separated capabilities */
  capabilities?: string;
  /** Whether the agent has a sidecar config */
  hasSidecar: boolean;
}

/**
 * v6 agent persona block (agent.persona.*)
 */
export interface BMADAgentPersona {
  role: string;
  identity: string;
  communication_style: string;
  /** Multi-line pipe-delimited principles */
  principles: string;
}

/**
 * v6 agent menu item (agent.menu[])
 *
 * Menu items reference workflows via one of three execution mechanisms:
 * - exec: path to a markdown workflow file
 * - workflow: path to a YAML workflow file
 * - action: inline action description (e.g. "list all tasks from ...")
 */
export interface BMADMenuItem {
  /** Trigger pattern, e.g. "CP or fuzzy match on create-prd" */
  trigger: string;
  /** Markdown workflow path (mutually exclusive with workflow/action) */
  exec?: string;
  /** YAML workflow path (mutually exclusive with exec/action) */
  workflow?: string;
  /** Inline action (mutually exclusive with exec/workflow) */
  action?: string;
  /** Optional data file reference */
  data?: string;
  /** User-facing description with trigger code prefix */
  description: string;
}

/**
 * v6 agent prompt definition (agent.prompts[])
 */
export interface BMADAgentPrompt {
  id: string;
  content: string;
}

/**
 * Complete v6 agent YAML document (parsed from .agent.yaml files)
 */
export interface BMADAgentYaml {
  metadata: BMADAgentMetadata;
  persona: BMADAgentPersona;
  /** Critical action directives the agent must follow */
  critical_actions?: string[];
  /** Menu items / available commands */
  menu?: BMADMenuItem[];
  /** Canned prompt definitions */
  prompts?: BMADAgentPrompt[];
}

// ---------------------------------------------------------------------------
// v6 Workflow YAML Schema
// ---------------------------------------------------------------------------

/**
 * v6 workflow YAML configuration
 */
export interface BMADWorkflowYaml {
  name: string;
  description: string;
  /** Config source reference, e.g. "{project-root}/_bmad/bmm/config.yaml" */
  config_source?: string;
  /** Path to installed workflow directory */
  installed_path?: string;
  /** Path to instruction file (.xml or .md) */
  instructions?: string;
  /** Path to validation checklist */
  validation?: string;
  /** All other key-value config entries */
  [key: string]: string | undefined;
}

// ---------------------------------------------------------------------------
// v6 Team Configuration
// ---------------------------------------------------------------------------

/**
 * v6 team bundle configuration
 */
export interface BMADTeamBundle {
  name: string;
  icon?: string;
  description?: string;
}

export interface BMADTeamYaml {
  bundle: BMADTeamBundle;
  /** Agent role names referenced in the team */
  agents: string[];
  /** Path to party CSV, e.g. "./default-party.csv" */
  party?: string;
}

// ---------------------------------------------------------------------------
// v6 Module Configuration
// ---------------------------------------------------------------------------

export interface BMADModuleYaml {
  code: BMADModuleCode;
  name: string;
  description?: string;
  default_selected?: boolean;
  header?: string;
  subheader?: string;
  /** Directory patterns to create */
  directories?: string[];
}

// ---------------------------------------------------------------------------
// Parsed BMAD Document (union)
// ---------------------------------------------------------------------------

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
  /** v6: Parsed agent YAML (when type is AGENT and source is .agent.yaml) */
  agentYaml?: BMADAgentYaml;
  /** v6: Parsed workflow YAML (when type is WORKFLOW) */
  workflowYaml?: BMADWorkflowYaml;
  /** v6: Parsed team YAML (when type is TEAM) */
  teamYaml?: BMADTeamYaml;
  /** v6: Parsed module YAML (when type is MODULE) */
  moduleYaml?: BMADModuleYaml;
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
  /** v6: Content is a structured agent YAML (agent.metadata + agent.persona) */
  isAgentYaml: boolean;
  /** v6: Content is a workflow YAML (has name + description + instructions/config_source) */
  isWorkflowYaml: boolean;
  /** v6: Content is a team bundle YAML (has bundle + agents) */
  isTeamYaml: boolean;
  /** v6: Content is a module config YAML (has code + name) */
  isModuleYaml: boolean;
}
