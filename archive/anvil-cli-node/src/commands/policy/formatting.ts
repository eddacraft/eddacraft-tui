import chalk from 'chalk';
import type { EnforcementLevel } from '../../services/policy-config.js';
import type { ResolvedPolicy } from '../../services/policy-config.js';
import { blank, print } from '../../utils/output.js';

export function formatEnforcement(level: EnforcementLevel): string {
  switch (level) {
    case 'block':
      return chalk.red('block');
    case 'warn':
      return chalk.yellow('warn');
    case 'info':
      return chalk.blue('info');
    case 'off':
      return chalk.dim('off');
  }
}

export function formatSource(source: ResolvedPolicy['source']): string {
  switch (source) {
    case 'org':
      return chalk.magenta('org');
    case 'team':
      return chalk.cyan('team');
    case 'local':
      return chalk.green('local');
    case 'starter':
      return chalk.dim('starter');
    case 'bundle':
      return chalk.blue('bundle');
  }
}

export function truncate(str: string, maxLen: number): string {
  if (str.length <= maxLen) return str;
  return str.slice(0, maxLen - 1) + '…';
}

export function printWhyBlock(policy: ResolvedPolicy): void {
  blank();
  print(`  ${chalk.red('✗')} ${chalk.bold(policy.name)}: ${formatEnforcement(policy.enforcement)}`);
  blank();

  if (policy.reason) {
    print(`  ${chalk.bold('Why:')} ${policy.reason}`);
  } else {
    print(chalk.dim('  No business reason documented for this policy.'));
    print(chalk.dim('  Add a "reason" field in .anvil/config.yml to document it.'));
  }

  if (policy.owner) {
    print(`  ${chalk.bold('Owner:')} ${policy.owner}`);
  }

  print(`  ${chalk.bold('Source:')} ${formatSource(policy.source)} policy`);

  blank();
  print(chalk.dim(`  anvil policy explain ${policy.name}    # full details`));
  print(chalk.dim(`  anvil policy disable ${policy.name}    # turn it off`));
  print(chalk.dim(`  anvil gate --skip ${policy.name}       # skip just this once`));
  blank();
}
