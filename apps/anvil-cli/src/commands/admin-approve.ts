/**
 * Admin Approve Command (BAUTH-016)
 *
 * Approve waitlisted users for beta access.
 *
 * Usage:
 *   anvil admin approve <email>
 *   anvil admin approve --batch <n>
 */

import { Command } from 'commander';
import { z } from 'zod';
import { adminApprove } from '../services/admin-client.js';
import { success, error, blank, print } from '../utils/output.js';
import { CliError } from '../utils/cli-error.js';
import { coercePositiveInt } from '../utils/option-coerce.js';

const emailSchema = z.string().email();

export function createAdminCommand(): Command {
  const command = new Command('admin');

  command.description('Admin operations (requires ANVIL_ADMIN_KEY)');

  command
    .command('approve [email]')
    .description('Approve waitlisted user(s) for beta access')
    .option('--batch <n>', 'Approve the oldest N unapproved waitlist entries')
    .action(async (email: string | undefined, options: { batch?: string }) => {
      if (!email && !options.batch) {
        error('Provide an <email> argument or --batch <n>');
        throw new CliError('Missing email or --batch');
      }

      if (email && options.batch) {
        error('Provide either <email> or --batch, not both');
        throw new CliError('Conflicting arguments');
      }

      if (email && !emailSchema.safeParse(email).success) {
        error('Argument must be a valid email address');
        throw new CliError('Invalid email address');
      }

      try {
        const params = email ? { email } : { batch: coercePositiveInt(options.batch!, '--batch') };

        const result = await adminApprove(params);

        if (result.approved.length === 0) {
          print('No users to approve.');
          return;
        }

        for (const entry of result.approved) {
          const expires = new Date(entry.expiresAt).toISOString().slice(0, 10);
          success(`Approved ${entry.email} (expires ${expires})`);
        }

        blank();
        print(`${result.approved.length} user(s) approved.`);
      } catch (err) {
        if (err instanceof CliError) throw err;
        error(err instanceof Error ? err.message : 'Failed to approve user(s)');
        throw new CliError('Failed to approve user(s)');
      }
    });

  return command;
}
