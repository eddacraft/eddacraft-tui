import { Command } from 'commander';
import { createAgentCommand } from './commands/agent/index.js';
import { createArchitectureCommand } from './commands/architecture.js';
import { createAuthorshipCommand } from './commands/authorship.js';
import { createBetaCommand } from './commands/beta.js';
import { createCheckCommand } from './commands/check.js';
import { createDoctorCommand } from './commands/doctor.js';
import { createDriftCommand } from './commands/drift.js';
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
import { createLoginCommand } from './commands/login.js';
import { createLogoutCommand } from './commands/logout.js';
import { createWhoamiCommand } from './commands/whoami.js';
import { isFirstRun } from './services/first-run-detector.js';
import { isAuthenticated } from './services/auth-store.js';
import { showWelcome, createStartCommand } from './commands/welcome.js';
import { loadAnvilEnv } from './utils/env.js';

loadAnvilEnv();

declare const __CLI_VERSION__: string;
const CLI_VERSION = typeof __CLI_VERSION__ !== 'undefined' ? __CLI_VERSION__ : '0.0.0-dev';

// Commands that don't require authentication
const AUTH_EXEMPT_COMMANDS = new Set([
  'login',
  'logout',
  'whoami',
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

  // Auth gate: check authentication before every command (except exempt ones)
  program.hook('preAction', (_thisCommand, actionCommand) => {
    // Walk up the command chain to find the top-level command name
    let cmd: Command = actionCommand;
    while (cmd.parent && cmd.parent.parent) {
      cmd = cmd.parent;
    }
    const commandName = cmd.name();

    if (AUTH_EXEMPT_COMMANDS.has(commandName)) return;
    if (isAuthenticated()) return;

    console.error(
      '\x1b[31m✗\x1b[0m Authentication required. Run \x1b[1manvil login\x1b[0m to authenticate.\n' +
        '   New here? Try \x1b[1manvil tutorial\x1b[0m first (no login required).'
    );
    process.exit(1);
  });

  // Auth commands
  program.addCommand(createLoginCommand());
  program.addCommand(createLogoutCommand());
  program.addCommand(createWhoamiCommand());
  program.addCommand(createBetaCommand());

  // Feature commands
  program.addCommand(createAgentCommand());
  program.addCommand(createArchitectureCommand());
  program.addCommand(createAuthorshipCommand());
  program.addCommand(createCheckCommand());
  program.addCommand(createDoctorCommand());
  program.addCommand(createDriftCommand());
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
  console.error('Error:', error instanceof Error ? error.message : String(error));
  process.exit(1);
});
