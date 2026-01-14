/**
 * Evidence Writer Service
 *
 * Injects gate execution evidence back into source documents.
 * Supports different formats (SpecKit, BMAD, etc.) through adapters.
 *
 * @module cli/services/evidence-writer
 */

import { writeFile, readFile } from 'node:fs/promises';
import type { APSPlan, Evidence } from '@anvil/core';
import type { GateRunResult } from '@anvil/core';

/**
 * Options for evidence injection
 */
export interface EvidenceWriteOptions {
  /** Source format (speckit, bmad, etc.) */
  format: string;
  /** Source file path */
  filePath: string;
  /** Gate execution results */
  gateResults: GateRunResult;
  /** Original plan with metadata */
  plan: APSPlan;
  /** Whether to append or replace evidence section */
  mode?: 'append' | 'replace';
}

/**
 * Result of evidence writing operation
 */
export interface EvidenceWriteResult {
  /** Whether write succeeded */
  success: boolean;
  /** Updated file path */
  filePath?: string;
  /** Error message if failed */
  error?: string;
  /** Evidence bundle that was written */
  evidence?: Evidence;
}

/**
 * Evidence writer service
 *
 * Handles injecting gate results back into source documents.
 */
export class EvidenceWriter {
  /**
   * Write evidence to source document
   *
   * @param options - Write options
   * @returns Write result
   */
  async writeEvidence(options: EvidenceWriteOptions): Promise<EvidenceWriteResult> {
    try {
      // Create evidence bundle from gate results
      const evidence = this.createEvidenceBundle(options.gateResults);

      // Format-specific evidence injection
      switch (options.format.toLowerCase()) {
        case 'speckit':
        case 'spec-kit':
        case 'spec.md':
          return await this.writeSpecKitEvidence(options, evidence);
        case 'bmad':
          return await this.writeBMADEvidence(options, evidence);
        default:
          return {
            success: false,
            error: `Evidence injection not supported for format: ${options.format}. Supported formats: speckit, bmad`,
          };
      }
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Unknown error writing evidence',
      };
    }
  }

  /**
   * Create evidence bundle from gate results
   */
  private createEvidenceBundle(results: GateRunResult): Evidence {
    return {
      gate_version: '1.0.0',
      timestamp: new Date().toISOString(),
      overall_status: results.overall ? 'passed' : 'failed',
      checks: results.checks.map((check) => ({
        check: check.check,
        status: check.skipped ? 'skipped' : check.passed ? 'passed' : 'failed',
        timestamp: new Date().toISOString(),
        message: check.message,
        details: check.details,
      })),
      summary: this.generateSummary(results),
    };
  }

  /**
   * Generate human-readable summary
   */
  private generateSummary(results: GateRunResult): string {
    const { total, passed, failed, skipped } = results.summary;
    const percentage = total > 0 ? Math.round((passed / total) * 100) : 0;

    return `Gate execution completed: ${passed}/${total} checks passed (${percentage}%), ${failed} failed, ${skipped} skipped`;
  }

  /**
   * Write evidence to SpecKit document
   */
  private async writeSpecKitEvidence(
    options: EvidenceWriteOptions,
    evidence: Evidence
  ): Promise<EvidenceWriteResult> {
    try {
      // Read original content
      const content = await readFile(options.filePath, 'utf-8');

      // Generate evidence markdown
      const evidenceMarkdown = this.generateSpecKitEvidenceMarkdown(evidence, options.gateResults);

      // Check if evidence section already exists
      const evidenceRegex = /^## Gate Evidence\s*\n(?:.*\n)*?(?=^##|\n*$)/m;
      let updatedContent: string;

      if (evidenceRegex.test(content)) {
        if (options.mode === 'replace') {
          // Replace existing evidence section
          updatedContent = content.replace(evidenceRegex, evidenceMarkdown);
        } else {
          // Append mode: add timestamp to distinguish runs
          const appendSection = this.generateSpecKitAppendSection(evidence, options.gateResults);
          updatedContent = content.replace(
            evidenceRegex,
            (match) => match.trimEnd() + '\n\n' + appendSection
          );
        }
      } else {
        // Append new evidence section at the end
        updatedContent = content.trimEnd() + '\n\n' + evidenceMarkdown;
      }

      // Write back to file
      await writeFile(options.filePath, updatedContent, 'utf-8');

      return {
        success: true,
        filePath: options.filePath,
        evidence,
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Failed to write SpecKit evidence',
      };
    }
  }

  /**
   * Generate SpecKit evidence markdown section
   */
  private generateSpecKitEvidenceMarkdown(evidence: Evidence, results: GateRunResult): string {
    const lines: string[] = [];

    // Section header
    lines.push('## Gate Evidence');
    lines.push('');

    // Overall status
    const statusEmoji = evidence.overall_status === 'passed' ? '✅' : '❌';
    lines.push(`**Status**: ${statusEmoji} ${evidence.overall_status.toUpperCase()}`);
    lines.push(`**Executed**: ${new Date(evidence.timestamp).toLocaleString()}`);
    lines.push(`**Score**: ${results.score.toFixed(1)}%`);
    lines.push('');

    // Summary
    lines.push('### Summary');
    lines.push('');
    lines.push(evidence.summary || 'No summary available');
    lines.push('');

    // Individual check results
    lines.push('### Check Results');
    lines.push('');

    for (const check of evidence.checks) {
      const emoji = check.status === 'passed' ? '✅' : check.status === 'failed' ? '❌' : '⏭️';
      lines.push(`#### ${emoji} ${check.check}`);
      lines.push('');
      lines.push(`- **Status**: ${check.status}`);
      if (check.message) {
        lines.push(`- **Message**: ${check.message}`);
      }
      if (check.details && Object.keys(check.details).length > 0) {
        lines.push('- **Details**:');
        lines.push('  ```json');
        lines.push('  ' + JSON.stringify(check.details, null, 2).split('\n').join('\n  '));
        lines.push('  ```');
      }
      lines.push('');
    }

    return lines.join('\n');
  }

  /**
   * Generate append section for existing evidence (multiple runs)
   */
  private generateSpecKitAppendSection(evidence: Evidence, results: GateRunResult): string {
    const lines: string[] = [];

    // Run header with timestamp
    const timestamp = new Date(evidence.timestamp).toLocaleString();
    const statusEmoji = evidence.overall_status === 'passed' ? '✅' : '❌';
    lines.push(`### Run: ${timestamp}`);
    lines.push('');
    lines.push(`**Status**: ${statusEmoji} ${evidence.overall_status.toUpperCase()}`);
    lines.push(`**Score**: ${results.score.toFixed(1)}%`);
    lines.push('');

    // Brief summary of checks
    const passed = evidence.checks.filter((c) => c.status === 'passed').length;
    const failed = evidence.checks.filter((c) => c.status === 'failed').length;
    const skipped = evidence.checks.filter((c) => c.status === 'skipped').length;

    lines.push(`- ✅ Passed: ${passed}`);
    lines.push(`- ❌ Failed: ${failed}`);
    lines.push(`- ⏭️ Skipped: ${skipped}`);
    lines.push('');

    return lines.join('\n');
  }

  /**
   * Write evidence to BMAD document
   */
  private async writeBMADEvidence(
    options: EvidenceWriteOptions,
    evidence: Evidence
  ): Promise<EvidenceWriteResult> {
    try {
      // Read original content
      const content = await readFile(options.filePath, 'utf-8');

      // Generate evidence markdown (similar to SpecKit)
      const evidenceMarkdown = this.generateBMADEvidenceMarkdown(evidence, options.gateResults);

      // Check if evidence section already exists
      const evidenceRegex = /^## Validation Evidence\s*\n(?:.*\n)*?(?=^##|\n*$)/m;
      let updatedContent: string;

      if (evidenceRegex.test(content)) {
        if (options.mode === 'replace') {
          updatedContent = content.replace(evidenceRegex, evidenceMarkdown);
        } else {
          const appendSection = this.generateSpecKitAppendSection(evidence, options.gateResults);
          updatedContent = content.replace(
            evidenceRegex,
            (match) => match.trimEnd() + '\n\n' + appendSection
          );
        }
      } else {
        updatedContent = content.trimEnd() + '\n\n' + evidenceMarkdown;
      }

      await writeFile(options.filePath, updatedContent, 'utf-8');

      return {
        success: true,
        filePath: options.filePath,
        evidence,
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Failed to write BMAD evidence',
      };
    }
  }

  /**
   * Generate BMAD evidence markdown section
   */
  private generateBMADEvidenceMarkdown(evidence: Evidence, results: GateRunResult): string {
    // Use same format as SpecKit but with BMAD-appropriate title
    return this.generateSpecKitEvidenceMarkdown(evidence, results).replace(
      '## Gate Evidence',
      '## Validation Evidence'
    );
  }
}
