import { Command } from 'commander';
import { clearAuth, loadAuth } from '../services/auth-store.js';
import { clearLicence } from '../services/licence-store.js';
import { success, info } from '../utils/output.js';

export function createLogoutCommand(): Command {
  const command = new Command('logout');

  command.description('Clear stored credentials').action(() => {
    const existing = loadAuth();
    clearAuth();
    clearLicence();

    if (existing) {
      success('Logged out. Local credentials removed.');
    } else {
      info('No active session');
    }
  });

  return command;
}
