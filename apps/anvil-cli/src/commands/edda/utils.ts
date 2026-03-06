import chalk from 'chalk';
import type { MemoryStatus } from '@eddacraft/anvil-edda-stack';

export function colourStatus(status: MemoryStatus | string): string {
  switch (status) {
    case 'active':
      return chalk.cyan(status);
    case 'superseded':
      return chalk.yellow(status);
    case 'retired':
      return chalk.red(status);
    default:
      return status;
  }
}

export function colourConfidence(confidence: 'low' | 'medium' | 'high'): string {
  switch (confidence) {
    case 'high':
      return chalk.green(confidence);
    case 'medium':
      return chalk.yellow(confidence);
    case 'low':
      return chalk.red(confidence);
    default:
      return confidence;
  }
}
