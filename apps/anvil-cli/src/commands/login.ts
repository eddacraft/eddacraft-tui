import { Command } from 'commander';
import chalk from 'chalk';
import inquirer from 'inquirer';
import { verifyToken } from '../services/auth-client.js';
import { saveAuth, loadAuth } from '../services/auth-store.js';
import { success, error, info } from '../utils/output.js';

export function createLoginCommand(): Command {
  const command = new Command('login');

  command
    .description('Authenticate with a beta access token')
    .option('--token <token>', 'Beta access token (or enter interactively)')
    .action(async (options: { token?: string }) => {
      // Check if already authenticated
      const existing = loadAuth();
      if (existing) {
        info(`Already authenticated as ${chalk.bold(existing.user.email)}`);
        const { proceed } = await inquirer.prompt([
          {
            type: 'confirm',
            name: 'proceed',
            message: 'Re-authenticate with a new token?',
            default: false,
          },
        ]);
        if (!proceed) return;
      }

      let token = options.token;

      if (!token) {
        const answers = await inquirer.prompt([
          {
            type: 'password',
            name: 'token',
            message: 'Enter your beta access token:',
            mask: '*',
            validate: (input: string) =>
              input.startsWith('anvil_beta_') || 'Token should start with anvil_beta_',
          },
        ]);
        token = answers.token as string;
      }

      try {
        info('Verifying token...');
        const result = await verifyToken(token);

        if (!result.valid || !result.user || !result.scopes || !result.expiresAt) {
          error('Invalid or expired token. Please check your token and try again.');
          process.exit(1);
        }

        saveAuth({
          token,
          user: result.user,
          scopes: result.scopes,
          expiresAt: result.expiresAt,
          verifiedAt: new Date().toISOString(),
        });

        success(`Authenticated as ${chalk.bold(result.user.email)}`);
        info(`Scopes: ${result.scopes.join(', ')}`);
        info(`Expires: ${new Date(result.expiresAt).toLocaleString()}`);
      } catch (err) {
        error(err instanceof Error ? err.message : 'Failed to verify token');
        process.exit(1);
      }
    });

  return command;
}
