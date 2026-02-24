import { Command } from 'commander';
import chalk from 'chalk';
import { existsSync, mkdirSync, rmSync, writeFileSync, readFileSync } from 'node:fs';
import { CliError } from '../utils/cli-error.js';
import { join } from 'node:path';
import { isTUIAvailable } from '../tui/utils/tty-detection.js';
import { renderTUIAndWait } from '../tui/utils/renderer.js';
import { Tutorial } from '../tui/commands/tutorial/index.js';
import { theme } from '../tui/utils/theme.js';
import { getWorkspaceRoot } from '../utils/file-io.js';
import { TutorialProgressSchema } from '../tui/commands/tutorial/types.js';
import type { TutorialProgress } from '../tui/commands/tutorial/types.js';
import type { TutorialOption } from '../tui/commands/tutorial/components/TutorialPicker.js';

interface TutorialOptions {
  reset?: boolean;
  // Commander.js --no-tui sets options.tui = false (not options.noTui = true)
  tui?: boolean;
}

function printTutorialTTYError(options: TutorialOptions): never {
  if (options.tui === false) {
    console.error(
      chalk.hex(theme.colours.molten)(
        'Tutorial plain-text mode is not available yet. Remove --no-tui to use the interactive tutorial (which requires a TTY).'
      )
    );
  } else {
    console.error(chalk.hex(theme.colours.molten)('Tutorial requires an interactive terminal.'));
    console.error(chalk.hex(theme.colours.smoke)('Please run in a TTY environment.'));
  }

  throw new CliError('Tutorial requires an interactive terminal');
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

/**
 * Relative path to the policy file created by the policies tutorial.
 * Shared across the reset handler and the TUI onCleanup handler.
 */
const POLICY_TUTORIAL_FILE = '.anvil/policies/max_file_length.rego' as const;

function cleanupPolicyTutorialFile(workspaceRoot: string): boolean {
  const policyFile = join(workspaceRoot, POLICY_TUTORIAL_FILE);
  if (existsSync(policyFile)) {
    rmSync(policyFile);
    return true;
  }
  return false;
}

const AVAILABLE_TUTORIALS = [
  { topic: 'policies', description: 'Write custom OPA/Rego rules' },
  { topic: 'architecture', description: 'Define architecture boundaries' },
  { topic: 'drift', description: 'Track architecture drift over time' },
  { topic: 'ci', description: 'Set up CI integration' },
];

/** Tutorials list in the shape the TutorialPicker component expects. */
const TUTORIAL_OPTIONS: TutorialOption[] = [
  { topic: 'core', description: 'Core tutorial (scan, watch, fix)' },
  ...AVAILABLE_TUTORIALS,
];

/**
 * Render a single tutorial and return the topic the user selected next,
 * or null if they quit.
 */
async function renderTutorial(
  currentTopic: string | undefined,
  options: TutorialOptions
): Promise<string | null> {
  const useTUI = isTUIAvailable({ tui: options.tui });
  if (!useTUI) printTutorialTTYError(options);

  let nextTopic: string | null = null;
  const onSelectTutorial = (topic: string) => {
    nextTopic = topic;
  };

  if (!currentTopic || currentTopic === 'core') {
    // Core tutorial
    const workspaceRoot = getWorkspaceRoot();
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
      onSelectTutorial,
      tutorials: TUTORIAL_OPTIONS,
    });

    if (cleanedUp) {
      console.log(
        chalk.hex(theme.colours.steel)(`\n${theme.icons.success} Tutorial files cleaned up`)
      );
    }

    return nextTopic;
  }

  if (currentTopic === 'policies') {
    const { PolicyTutorial } = await import('../tui/commands/tutorial/features/PolicyTutorial.js');

    let policyCleanedUp = false;

    await renderTUIAndWait(PolicyTutorial, {
      onComplete: () => {},
      onCleanup: () => {
        const workspaceRoot = getWorkspaceRoot();
        cleanupPolicyTutorialFile(workspaceRoot);
        policyCleanedUp = true;
      },
      onSelectTutorial,
      tutorials: TUTORIAL_OPTIONS,
    });

    if (policyCleanedUp) {
      console.log(
        chalk.hex(theme.colours.steel)(`\n${theme.icons.success} Tutorial policy file cleaned up`)
      );
    }

    return nextTopic;
  }

  if (currentTopic === 'architecture') {
    const { ArchitectureTutorial } =
      await import('../tui/commands/tutorial/features/ArchitectureTutorial.js');

    await renderTUIAndWait(ArchitectureTutorial, {
      onComplete: () => {},
      onCleanup: () => {},
      onSelectTutorial,
      tutorials: TUTORIAL_OPTIONS,
    });

    return nextTopic;
  }

  if (currentTopic === 'drift') {
    const { DriftTutorial } = await import('../tui/commands/tutorial/features/DriftTutorial.js');

    await renderTUIAndWait(DriftTutorial, {
      onComplete: () => {},
      onCleanup: () => {},
      onSelectTutorial,
      tutorials: TUTORIAL_OPTIONS,
    });

    return nextTopic;
  }

  if (currentTopic === 'ci') {
    const { CITutorial } = await import('../tui/commands/tutorial/features/CITutorial.js');

    await renderTUIAndWait(CITutorial, {
      onComplete: () => {},
      onCleanup: () => {},
      onSelectTutorial,
      tutorials: TUTORIAL_OPTIONS,
    });

    return nextTopic;
  }

  // Unknown topic
  return null;
}

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

      if (topic && options.reset) {
        const validTopics = AVAILABLE_TUTORIALS.map((t) => t.topic);
        if (!validTopics.includes(topic)) {
          console.log(
            chalk.hex(theme.colours.slag)(
              `\nUnknown tutorial topic '${topic}'. Run ${chalk.hex(theme.colours.text)('anvil tutorial --list')} to see available tutorials.\n`
            )
          );
          return;
        }

        // Topic-specific reset: clean up artifacts created by that tutorial
        if (topic === 'policies') {
          const workspaceRoot = getWorkspaceRoot();
          if (cleanupPolicyTutorialFile(workspaceRoot)) {
            console.log(
              chalk.hex(theme.colours.steel)(`${theme.icons.success} Removed tutorial policy file`)
            );
          }
        }

        // For architecture, drift, ci — no persistent files are created
        console.log(
          chalk.hex(theme.colours.steel)(`${theme.icons.success} Tutorial '${topic}' reset`)
        );
        console.log(
          chalk.hex(theme.colours.smoke)(
            `Run ${chalk.hex(theme.colours.text)(`anvil tutorial ${topic}`)} to start fresh.`
          )
        );
        return;
      }

      if (!topic) {
        // No topic: check for --reset on core tutorial
        const workspaceRoot = getWorkspaceRoot();

        if (options.reset) {
          cleanupTutorialFiles(workspaceRoot);
          console.log(
            chalk.hex(theme.colours.steel)(`${theme.icons.success} Tutorial progress reset`)
          );
          console.log(chalk.hex(theme.colours.smoke)('Run anvil tutorial to start fresh.'));
          return;
        }
      }

      // Validate topic if provided
      if (topic) {
        const validTopics = AVAILABLE_TUTORIALS.map((t) => t.topic);
        if (!validTopics.includes(topic)) {
          console.log(
            chalk.hex(theme.colours.slag)(
              `\nUnknown tutorial topic '${topic}'. Run ${chalk.hex(theme.colours.text)('anvil tutorial --list')} to see available tutorials.\n`
            )
          );
          return;
        }
      }

      // Tutorial loop: render tutorials until the user quits without selecting another
      let currentTopic = topic ?? undefined;
      while (true) {
        const next = await renderTutorial(currentTopic, options);
        if (!next) break;
        currentTopic = next;
      }
    });

  return command;
}
