/**
 * Agent Command Group
 *
 * Commands for multi-agent coordination and management.
 */

import { Command } from 'commander';
import { createAgentStatusCommand } from './status.js';
import { createAgentListCommand } from './list.js';
import { createAgentCleanupCommand } from './cleanup.js';
import { createAgentInfoCommand } from './info.js';

export function createAgentCommand(): Command {
  const command = new Command('agent');

  command
    .description('Multi-agent coordination and management')
    .addCommand(createAgentStatusCommand())
    .addCommand(createAgentListCommand())
    .addCommand(createAgentCleanupCommand())
    .addCommand(createAgentInfoCommand());

  return command;
}
