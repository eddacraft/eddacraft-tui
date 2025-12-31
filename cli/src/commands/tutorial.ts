import { Command } from 'commander';
import chalk from 'chalk';
import { existsSync, mkdirSync, rmSync, writeFileSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { isTUIAvailable } from '../tui/utils/tty-detection.js';
import { renderTUIAndWait } from '../tui/utils/renderer.js';
import { Tutorial } from '../tui/commands/tutorial/index.js';
import { theme } from '../tui/utils/theme.js';
import { getWorkspaceRoot } from '../utils/file-io.js';
import type { TutorialProgress } from '../tui/commands/tutorial/types.js';

interface TutorialOptions {
  reset?: boolean;
  tui?: boolean;
  noTui?: boolean;
}

function getProgressFilePath(workspaceRoot: string): string {
  return join(workspaceRoot, '.anvil', 'tutorial-progress.json');
}

function getTutorialDir(workspaceRoot: string): string {
  return join(workspaceRoot, '.anvil', 'tutorial');
}

function loadProgress(workspaceRoot: string): TutorialProgress | null {
  const progressPath = getProgressFilePath(workspaceRoot);
  if (!existsSync(progressPath)) return null;

  try {
    const content = readFileSync(progressPath, 'utf-8');
    return JSON.parse(content) as TutorialProgress;
  } catch (err) {
    console.warn(
      chalk.hex(theme.colours.molten)(
        `${theme.icons.warning} Could not parse tutorial progress: ${err instanceof Error ? err.message : 'invalid JSON'}`
      )
    );
    return null;
  }
}

export { loadProgress };

function saveProgress(workspaceRoot: string, progress: TutorialProgress): void {
  const progressPath = getProgressFilePath(workspaceRoot);
  const dir = join(workspaceRoot, '.anvil');

  if (!existsSync(dir)) {
    mkdirSync(dir, { recursive: true });
  }

  writeFileSync(progressPath, JSON.stringify(progress, null, 2));
}

function cleanupTutorialFiles(workspaceRoot: string): void {
  const tutorialDir = getTutorialDir(workspaceRoot);
  const progressPath = getProgressFilePath(workspaceRoot);

  if (existsSync(tutorialDir)) {
    rmSync(tutorialDir, { recursive: true, force: true });
  }

  if (existsSync(progressPath)) {
    rmSync(progressPath);
  }
}

function ensureTutorialDir(workspaceRoot: string): string {
  const tutorialDir = getTutorialDir(workspaceRoot);
  if (!existsSync(tutorialDir)) {
    mkdirSync(tutorialDir, { recursive: true });
  }
  return tutorialDir;
}

export function createTutorialCommand(): Command {
  const command = new Command('tutorial');

  command
    .description('Interactive tutorial to learn Anvil basics (< 5 minutes)')
    .option('--reset', 'Clear previous progress and start fresh')
    .option('--tui', 'Force TUI mode')
    .option('--no-tui', 'Force plain text mode (not recommended)')
    .action(async (options: TutorialOptions) => {
      const workspaceRoot = getWorkspaceRoot();

      if (options.reset) {
        cleanupTutorialFiles(workspaceRoot);
        console.log(
          chalk.hex(theme.colours.steel)(`${theme.icons.success} Tutorial progress reset`)
        );
        console.log(chalk.hex(theme.colours.smoke)('Run anvil tutorial to start fresh.'));
        return;
      }

      const useTUI = isTUIAvailable({ tui: options.tui, noTui: options.noTui });

      if (!useTUI) {
        console.log(chalk.hex(theme.colours.molten)('Tutorial requires an interactive terminal.'));
        console.log(chalk.hex(theme.colours.smoke)('Please run in a TTY environment.'));
        process.exit(1);
      }

      ensureTutorialDir(workspaceRoot);

      const handleComplete = () => {
        const progress: TutorialProgress = {
          currentStep: 5,
          totalSteps: 5,
          completedSteps: ['intro', 'plan', 'validate', 'gate', 'completion'],
          startedAt: new Date().toISOString(),
        };
        saveProgress(workspaceRoot, progress);
      };

      const handleCleanup = () => {
        cleanupTutorialFiles(workspaceRoot);
        console.log(
          chalk.hex(theme.colours.steel)(`\n${theme.icons.success} Tutorial files cleaned up`)
        );
      };

      await renderTUIAndWait(Tutorial, {
        onComplete: handleComplete,
        onCleanup: handleCleanup,
      });
    });

  return command;
}
