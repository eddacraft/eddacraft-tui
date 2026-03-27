import { Command } from 'commander';
import {
  createPolicyListCommand,
  createPolicyExplainCommand,
  createPolicyWhyCommand,
  createPolicyDiffCommand,
  createPolicyDisableCommand,
  createPolicyEnableCommand,
  createPolicyDocCommand,
  createPolicyScaffoldCommand,
  createPolicyValidateCommand,
  createPolicyTestCommand,
  createPolicyInitCommand,
  createPolicyBundleCommand,
} from './policy/index.js';

export function createPolicyCommand(): Command {
  const command = new Command('policy');

  command
    .description('Manage OPA/Rego policies')
    .addCommand(createPolicyListCommand())
    .addCommand(createPolicyExplainCommand())
    .addCommand(createPolicyWhyCommand())
    .addCommand(createPolicyDiffCommand())
    .addCommand(createPolicyDisableCommand())
    .addCommand(createPolicyEnableCommand())
    .addCommand(createPolicyDocCommand())
    .addCommand(createPolicyScaffoldCommand())
    .addCommand(createPolicyValidateCommand())
    .addCommand(createPolicyTestCommand())
    .addCommand(createPolicyInitCommand())
    .addCommand(createPolicyBundleCommand());

  return command;
}
