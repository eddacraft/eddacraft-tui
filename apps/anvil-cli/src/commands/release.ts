import { Command } from 'commander';
import type { ReleaseConfig } from '../services/release-types.js';
import { runRelease } from '../services/release-runner.js';

interface ReleaseOptions {
  profile: string;
  target?: string;
  execute: boolean;
  resume: boolean;
  skipPreflight: boolean;
  verbose: boolean;
  json: boolean;
}

export function createReleaseCommand(): Command {
  const command = new Command('release');

  command
    .description('Interactive release workflow for the Anvil CLI')
    .option('--profile <name>', 'Release profile: beta (default), stable, hotfix', 'beta')
    .option('--target <version>', 'Target version (skip interactive prompt)')
    .option('--execute', 'Actually perform changes (default: dry-run)', false)
    .option('--resume', 'Resume from last saved state', false)
    .option('--skip-preflight', 'Skip preflight checks', false)
    .option('-v, --verbose', 'Show full command output', false)
    .option('--json', 'Output progress as JSON', false)
    .action(async (options: ReleaseOptions) => {
      const config: ReleaseConfig = {
        execute: options.execute,
        verbose: options.verbose,
        profile: options.profile,
        skipPreflight: options.skipPreflight,
        targetVersion: options.target,
        resume: options.resume,
      };

      await runRelease(config);
    });

  return command;
}
