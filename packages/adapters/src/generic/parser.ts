/**
 * Generic Markdown Parser
 *
 * Parses generic markdown documents into APS plans.
 */

import type { APSPlan, Change } from '@anvil/core';
import type { ParseContext } from '../base/types.js';
import type { GenericDocument } from './types.js';
import { parseGenericDocument } from './utils.js';

/**
 * Convert generic document item to APS change
 */
function itemToChange(item: string, type: 'requirement' | 'task' | 'feature'): Change {
  // Determine change type based on item type
  let changeType: Change['type'] = 'file_create';
  let pathPrefix = 'src';

  switch (type) {
    case 'requirement':
      changeType = 'file_create';
      pathPrefix = 'src/features';
      break;
    case 'task':
      changeType = 'file_update';
      pathPrefix = 'src/tasks';
      break;
    case 'feature':
      changeType = 'file_create';
      pathPrefix = 'src/features';
      break;
  }

  // Generate a reasonable path from the item description
  const slug = item
    .toLowerCase()
    .replace(/[^a-z0-9\s]/g, '')
    .replace(/\s+/g, '-')
    .substring(0, 50);

  return {
    type: changeType,
    path: `${pathPrefix}/${slug}.ts`,
    description: item,
  };
}

/**
 * Convert generic document to APS plan
 */
export function genericToAPS(document: GenericDocument, context?: ParseContext): APSPlan {
  // Generate plan ID
  const planId = `aps-${Math.random().toString(16).substring(2, 10)}`;

  // Collect all changes from requirements, tasks, and features
  const changes: Change[] = [];

  // Add requirements as changes
  if (document.requirements && document.requirements.length > 0) {
    changes.push(...document.requirements.map((req) => itemToChange(req, 'requirement')));
  }

  // Add tasks as changes
  if (document.tasks && document.tasks.length > 0) {
    changes.push(...document.tasks.map((task) => itemToChange(task, 'task')));
  }

  // Add features as changes
  if (document.features && document.features.length > 0) {
    changes.push(...document.features.map((feat) => itemToChange(feat, 'feature')));
  }

  // Build intent from document
  const intent =
    document.intent || document.title || document.overview?.substring(0, 200) || 'Generic plan';

  // Build provenance
  const timestamp = context?.timestamp || new Date().toISOString();
  const author = context?.author || 'unknown';

  // Create APS plan
  const plan: APSPlan = {
    id: planId,
    schema_version: '0.1.0',
    intent,
    proposed_changes: changes,
    provenance: {
      timestamp,
      author,
      source: 'cli',
      version: '1.0.0',
      repository: context?.repositoryPath,
      branch: context?.branch,
      commit: context?.commit,
    },
    validations: {
      required_checks: ['lint', 'test'],
      skip_checks: [],
    },
    metadata: {
      source_format: 'generic-markdown',
      title: document.title,
      overview: document.overview,
      goals: document.goals,
    },
    hash: '0000000000000000000000000000000000000000000000000000000000000000',
  };

  return plan;
}

/**
 * Parse generic markdown content to APS plan (main entry point)
 */
export function parseGeneric(content: string, context?: ParseContext): APSPlan {
  const document = parseGenericDocument(content);
  return genericToAPS(document, context);
}
