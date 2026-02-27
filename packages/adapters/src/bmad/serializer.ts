/**
 * BMAD Serializer
 *
 * Serializes APS plans to BMAD v6 format documents.
 * Uses v6 conventions: {project-root} hyphenated variable syntax,
 * _bmad folder references.
 */

import type { APSPlan, Change } from '@eddacraft/anvil-core';
import { BMAD_UPSTREAM_VERSION } from './types.js';

/**
 * Serialize APS plan to BMAD format
 *
 * Generates a BMAD PRD document from an APS plan.
 *
 * @param plan - APS plan to serialize
 * @returns BMAD markdown content
 */
export function serializeToBMAD(plan: APSPlan): string {
  const lines: string[] = [];

  // Generate YAML front-matter (v6 format)
  lines.push('---');
  lines.push('name: "Product Requirements Document"');
  lines.push(`version: "${plan.provenance.version || '1.0.0'}"`);
  lines.push(`date: "${plan.provenance.timestamp}"`);
  lines.push(`author: "${plan.provenance.author || 'unknown'}"`);
  lines.push('description: "Generated from Anvil Plan Specification"');
  const outputName = plan.metadata?.['outputFile'] || 'docs/PRD.md';
  lines.push(`output_file: "{project-root}/${outputName}"`);
  lines.push('---');
  lines.push('');

  // Generate document header
  const projectName = plan.metadata?.['projectName'] || 'Project';
  lines.push(`# ${projectName} - Product Requirements Document`);
  lines.push('');
  lines.push(`**Author:** ${plan.provenance.author || 'unknown'}`);
  lines.push(`**Date:** ${plan.provenance.timestamp}`);
  lines.push(`**Version:** ${plan.provenance.version || '1.0.0'}`);
  lines.push('');

  // Generate change log
  lines.push('## Change Log');
  lines.push('');
  lines.push('| Date | Version | Description | Author |');
  lines.push('| :--- | :------ | :---------- | :----- |');

  const date = new Date(plan.provenance.timestamp).toISOString().split('T')[0];
  const version = plan.provenance.version || '1.0.0';
  const author = plan.provenance.author || 'unknown';

  lines.push(`| ${date} | ${version} | Initial version | ${author} |`);
  lines.push('');

  // Generate overview/intent section
  lines.push('## Overview');
  lines.push('');
  lines.push(plan.intent);
  lines.push('');

  // Categorize changes
  const functionalRequirements: Change[] = [];
  const nonFunctionalRequirements: Change[] = [];
  const userStories: Change[] = [];

  for (const change of plan.proposed_changes) {
    // Categorize based on change type and path
    if (change.path.includes('stories/') || change.description.match(/^US-\d{2}/)) {
      userStories.push(change);
    } else if (
      change.type === 'config_update' ||
      change.type === 'dependency_add' ||
      change.type === 'dependency_update' ||
      change.description.match(/^NFR-\d{2}/)
    ) {
      nonFunctionalRequirements.push(change);
    } else {
      functionalRequirements.push(change);
    }
  }

  // Generate Functional Requirements section
  if (functionalRequirements.length > 0) {
    lines.push('## Functional Requirements');
    lines.push('');

    functionalRequirements.forEach((change, index) => {
      const reqNum = String(index + 1).padStart(2, '0');
      const reqId = `FR-${reqNum}`;

      // Extract description (remove existing FR-XX if present)
      let description = change.description.replace(/^FR-\d{2}:\s*/, '');

      // Add path context if not already in description
      if (!description.includes(change.path)) {
        description += ` (${change.path})`;
      }

      lines.push(`${reqId}: ${description}`);
    });

    lines.push('');
  }

  // Generate Non-Functional Requirements section
  if (nonFunctionalRequirements.length > 0) {
    lines.push('## Non-Functional Requirements');
    lines.push('');

    nonFunctionalRequirements.forEach((change, index) => {
      const reqNum = String(index + 1).padStart(2, '0');
      const reqId = `NFR-${reqNum}`;

      // Extract description
      const description = change.description.replace(/^NFR-\d{2}:\s*/, '');

      lines.push(`${reqId}: ${description}`);
    });

    lines.push('');
  }

  // Add validation requirements as NFRs
  if (plan.validations.required_checks.length > 0) {
    if (nonFunctionalRequirements.length === 0) {
      lines.push('## Non-Functional Requirements');
      lines.push('');
    }

    const startIndex = nonFunctionalRequirements.length + 1;
    plan.validations.required_checks.forEach((check, index) => {
      const reqNum = String(startIndex + index).padStart(2, '0');
      const reqId = `NFR-${reqNum}`;
      lines.push(`${reqId}: Must pass ${check} validation`);
    });

    lines.push('');
  }

  // Generate User Stories section
  if (userStories.length > 0) {
    lines.push('## User Stories');
    lines.push('');

    userStories.forEach((story, index) => {
      const storyNum = String(index + 1).padStart(2, '0');
      const storyId = `US-${storyNum}`;

      // Extract story title
      let title = story.description.replace(/^US-\d{2}:\s*/, '');

      // Check if description contains "As a... I want... so that..." pattern
      const storyMatch = title.match(/\(As a (.+?), I want (.+?), so that (.+?)\)/);

      if (storyMatch) {
        const [, userType, action, benefit] = storyMatch;
        title = title.replace(/\(As a .+?\)/, '').trim();

        lines.push(`### ${storyId}: ${title}`);
        lines.push('');
        lines.push(`As a ${userType},`);
        lines.push(`I want ${action},`);
        lines.push(`so that ${benefit}.`);
        lines.push('');

        // Extract acceptance criteria if present
        const criteriaMatch = title.match(/- Acceptance criteria: (.+)$/);
        if (criteriaMatch) {
          const criteria = criteriaMatch[1].split(';').map((c) => c.trim());
          lines.push('**Acceptance Criteria:**');
          lines.push('');
          criteria.forEach((criterion, i) => {
            lines.push(`${i + 1}. ${criterion}`);
          });
          lines.push('');
        }
      } else {
        lines.push(`### ${storyId}: ${title}`);
        lines.push('');
      }
    });
  }

  // Add repository information if available
  if (plan.provenance.repository || plan.provenance.branch) {
    lines.push('## Repository Information');
    lines.push('');

    if (plan.provenance.repository) {
      lines.push(`**Repository:** ${plan.provenance.repository}`);
    }
    if (plan.provenance.branch) {
      lines.push(`**Branch:** ${plan.provenance.branch}`);
    }
    if (plan.provenance.commit) {
      lines.push(`**Commit:** ${plan.provenance.commit}`);
    }

    lines.push('');
  }

  // Add footer
  lines.push('---');
  lines.push('');
  lines.push(
    `*Generated by Anvil (BMAD v${BMAD_UPSTREAM_VERSION} compatible) - https://github.com/EddaCraft/anvil-001*`
  );
  lines.push('');

  return lines.join('\n');
}
