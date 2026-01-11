/**
 * Generic Markdown Serializer
 *
 * Serializes APS plans to generic markdown format.
 */

import type { APSPlan } from '@anvil/core';

/**
 * Type guard to check if value is a string
 */
function isString(value: unknown): value is string {
  return typeof value === 'string';
}

/**
 * Type guard to check if value is a string array
 */
function isStringArray(value: unknown): value is string[] {
  return Array.isArray(value) && value.every((item) => typeof item === 'string');
}

/**
 * Serialize APS plan to generic markdown format
 */
export function serializeToGeneric(plan: APSPlan): string {
  const lines: string[] = [];

  // Title (from intent or metadata) - safely check if title is a non-empty string
  const metadataTitle = plan.metadata?.title;
  const title = isString(metadataTitle) && metadataTitle ? metadataTitle : plan.intent;
  lines.push(`# ${title}`);
  lines.push('');

  // Overview section - safely check if overview is a string
  const metadataOverview = plan.metadata?.overview;
  if (metadataOverview && isString(metadataOverview)) {
    lines.push('## Overview');
    lines.push('');
    lines.push(metadataOverview);
    lines.push('');
  } else if (plan.intent) {
    lines.push('## Purpose');
    lines.push('');
    lines.push(plan.intent);
    lines.push('');
  }

  // Goals section - safely check if goals is a string array
  const metadataGoals = plan.metadata?.goals;
  if (metadataGoals && isStringArray(metadataGoals)) {
    lines.push('## Goals');
    lines.push('');
    for (const goal of metadataGoals) {
      lines.push(`- ${goal}`);
    }
    lines.push('');
  }

  // Changes section
  if (plan.proposed_changes.length > 0) {
    lines.push('## Changes');
    lines.push('');

    // Group changes by type
    const creates = plan.proposed_changes.filter((c) => c.type === 'file_create');
    const updates = plan.proposed_changes.filter((c) => c.type === 'file_update');
    const deletes = plan.proposed_changes.filter((c) => c.type === 'file_delete');
    const configs = plan.proposed_changes.filter((c) => c.type === 'config_update');

    if (creates.length > 0) {
      lines.push('### Files to Create');
      lines.push('');
      for (const change of creates) {
        lines.push(`- **${change.path}**: ${change.description}`);
      }
      lines.push('');
    }

    if (updates.length > 0) {
      lines.push('### Files to Update');
      lines.push('');
      for (const change of updates) {
        lines.push(`- **${change.path}**: ${change.description}`);
      }
      lines.push('');
    }

    if (configs.length > 0) {
      lines.push('### Configuration Changes');
      lines.push('');
      for (const change of configs) {
        lines.push(`- **${change.path}**: ${change.description}`);
      }
      lines.push('');
    }

    if (deletes.length > 0) {
      lines.push('### Files to Delete');
      lines.push('');
      for (const change of deletes) {
        lines.push(`- **${change.path}**: ${change.description}`);
      }
      lines.push('');
    }
  }

  // Validation requirements
  if (plan.validations.required_checks.length > 0) {
    lines.push('## Validation Requirements');
    lines.push('');
    for (const check of plan.validations.required_checks) {
      lines.push(`- ${check}`);
    }
    lines.push('');
  }

  // Metadata section
  if (plan.provenance) {
    lines.push('## Metadata');
    lines.push('');
    lines.push(`- **Author**: ${plan.provenance.author || 'Unknown'}`);
    lines.push(`- **Created**: ${plan.provenance.timestamp}`);
    if (plan.provenance.repository) {
      lines.push(`- **Repository**: ${plan.provenance.repository}`);
    }
    if (plan.provenance.branch) {
      lines.push(`- **Branch**: ${plan.provenance.branch}`);
    }
    lines.push('');
  }

  return lines.join('\n');
}
