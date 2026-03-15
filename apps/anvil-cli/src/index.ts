import { Command } from 'commander';
import { createAgentCommand } from './commands/agent/index.js';
import { createArchitectureCommand } from './commands/architecture.js';
import { createAuthorshipCommand } from './commands/authorship.js';
import { createAdminCommand } from './commands/admin-approve.js';
import { createBetaCommand } from './commands/beta.js';
import { createCheckCommand } from './commands/check.js';
import { createDoctorCommand } from './commands/doctor.js';
import { createDriftCommand } from './commands/drift.js';
import { createEddaCommand } from './commands/edda/index.js';
import { createEmberCommand } from './commands/ember/index.js';
import { createExplainCommand } from './commands/explain.js';
import { createGateCommand } from './commands/gate.js';
import { createGateConfigCommand } from './commands/gate-config.js';
import { createNewCommand } from './commands/new.js';
import { createPlanCommand } from './commands/plan.js';
import { createValidateCommand } from './commands/validate.js';
import { createExportCommand } from './commands/export.js';
import { createInitCommand } from './commands/init.js';
import { createHooksCommand } from './commands/hooks.js';
import { createPolicyCommand } from './commands/policy.js';
import { createAuditCommand } from './commands/audit.js';
import { createStackCommand } from './commands/stack.js';
import { createWatchCommand } from './commands/watch.js';
import { createStatusCommand } from './commands/status.js';
import { createTutorialCommand } from './commands/tutorial.js';
import { createMcpConfigCommand } from './commands/mcp-config.js';
import { createReleaseCommand } from './commands/release.js';
import { createAuthLoginCommand } from './commands/auth-login.js';
import { createLoginCommand } from './commands/login.js';
import { createLogoutCommand } from './commands/logout.js';
import { createWhoamiCommand } from './commands/whoami.js';
import { isFirstRun } from './services/first-run-detector.js';
import { isAuthenticated, loadAuth } from './services/auth-store.js';
import { loadLicence } from './services/licence-store.js';
import { verifyLicence } from './services/licence-verifier.js';
import { scheduleRefresh } from './services/licence-refresh.js';
import { showWelcome, createStartCommand } from './commands/welcome.js';
import { CliError, CliExit } from './utils/cli-error.js';
import { json } from './utils/output.js';
import { loadAnvilEnv } from './utils/env.js';

loadAnvilEnv();

declare const __CLI_VERSION__: string;
const CLI_VERSION = typeof __CLI_VERSION__ !== 'undefined' ? __CLI_VERSION__ : '0.0.0-dev';

// Commands that don't require authentication
const AUTH_EXEMPT_COMMANDS = new Set([
  'auth',
  'login',
  'logout',
  'whoami',
  'admin',
  'beta',
  'start',
  'help',
  'tutorial',
  'release',
]);

async function main(): Promise<void> {
  const args = process.argv.slice(2);
  const hasSubcommand = args.length > 0 && !args[0].startsWith('-');
  const isHelpOrVersion =
    args.includes('--help') ||
    args.includes('-h') ||
    args.includes('--version') ||
    args.includes('-V');

  if (!hasSubcommand && !isHelpOrVersion && isFirstRun()) {
    await showWelcome();
    return;
  }

  const program = new Command();

  program
    .name('anvil')
    .description('Anvil - Deterministic development automation platform')
    .version(CLI_VERSION);

  // Auth gate: check licence before every command (except exempt ones)
  program.hook('preAction', async (_thisCommand, actionCommand) => {
    let cmd: Command = actionCommand;
    while (cmd.parent && cmd.parent.parent) {
      cmd = cmd.parent;
    }
    const commandName = cmd.name();

    if (AUTH_EXEMPT_COMMANDS.has(commandName)) return;

    const isJson = actionCommand.opts().json;

    const jwt = loadLicence(process.cwd());
    if (!jwt) {
      // Backwards compat: auth.json exists but no licence
      const message = isAuthenticated()
        ? 'Your session needs to be refreshed. Run anvil login to continue.'
        : 'Authentication required. Run anvil login to authenticate.\n   New here? Try anvil tutorial first (no login required).';
      if (isJson) json({ error: message });
      throw new CliError(message, 1, { reported: isJson });
    }

    const result = await verifyLicence(jwt);

    if (!result.valid) {
      const message =
        result.reason === 'expired'
          ? 'Your licence needs to be renewed. Run anvil login to continue.'
          : 'Your licence could not be verified. Run anvil login or contact support@eddacraft.ai if this is unexpected.';
      if (isJson) json({ error: message, reason: result.reason });
      throw new CliError(message, 1, { reported: isJson });
    }

    // Background refresh if needed (non-blocking)
    if (result.needsRefresh) {
      const auth = loadAuth();
      if (auth) {
        scheduleRefresh(auth.token).catch(() => {
          // Swallow — refresh is best-effort
        });
      }
    }
  });

  // Auth commands
  program.addCommand(createAuthLoginCommand());
  program.addCommand(createLoginCommand());
  program.addCommand(createLogoutCommand());
  program.addCommand(createWhoamiCommand());
  program.addCommand(createAdminCommand());
  program.addCommand(createBetaCommand());

  // Feature commands
  program.addCommand(createAgentCommand());
  program.addCommand(createArchitectureCommand());
  program.addCommand(createAuthorshipCommand());
  program.addCommand(createCheckCommand());
  program.addCommand(createDoctorCommand());
  program.addCommand(createDriftCommand());
  program.addCommand(createEddaCommand());
  program.addCommand(createEmberCommand());
  program.addCommand(createExplainCommand());
  program.addCommand(createInitCommand());
  program.addCommand(createGateCommand());
  program.addCommand(createGateConfigCommand());
  program.addCommand(createNewCommand());
  program.addCommand(createPlanCommand());
  program.addCommand(createValidateCommand());
  program.addCommand(createExportCommand());
  program.addCommand(createHooksCommand());
  program.addCommand(createPolicyCommand());
  program.addCommand(createAuditCommand());
  program.addCommand(createStackCommand());
  program.addCommand(createWatchCommand());
  program.addCommand(createStartCommand());
  program.addCommand(createStatusCommand());
  program.addCommand(createTutorialCommand());
  program.addCommand(createMcpConfigCommand());
  program.addCommand(createReleaseCommand());

  await program.parseAsync(process.argv);
}

main().catch((error: unknown) => {
  if (error instanceof CliExit) {
    process.exit(0);
  }
  if (error instanceof CliError) {
    if (!error.reported) {
      console.error(`\x1b[31m✗\x1b[0m ${error.message}`);
    }
    process.exit(error.exitCode);
  }
  console.error('Unexpected error:', error instanceof Error ? error.message : String(error));
  process.exit(1);
});
