/**
 * BMAD Parser
 *
 * Parses BMAD format documents (PRD, Architecture, Epics, Stories) into APS plans.
 */

import type { APSPlan, Change } from '@anvil/core';
import type { ParseContext, AdapterError, AdapterWarning } from '../base/types.js';
import { BMADDocument, BMADRequirement, BMADUserStory, RequirementType } from './types.js';
import {
  extractFrontMatter,
  extractRequirements,
  extractUserStories,
  extractChangeLog,
  identifyDocumentType,
  extractTitle,
  extractIntent,
} from './utils.js';
import { createError, createWarning } from '../base/utils.js';

/**
 * Parse BMAD document into internal structure
 *
 * @param content - BMAD markdown content
 * @returns Parsed document
 */
export function parseBMADDocument(content: string): BMADDocument {
  const frontMatter = extractFrontMatter(content);
  const docType = identifyDocumentType(content, frontMatter);
  const requirements = extractRequirements(content);
  const userStories = extractUserStories(content);
  const changeLog = extractChangeLog(content);
  const title = extractTitle(content);
  const intent = extractIntent(content, docType);

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
        path: `features/${requirement.id.toLowerCase()}.ts`,
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
        path: `features/stories/${requirement.id.toLowerCase()}.ts`,
        description: `${requirement.id}: ${requirement.description}`,
      };

    default:
      return {
        type: 'file_create',
        path: `requirements/${requirement.id.toLowerCase()}.md`,
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
    path: `features/stories/${story.id.toLowerCase()}.ts`,
    description,
  };
}

/**
 * Convert BMAD document to APS plan
 *
 * @param document - Parsed BMAD document
 * @param context - Parse context
 * @returns APS plan
 */
export function bmadToAPS(document: BMADDocument, context?: ParseContext): APSPlan {
  // Generate plan ID
  const planId = `aps-${Math.random().toString(16).substring(2, 10)}`;

  // Convert requirements and stories to changes
  const changes: Change[] = [];
  const errors: AdapterError[] = [];
  const warnings: AdapterWarning[] = [];

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

  // If no changes found, add warning
  if (changes.length === 0) {
    warnings.push(
      createWarning('NO_CHANGES', 'No requirements or user stories found in document', {
        details: { type: document.type },
      })
    );
  }

  // Extract provenance from front-matter or context
  const timestamp = document.frontMatter?.date || context?.timestamp || new Date().toISOString();
  const author = document.frontMatter?.author || context?.author || 'unknown';
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

/**
 * Parse BMAD content to APS plan (main entry point)
 *
 * @param content - BMAD markdown content
 * @param context - Parse context
 * @returns APS plan
 */
export function parseBMAD(content: string, context?: ParseContext): APSPlan {
  const document = parseBMADDocument(content);
  return bmadToAPS(document, context);
}
