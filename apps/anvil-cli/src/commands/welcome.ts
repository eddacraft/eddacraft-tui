import { spawn } from 'node:child_process';
import chalk from 'chalk';
import { Command } from 'commander';
import { isTUIAvailable } from '../tui/utils/tty-detection.js';
import { renderTUI } from '../tui/utils/renderer.js';
import { Welcome } from '../tui/commands/welcome/Welcome.js';
import { markFirstRunComplete } from '../services/first-run-detector.js';
import { blank, print } from '../utils/output.js';
import type { QuickStartOption } from '../tui/commands/welcome/content.js';
import {
  ANVIL_LOGO,
  VALUE_PROPOSITION,
  QUICK_START_OPTIONS,
} from '../tui/commands/welcome/content.js';

export function createStartCommand(): Command {
  const command = new Command('start');

  command.description('Show getting started options (login, tutorial, help)').action(async () => {
    await showWelcome();
  });

  return command;
}

export async function showWelcome(): Promise<void> {
  const useTUI = isTUIAvailable();

  if (useTUI) {
    await showWelcomeTUI();
  } else {
    showWelcomePlain();
  }
}

async function showWelcomeTUI(): Promise<void> {
  return new Promise<void>((resolve, reject) => {
    let selectedOption: QuickStartOption | null = null;

    const result = renderTUI(Welcome, {
      onSelect: (option: QuickStartOption) => {
        selectedOption = option;
      },
      onQuit: () => {
        markFirstRunComplete();
        resolve();
      },
    });

    if (result) {
      result
        .waitUntilExit()
        .then(async () => {
          markFirstRunComplete();

          if (selectedOption?.command) {
            await runCommand(selectedOption.command);
          }

          resolve();
        })
        .catch(reject);
    } else {
      showWelcomePlain();
      resolve();
    }
  });
}

function showWelcomePlain(): void {
  print(chalk.cyan.bold(ANVIL_LOGO));
  print(VALUE_PROPOSITION);
  blank();
  print(chalk.bold('Quick Start:'));
  blank();

  for (const option of QUICK_START_OPTIONS) {
    if (option.command) {
      print(`  ${chalk.cyan(option.command.padEnd(20))} ${option.description}`);
    }
  }

  blank();
  print(chalk.dim('This welcome message appears once. Set ANVIL_SKIP_WELCOME=1 to disable.'));
  blank();

  markFirstRunComplete();
}

function runCommand(command: string): Promise<void> {
  return new Promise<void>((resolve, reject) => {
    const [cmd, ...args] = command.split(' ');
    // Windows requires shell: true to execute .cmd batch files created by npm.
    // Linux/macOS don't need it and it triggers DEP0190 deprecation in Node.js v24+.
    const child = spawn(cmd, args, {
      stdio: 'inherit',
      shell: process.platform === 'win32',
    });

    child.on('close', (code, signal) => {
      if (code === 0) {
        resolve();
      } else {
        const reason =
          code === null
            ? `terminated by signal ${signal ?? 'unknown'}`
            : `exited with code ${code}`;
        reject(new Error(`Command "${command}" ${reason}`));
      }
    });

    child.on('error', (err) => {
      reject(new Error(`Failed to run "${command}": ${err.message}`, { cause: err }));
    });
  });
}
