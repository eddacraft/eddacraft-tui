/**
 * BMAD Parser
 *
 * Parses BMAD format documents (PRD, Architecture, Epics, Stories) into APS plans.
 */

import { type APSPlan, type Change, validateRelativePath } from '@eddacraft/anvil-core';
import type { ParseContext, AdapterError, AdapterWarning } from '../base/types.js';
import {
  BMADDocument,
  BMADDocumentType,
  BMADRequirement,
  BMADUserStory,
  RequirementType,
} from './types.js';
import {
  extractFrontMatter,
  extractRequirements,
  extractUserStories,
  extractChangeLog,
  identifyDocumentType,
  extractTitle,
  extractIntent,
  parseAgentYaml,
  parseWorkflowYaml,
  parseTeamYaml,
  parseModuleYaml,
} from './utils.js';
import { createError, createWarning, generateDeterministicPlanId } from '../base/utils.js';

/**
 * Parse BMAD document into internal structure
 *
 * @param content - BMAD markdown content
 * @returns Parsed document
 */
/**
 * Validate and sanitize a file path to prevent path traversal attacks.
 * Falls back to stripping special characters if validation fails.
 */
function safePath(raw: string): string {
  const cleaned = raw.replace(/\{project-root\}\/?/g, '');
  try {
    return validateRelativePath(cleaned);
  } catch {
    throw new Error(
      `Invalid path in BMAD document: "${raw}" — must be a relative path within the project`
    );
  }
}

/** Maximum input size for BMAD parsing (2MB) */
const MAX_INPUT_SIZE = 2 * 1024 * 1024;

export function parseBMADDocument(content: string): BMADDocument {
  if (content.length > MAX_INPUT_SIZE) {
    throw new Error(`Input exceeds maximum size of ${MAX_INPUT_SIZE} bytes`);
  }
  const frontMatter = extractFrontMatter(content);
  const docType = identifyDocumentType(content, frontMatter);

  // For v6 YAML document types, parse the structured content
  const agentYaml = docType === BMADDocumentType.AGENT ? parseAgentYaml(content) : null;
  const workflowYaml = docType === BMADDocumentType.WORKFLOW ? parseWorkflowYaml(content) : null;
  const teamYaml = docType === BMADDocumentType.TEAM ? parseTeamYaml(content) : null;
  const moduleYaml = docType === BMADDocumentType.MODULE ? parseModuleYaml(content) : null;

  // For YAML-based types, skip markdown extraction
  const isYamlType = agentYaml || workflowYaml || teamYaml || moduleYaml;

  const requirements = isYamlType ? [] : extractRequirements(content);
  const userStories = isYamlType ? [] : extractUserStories(content);
  const changeLog = isYamlType ? [] : extractChangeLog(content);

  // Derive title and intent from YAML or markdown
  let title: string | null = null;
  let intent: string;

  if (agentYaml) {
    title = `${agentYaml.metadata.name} - ${agentYaml.metadata.title}`;
    intent = agentYaml.persona.role;
  } else if (workflowYaml) {
    title = workflowYaml.name;
    intent = workflowYaml.description;
  } else if (teamYaml) {
    title = teamYaml.bundle.name;
    intent = teamYaml.bundle.description || `Team with ${teamYaml.agents.length} agents`;
  } else if (moduleYaml) {
    title = moduleYaml.name;
    intent = moduleYaml.description ?? `Module: ${moduleYaml.code}`;
  } else {
    title = extractTitle(content);
    intent = extractIntent(content, docType);
  }

  return {
    type: docType,
    frontMatter: frontMatter || undefined,
    title: title || undefined,
    intent,
    requirements,
    userStories,
    changeLog,
    sections: new Map(),
    raw: content,
    agentYaml: agentYaml || undefined,
    workflowYaml: workflowYaml || undefined,
    teamYaml: teamYaml || undefined,
    moduleYaml: moduleYaml || undefined,
  };
}

/**
 * Convert BMAD requirement to APS change
 *
 * @param requirement - BMAD requirement
 * @returns APS change
 */
function requirementToChange(requirement: BMADRequirement): Change {
  // Map requirement type to change type
  switch (requirement.type) {
    case RequirementType.FUNCTIONAL:
      // FR typically means creating or updating files
      return {
        type: 'file_create',
        path: safePath(`features/${requirement.id.toLowerCase()}.ts`),
        description: `${requirement.id}: ${requirement.description}`,
      };

    case RequirementType.NON_FUNCTIONAL:
      // NFR typically means configuration or validation
      return {
        type: 'config_update',
        path: 'config/requirements.json',
        description: `${requirement.id}: ${requirement.description}`,
      };

    case RequirementType.USER_STORY:
      // US typically means creating feature files
      return {
        type: 'file_create',
        path: safePath(`features/stories/${requirement.id.toLowerCase()}.ts`),
        description: `${requirement.id}: ${requirement.description}`,
      };

    default:
      return {
        type: 'file_create',
        path: safePath(`requirements/${requirement.id.toLowerCase()}.md`),
        description: requirement.description,
      };
  }
}

/**
 * Convert BMAD user story to APS change
 *
 * @param story - BMAD user story
 * @returns APS change
 */
function userStoryToChange(story: BMADUserStory): Change {
  let description = `${story.id}: ${story.title}`;

  if (story.userType && story.action && story.benefit) {
    description += ` (As a ${story.userType}, I want ${story.action}, so that ${story.benefit})`;
  }

  if (story.acceptanceCriteria && story.acceptanceCriteria.length > 0) {
    description += ` - Acceptance criteria: ${story.acceptanceCriteria.join('; ')}`;
  }

  return {
    type: 'file_create',
    path: safePath(`features/stories/${story.id.toLowerCase()}.ts`),
    description,
  };
}

