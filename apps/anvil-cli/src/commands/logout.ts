import { Command } from 'commander';
import { clearAuth, loadAuth } from '../services/auth-store.js';
import { success, info } from '../utils/output.js';

export function createLogoutCommand(): Command {
  const command = new Command('logout');

  command.description('Clear stored beta access credentials').action(() => {
    const existing = loadAuth();
    clearAuth();

    if (existing) {
      success(`Logged out (was ${existing.user.email})`);
    } else {
      info('No active session');
    }
  });

  return command;
}
