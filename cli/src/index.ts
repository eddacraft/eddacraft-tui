#!/usr/bin/env node

import { Command } from 'commander';
import { createCheckCommand } from './commands/check.js';
import { createGateCommand } from './commands/gate.js';
import { createGateConfigCommand } from './commands/gate-config.js';
import { createPlanCommand } from './commands/plan.js';
import { createValidateCommand } from './commands/validate.js';
import { createExportCommand } from './commands/export.js';
import { createInitCommand } from './commands/init.js';
import { createHooksCommand } from './commands/hooks.js';
import { createPolicyCommand } from './commands/policy.js';
import { createWatchCommand } from './commands/watch.js';
import { createStatusCommand } from './commands/status.js';
import { readFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

// Get package.json from the CLI package directory, not the user's cwd
const __dirname = dirname(fileURLToPath(import.meta.url));
const packageJson = JSON.parse(readFileSync(join(__dirname, '..', 'package.json'), 'utf-8'));

const program = new Command();

program
  .name('anvil')
  .description('Anvil - Deterministic development automation platform')
  .version(packageJson.version);

// Register commands
program.addCommand(createCheckCommand());
program.addCommand(createInitCommand());
program.addCommand(createGateCommand());
program.addCommand(createGateConfigCommand());
program.addCommand(createPlanCommand());
program.addCommand(createValidateCommand());
program.addCommand(createExportCommand());
program.addCommand(createHooksCommand());
program.addCommand(createPolicyCommand());
program.addCommand(createWatchCommand());
program.addCommand(createStatusCommand());

// Parse command line arguments
program.parse();