export function bmadToAPS(
  document: BMADDocument,
  context?: ParseContext,
  originalContent?: string
): APSPlan {
  const planId =
    context?.planId ??
    (originalContent
      ? generateDeterministicPlanId(originalContent)
      : `aps-${Date.now().toString(16).substring(0, 8)}`);

  // Convert requirements and stories to changes
  const changes: Change[] = [];
  const errors: AdapterError[] = [];
  const warnings: AdapterWarning[] = [];

  // v6: Handle agent YAML documents
  if (document.agentYaml) {
    const agent = document.agentYaml;
    changes.push({
      type: 'file_create',
      path: safePath(agent.metadata.id || `agents/${agent.metadata.name.toLowerCase()}.md`),
      description: `Agent: ${agent.metadata.name} (${agent.metadata.title}) - ${agent.persona.role}`,
    });

    // Add menu items as references
    if (agent.menu) {
      for (const item of agent.menu) {
        const workflowPath = item.exec || item.workflow;
        if (workflowPath) {
          changes.push({
            type: 'file_create',
            path: safePath(workflowPath),
            description: item.description,
          });
        }
      }
    }
  }

  // v6: Handle workflow YAML documents
  // Resolve {installed_path} to conventional BMAD location; strip remaining placeholders
  if (document.workflowYaml) {
    const wf = document.workflowYaml;
    const installedPath = wf.installed_path ?? '_bmad';
    const resolvePath = (p: string) =>
      safePath(p.replace(/\{installed_path\}/g, installedPath).replace(/\{[^}]+\}\/?/g, ''));
    if (wf.instructions) {
      changes.push({
        type: 'file_create',
        path: resolvePath(wf.instructions),
        description: `Workflow instructions: ${wf.name}`,
      });
    }
    if (wf.validation) {
      changes.push({
        type: 'file_create',
        path: resolvePath(wf.validation),
        description: `Workflow validation: ${wf.name}`,
      });
    }
  }

  // v6: Handle team YAML documents
  if (document.teamYaml) {
    for (const agentName of document.teamYaml.agents) {
      changes.push({
        type: 'file_create',
        path: safePath(`agents/${agentName}.md`),
        description: `Team agent: ${agentName}`,
      });
    }
  }

  // v6: Handle module YAML documents
  if (document.moduleYaml) {
    const mod = document.moduleYaml;
    changes.push({
      type: 'config_update',
      path: safePath(`_bmad/${mod.code}/module.yaml`),
      description: `Module config: ${mod.name}`,
    });
    if (mod.directories) {
      for (const dir of mod.directories) {
        changes.push({
          type: 'file_create',
          path: safePath(dir),
          description: `Module directory: ${dir}`,
        });
      }
    }
  }

  // Add requirements as changes
  for (const requirement of document.requirements) {
    try {
      const change = requirementToChange(requirement);
      changes.push(change);
    } catch (error) {
      errors.push(
        createError(
          'REQUIREMENT_CONVERSION_ERROR',
          `Failed to convert requirement ${requirement.id}`,
          {
            details: error,
          }
        )
      );
    }
  }

  // Add user stories as changes
  for (const story of document.userStories) {
    try {
      const change = userStoryToChange(story);
      changes.push(change);
    } catch (error) {
      errors.push(
        createError('STORY_CONVERSION_ERROR', `Failed to convert story ${story.id}`, {
          details: error,
        })
      );
    }
  }

  // If document was detected as YAML type but structured parsing failed, emit error
  const isYamlDocType = [
    BMADDocumentType.AGENT,
    BMADDocumentType.WORKFLOW,
    BMADDocumentType.TEAM,
    BMADDocumentType.MODULE,
  ].includes(document.type);
  const hasYamlData =
    document.agentYaml || document.workflowYaml || document.teamYaml || document.moduleYaml;
  if (isYamlDocType && !hasYamlData) {
    errors.push(
      createError(
        'YAML_PARSE_FAILED',
        `Document detected as ${document.type} but structured YAML parsing failed`,
        { details: { type: document.type } }
      )
    );
  }

  // If no changes found (and not a YAML document type), add warning
  if (changes.length === 0 && !isYamlDocType) {
    warnings.push(
      createWarning('NO_CHANGES', 'No requirements or user stories found in document', {
        details: { type: document.type },
      })
    );
  }

  // Extract provenance from front-matter, agent yaml, or context
  const timestamp = document.frontMatter?.date || context?.timestamp || new Date().toISOString();
  const author =
    document.frontMatter?.author ||
    context?.author ||
    (document.agentYaml ? document.agentYaml.metadata.name : 'unknown');
  const version = document.frontMatter?.version || '1.0.0';

  // Build APS plan
  const plan: APSPlan = {
    id: planId,
    schema_version: '0.1.0',
    intent: document.intent || 'BMAD document conversion',
    proposed_changes: changes,
    provenance: {
      timestamp,
      author,
      source: 'cli',
      version,
      repository: context?.repositoryPath,
      branch: context?.branch,
      commit: context?.commit,
    },
    validations: {
      required_checks: ['lint', 'test', 'coverage'],
      skip_checks: [],
    },
    // Note: hash will be generated by the adapter after plan creation
    hash: '0000000000000000000000000000000000000000000000000000000000000000',
  };

  return plan;
}

export function parseBMAD(content: string, context?: ParseContext): APSPlan {
  const document = parseBMADDocument(content);
  return bmadToAPS(document, context, content);
}
