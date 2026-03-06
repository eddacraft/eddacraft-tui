import { Command } from 'commander';
import { createEddaListCommand } from './list.js';
import { createEddaShowCommand } from './show.js';
import { createEddaPromoteCommand } from './promote.js';
import { createEddaRetireCommand } from './retire.js';
import { createEddaTraceCommand } from './trace.js';

export function createEddaCommand(): Command {
  const command = new Command('edda');
  command
    .description('Manage Edda canonical memories')
    .addCommand(createEddaListCommand())
    .addCommand(createEddaShowCommand())
    .addCommand(createEddaPromoteCommand())
    .addCommand(createEddaRetireCommand())
    .addCommand(createEddaTraceCommand());
  return command;
}
