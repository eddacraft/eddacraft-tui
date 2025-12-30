import { spawn } from 'child_process';
import chalk from 'chalk';
import { isTUIAvailable } from '../tui/utils/tty-detection.js';
import { renderTUI } from '../tui/utils/renderer.js';
import { Welcome } from '../tui/commands/welcome/Welcome.js';
import { markFirstRunComplete } from '../services/first-run-detector.js';
import type { QuickStartOption } from '../tui/commands/welcome/content.js';
import {
  ANVIL_LOGO,
  VALUE_PROPOSITION,
  QUICK_START_OPTIONS,
} from '../tui/commands/welcome/content.js';

export async function showWelcome(): Promise<void> {
  const useTUI = isTUIAvailable();

  if (useTUI) {
    await showWelcomeTUI();
  } else {
    showWelcomePlain();
  }
}

async function showWelcomeTUI(): Promise<void> {
  return new Promise<void>((resolve) => {
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
      result.waitUntilExit().then(() => {
        markFirstRunComplete();

        if (selectedOption?.command) {
          runCommand(selectedOption.command);
        }

        resolve();
      });
    } else {
      showWelcomePlain();
      resolve();
    }
  });
}

function showWelcomePlain(): void {
  console.log(chalk.cyan.bold(ANVIL_LOGO));
  console.log(VALUE_PROPOSITION);
  console.log('');
  console.log(chalk.bold('Quick Start:'));
  console.log('');

  for (const option of QUICK_START_OPTIONS) {
    if (option.command) {
      console.log(`  ${chalk.cyan(option.command.padEnd(20))} ${option.description}`);
    }
  }

  console.log('');
  console.log(chalk.dim('This welcome message appears once. Set ANVIL_SKIP_WELCOME=1 to disable.'));
  console.log('');

  markFirstRunComplete();
}

function runCommand(command: string): void {
  const [cmd, ...args] = command.split(' ');
  spawn(cmd, args, {
    stdio: 'inherit',
    shell: true,
  });
}
