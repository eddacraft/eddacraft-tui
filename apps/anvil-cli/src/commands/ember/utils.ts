import chalk from 'chalk';
import type { ProposalStatus } from '@eddacraft/anvil-edda-stack';

export function colourConfidence(confidence: number): string {
  const text = confidence.toFixed(2);
  if (confidence > 0.7) {
    return chalk.green(text);
  }
  if (confidence >= 0.4) {
    return chalk.yellow(text);
  }
  return chalk.red(text);
}

export function colourStatus(status: ProposalStatus | string): string {
  switch (status) {
    case 'active':
      return chalk.cyan(status);
    case 'promoted':
      return chalk.green(status);
    case 'dismissed':
      return chalk.yellow(status);
    case 'expired':
      return chalk.red(status);
    default:
      return status;
  }
}
