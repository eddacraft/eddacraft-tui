/**
 * Stack Command (STACK-013, STACK-014)
 *
 * Manage Edda Stack configuration and state.
 *
 * The Edda Stack is a three-layer architecture:
 * - Kindling: Observation layer (captures activity)
 * - Ember: Candidate layer (proposes meaning)
 * - Edda: Memory layer (preserves truths)
 *
 * Subcommands:
 *   anvil stack status    Show stack health and status
 *   anvil stack validate  Validate stack configuration
 */

import { Command } from 'commander';
import { createStatusSubcommand, createValidateSubcommand } from './stack/index.js';

/**
 * Create the stack command with subcommands
 */
export function createStackCommand(): Command {
  const stackCommand = new Command('stack')
    .description('Manage Edda Stack configuration and state')
    .addCommand(createStatusSubcommand())
    .addCommand(createValidateSubcommand());

  return stackCommand;
}
