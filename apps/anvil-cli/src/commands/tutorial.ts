import { Command } from 'commander';
import chalk from 'chalk';
import { existsSync, mkdirSync, rmSync, writeFileSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { isTUIAvailable } from '../tui/utils/tty-detection.js';
import { renderTUIAndWait } from '../tui/utils/renderer.js';
import { Tutorial } from '../tui/commands/tutorial/index.js';
import { theme } from '../tui/utils/theme.js';
import { getWorkspaceRoot } from '../utils/file-io.js';
import { TutorialProgressSchema } from '../tui/commands/tutorial/types.js';
import type { TutorialProgress } from '../tui/commands/tutorial/types.js';

interface TutorialOptions {
  reset?: boolean;
  // Commander.js --no-tui sets options.tui = false (not options.noTui = true)
  tui?: boolean;
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
    const parsed = JSON.parse(content);
    const result = TutorialProgressSchema.safeParse(parsed);

    if (!result.success) {
      console.warn(
        chalk.hex(theme.colours.molten)(
          `${theme.icons.warning} Invalid tutorial progress format, starting fresh`
        )
      );
      return null;
    }

    return result.data;
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

const AVAILABLE_TUTORIALS = [
  { topic: 'policies', description: 'Write custom OPA/Rego rules' },
  { topic: 'architecture', description: 'Define architecture boundaries' },
  { topic: 'drift', description: 'Track architecture drift over time' },
  { topic: 'ci', description: 'Set up CI integration' },
];

export function createTutorialCommand(): Command {
  const command = new Command('tutorial');

  command
    .description('Interactive tutorial to learn Anvil basics (< 5 minutes)')
    .argument('[topic]', 'Tutorial topic (policies, architecture, drift, ci)')
    .option('--list', 'Show available tutorials')
    .option('--reset', 'Clear previous progress and start fresh')
    .option('--tui', 'Force TUI mode')
    .option('--no-tui', 'Force plain text mode (not recommended)')
    .action(async (topic: string | undefined, options: TutorialOptions & { list?: boolean }) => {
      if (options.list) {
        console.log(chalk.hex(theme.colours.ember)('\nAvailable tutorials:\n'));
        console.log(
          chalk.hex(theme.colours.text)('  anvil tutorial') +
            chalk.hex(theme.colours.smoke)('              Core tutorial (scan, watch, fix)')
        );
        for (const t of AVAILABLE_TUTORIALS) {
          console.log(
            chalk.hex(theme.colours.text)(`  anvil tutorial ${t.topic}`) +
              chalk.hex(theme.colours.smoke)(`${' '.repeat(14 - t.topic.length)}${t.description}`)
          );
        }
        console.log();
        return;
      }

      if (topic) {
        if (topic === 'policies') {
          const useTUI = isTUIAvailable({ tui: options.tui });

          if (!useTUI) {
            console.log(
              chalk.hex(theme.colours.molten)('Tutorial requires an interactive terminal.')
            );
            console.log(chalk.hex(theme.colours.smoke)('Please run in a TTY environment.'));
            process.exit(1);
          }

          const { PolicyTutorial } =
            await import('../tui/commands/tutorial/features/PolicyTutorial.js');

          let policyCleanedUp = false;

          await renderTUIAndWait(PolicyTutorial, {
            onComplete: () => {},
            onCleanup: () => {
              // Clean up the policy file created by the tutorial
              const workspaceRoot = getWorkspaceRoot();
              const policyFile = join(workspaceRoot, '.anvil', 'policies', 'max_file_length.rego');

              if (existsSync(policyFile)) {
                rmSync(policyFile);
              }
              policyCleanedUp = true;
            },
          });

          if (policyCleanedUp) {
            console.log(
              chalk.hex(theme.colours.steel)(
                `\n${theme.icons.success} Tutorial policy file cleaned up`
              )
            );
          }
          return;
        }

        if (topic === 'architecture') {
          const useTUI = isTUIAvailable({ tui: options.tui });

          if (!useTUI) {
            console.log(
              chalk.hex(theme.colours.molten)('Tutorial requires an interactive terminal.')
            );
            console.log(chalk.hex(theme.colours.smoke)('Please run in a TTY environment.'));
            process.exit(1);
          }

          const { ArchitectureTutorial } =
            await import('../tui/commands/tutorial/features/ArchitectureTutorial.js');

          await renderTUIAndWait(ArchitectureTutorial, {
            onComplete: () => {},
            onCleanup: () => {},
          });

          return;
        }

        if (topic === 'drift') {
          const useTUI = isTUIAvailable({ tui: options.tui });

          if (!useTUI) {
            console.log(
              chalk.hex(theme.colours.molten)('Tutorial requires an interactive terminal.')
            );
            console.log(chalk.hex(theme.colours.smoke)('Please run in a TTY environment.'));
            process.exit(1);
          }

          const { DriftTutorial } =
            await import('../tui/commands/tutorial/features/DriftTutorial.js');

          await renderTUIAndWait(DriftTutorial, {
            onComplete: () => {},
            onCleanup: () => {},
          });

          return;
        }

        const known = AVAILABLE_TUTORIALS.find((t) => t.topic === topic);
        if (known) {
          console.log(
            chalk.hex(theme.colours.molten)(
              `\nTutorial '${topic}' coming soon. Run ${chalk.hex(theme.colours.text)('anvil tutorial')} for the core tutorial.\n`
            )
          );
        } else {
          console.log(
            chalk.hex(theme.colours.slag)(
              `\nUnknown tutorial topic '${topic}'. Run ${chalk.hex(theme.colours.text)('anvil tutorial --list')} to see available tutorials.\n`
            )
          );
        }
        return;
      }
      const workspaceRoot = getWorkspaceRoot();

      if (options.reset) {
        cleanupTutorialFiles(workspaceRoot);
        console.log(
          chalk.hex(theme.colours.steel)(`${theme.icons.success} Tutorial progress reset`)
        );
        console.log(chalk.hex(theme.colours.smoke)('Run anvil tutorial to start fresh.'));
        return;
      }

      const useTUI = isTUIAvailable({ tui: options.tui });

      if (!useTUI) {
        console.log(chalk.hex(theme.colours.molten)('Tutorial requires an interactive terminal.'));
        console.log(chalk.hex(theme.colours.smoke)('Please run in a TTY environment.'));
        process.exit(1);
      }

      ensureTutorialDir(workspaceRoot);

      let cleanedUp = false;

      const handleComplete = () => {
        const progress: TutorialProgress = {
          currentStep: 4,
          totalSteps: 4,
          completedSteps: ['scan', 'watch', 'fix', 'next-steps'],
          startedAt: new Date().toISOString(),
        };
        saveProgress(workspaceRoot, progress);
      };

      const handleCleanup = () => {
        cleanupTutorialFiles(workspaceRoot);
        cleanedUp = true;
      };

      await renderTUIAndWait(Tutorial, {
        onComplete: handleComplete,
        onCleanup: handleCleanup,
      });

      // Print cleanup message after TUI exits to avoid rendering issues
      if (cleanedUp) {
        console.log(
          chalk.hex(theme.colours.steel)(`\n${theme.icons.success} Tutorial files cleaned up`)
        );
      }
    });

  return command;
}
