import { Command } from 'commander';
import { createEmberListCommand } from './list.js';
import { createEmberShowCommand } from './show.js';
import { createEmberPromoteCommand } from './promote.js';

export function createEmberCommand(): Command {
  const command = new Command('ember');

  command
    .description('Manage Ember candidate proposals')
    .addCommand(createEmberListCommand())
    .addCommand(createEmberShowCommand())
    .addCommand(createEmberPromoteCommand());

  return command;
}
