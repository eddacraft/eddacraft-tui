#!/usr/bin/env node

import { Command } from 'commander';
import { createCheckCommand } from './commands/check.js';
import { createDoctorCommand } from './commands/doctor.js';
import { createGateCommand } from './commands/gate.js';
import { createGateConfigCommand } from './commands/gate-config.js';
import { createNewCommand } from './commands/new.js';
import { createPlanCommand } from './commands/plan.js';
import { createValidateCommand } from './commands/validate.js';
import { createExportCommand } from './commands/export.js';
import { createInitCommand } from './commands/init.js';
import { createHooksCommand } from './commands/hooks.js';
import { createPolicyCommand } from './commands/policy.js';
import { createWatchCommand } from './commands/watch.js';
import { createStatusCommand } from './commands/status.js';
import { isFirstRun } from './services/first-run-detector.js';
import { showWelcome } from './commands/welcome.js';
import { readFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const packageJson = JSON.parse(readFileSync(join(__dirname, '..', 'package.json'), 'utf-8'));

async function main(): Promise<void> {
  const args = process.argv.slice(2);
  const hasSubcommand = args.length > 0 && !args[0].startsWith('-');
  const isHelpOrVersion =
    args.includes('--help') ||
    args.includes('-h') ||
    args.includes('--version') ||
    args.includes('-V');

  if (!hasSubcommand && !isHelpOrVersion && isFirstRun()) {
    await showWelcome();
    return;
  }

  const program = new Command();

  program
    .name('anvil')
    .description('Anvil - Deterministic development automation platform')
    .version(packageJson.version);

  program.addCommand(createCheckCommand());
  program.addCommand(createDoctorCommand());
  program.addCommand(createInitCommand());
  program.addCommand(createGateCommand());
  program.addCommand(createGateConfigCommand());
  program.addCommand(createNewCommand());
  program.addCommand(createPlanCommand());
  program.addCommand(createValidateCommand());
  program.addCommand(createExportCommand());
  program.addCommand(createHooksCommand());
  program.addCommand(createPolicyCommand());
  program.addCommand(createWatchCommand());
  program.addCommand(createStatusCommand());

  program.parse();
}

main().catch((error) => {
  console.error('Error:', error.message);
  process.exit(1);
});
