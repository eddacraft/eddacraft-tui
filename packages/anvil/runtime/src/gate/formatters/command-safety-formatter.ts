import type {
  CommandSafetyFinding,
  CommandAnalysisSummary,
  CommandSafetyOutputConfig,
} from '../rules/types.js';
import { createDebugger } from '@eddacraft/anvil-core';

const debug = createDebugger('gate');

export interface FormatOptions {
  verbose?: boolean;
  showSuggestions?: boolean;
  showReferences?: boolean;
}

const DEFAULT_FORMAT_OPTIONS: Required<FormatOptions> = {
  verbose: true,
  showSuggestions: true,
  showReferences: true,
};

export function formatBlockedCommands(
  blocked: CommandSafetyFinding[],
  options?: FormatOptions | CommandSafetyOutputConfig
): string {
  debug('formatBlockedCommands: count=%d', blocked.length);
  if (blocked.length === 0) {
    return '';
  }

  const opts = { ...DEFAULT_FORMAT_OPTIONS, ...options };
  const lines = [`Blocked ${blocked.length} dangerous command(s):`, ''];

  for (let i = 0; i < blocked.length; i++) {
    const finding = blocked[i];
    lines.push(`${i + 1}. ${finding.command}`);
    if (opts.verbose) {
      lines.push(`   Reason: ${finding.reason}`);
    }
    if (opts.showSuggestions && finding.suggestion) {
      lines.push(`   Suggestion: ${finding.suggestion}`);
    }
    if (opts.showReferences && finding.references && finding.references.length > 0) {
      lines.push(`   Reference: ${finding.references[0]}`);
    }
    lines.push('');
  }

  return lines.join('\n');
}

export function formatWarningCommands(
  warnings: CommandSafetyFinding[],
  options?: FormatOptions | CommandSafetyOutputConfig
): string {
  debug('formatWarningCommands: count=%d', warnings.length);
  if (warnings.length === 0) {
    return '';
  }

  const opts = { ...DEFAULT_FORMAT_OPTIONS, ...options };
  const lines = [`Found ${warnings.length} potentially dangerous command(s):`, ''];

  for (let i = 0; i < warnings.length; i++) {
    const finding = warnings[i];
    lines.push(`${i + 1}. ${finding.command}`);
    if (opts.verbose) {
      lines.push(`   Reason: ${finding.reason}`);
    }
    if (opts.showSuggestions && finding.suggestion) {
      lines.push(`   Suggestion: ${finding.suggestion}`);
    }
    lines.push('');
  }

  return lines.join('\n');
}

export function formatSummary(summary: CommandAnalysisSummary): string {
  debug(
    'formatSummary: total=%d blocked=%d warned=%d',
    summary.total,
    summary.blocked,
    summary.warned
  );
  const { total, blocked, warned } = summary;

  if (total === 0) {
    return 'No commands to analyse';
  }

  if (blocked === 0 && warned === 0) {
    return `All ${total} command(s) passed safety check`;
  }

  if (blocked === 0) {
    return `${total} command(s) analysed: ${warned} warning(s)`;
  }

  return `Command safety check failed: ${blocked} blocked, ${warned} warning(s) of ${total} total`;
}
