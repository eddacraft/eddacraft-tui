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
import { blank, print } from '../utils/output.js';
import { TutorialProgressSchema } from '../tui/commands/tutorial/types.js';
import type { TutorialProgress } from '../tui/commands/tutorial/types.js';
import type { TutorialOption } from '../tui/commands/tutorial/components/TutorialPicker.js';

interface TutorialOptions {
  reset?: boolean;
  tui?: boolean;
}

function printTutorialTTYError(): never {
  print(chalk.hex(theme.colours.molten)('Tutorial requires an interactive terminal.'));
  print(chalk.hex(theme.colours.smoke)('Please run in a TTY environment.'));

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
      print(
        chalk.hex(theme.colours.molten)(
          `${theme.icons.warning} Invalid tutorial progress format, starting fresh`
        )
      );
      return null;
    }

    return result.data;
  } catch (err) {
    print(
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

interface RenderResult {
  nextTopic: string | null;
  cleanedUp: boolean;
  completed: boolean;
}

/**
 * Render a single tutorial and return the topic the user selected next,
 * or null if they quit, plus whether cleanup was performed.
 */
async function renderTutorial(
  currentTopic: string | undefined,
  options: TutorialOptions,
  completedTopics: string[]
): Promise<RenderResult> {
  const useTUI = isTUIAvailable({ tui: options.tui });
  if (!useTUI) printTutorialTTYError();

  let nextTopic: string | null = null;
  const onSelectTutorial = (topic: string) => {
    nextTopic = topic;
  };

  if (!currentTopic || currentTopic === 'core') {
    // Core tutorial
    const workspaceRoot = getWorkspaceRoot();
    ensureTutorialDir(workspaceRoot);

    let cleanedUp = false;
    let completed = false;

    const handleComplete = () => {
      completed = true;
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
      completedTopics,
    });

    if (cleanedUp) {
      print(chalk.hex(theme.colours.steel)(`\n${theme.icons.success} Tutorial files cleaned up`));
    }

    return { nextTopic, cleanedUp, completed };
  }

  if (currentTopic === 'policies') {
    const { PolicyTutorial } = await import('../tui/commands/tutorial/features/PolicyTutorial.js');

    let policyCleanedUp = false;
    let completed = false;

    await renderTUIAndWait(PolicyTutorial, {
      onComplete: () => {
        completed = true;
      },
      onCleanup: () => {
        const workspaceRoot = getWorkspaceRoot();
        cleanupPolicyTutorialFile(workspaceRoot);
        policyCleanedUp = true;
      },
      onSelectTutorial,
      tutorials: TUTORIAL_OPTIONS,
      completedTopics,
    });

    if (policyCleanedUp) {
      print(
        chalk.hex(theme.colours.steel)(`\n${theme.icons.success} Tutorial policy file cleaned up`)
      );
    }

    return { nextTopic, cleanedUp: policyCleanedUp, completed };
  }

  if (currentTopic === 'architecture') {
    const { ArchitectureTutorial } =
      await import('../tui/commands/tutorial/features/ArchitectureTutorial.js');

    let completed = false;

    await renderTUIAndWait(ArchitectureTutorial, {
      onComplete: () => {
        completed = true;
      },
      onSelectTutorial,
      tutorials: TUTORIAL_OPTIONS,
      completedTopics,
    });

    return { nextTopic, cleanedUp: false, completed };
  }

  if (currentTopic === 'drift') {
    const { DriftTutorial } = await import('../tui/commands/tutorial/features/DriftTutorial.js');

    let completed = false;

    await renderTUIAndWait(DriftTutorial, {
      onComplete: () => {
        completed = true;
      },
      onSelectTutorial,
      tutorials: TUTORIAL_OPTIONS,
      completedTopics,
    });

    return { nextTopic, cleanedUp: false, completed };
  }

  if (currentTopic === 'ci') {
    const { CITutorial } = await import('../tui/commands/tutorial/features/CITutorial.js');

    let completed = false;

    await renderTUIAndWait(CITutorial, {
      onComplete: () => {
        completed = true;
      },
      onSelectTutorial,
      tutorials: TUTORIAL_OPTIONS,
      completedTopics,
    });

    return { nextTopic, cleanedUp: false, completed };
  }

  // Unknown topic
  return { nextTopic: null, cleanedUp: false, completed: false };
}

export function createTutorialCommand(): Command {
  const command = new Command('tutorial');

  command
    .description('Interactive tutorial to learn Anvil basics (< 5 minutes)')
    .argument('[topic]', 'Tutorial topic (core, policies, architecture, drift, ci)')
    .option('--list', 'Show available tutorials')
    .option('--reset', 'Clear previous progress and start fresh')
    .option('--tui', 'Force TUI mode (default: auto-detect)')
    .action(async (topic: string | undefined, options: TutorialOptions & { list?: boolean }) => {
      if (options.list) {
        print(chalk.hex(theme.colours.ember)('\nAvailable tutorials:\n'));
        print(
          chalk.hex(theme.colours.text)('  anvil tutorial') +
            chalk.hex(theme.colours.smoke)('              Core tutorial (scan, watch, fix)')
        );
        for (const t of AVAILABLE_TUTORIALS) {
          print(
            chalk.hex(theme.colours.text)(`  anvil tutorial ${t.topic}`) +
              chalk.hex(theme.colours.smoke)(`${' '.repeat(14 - t.topic.length)}${t.description}`)
          );
        }
        blank();
        return;
      }

      if (topic && options.reset) {
        const validTopics = TUTORIAL_OPTIONS.map((t) => t.topic);
        if (!validTopics.includes(topic)) {
          print(
            chalk.hex(theme.colours.slag)(
              `\nUnknown tutorial topic '${topic}'. Run ${chalk.hex(theme.colours.text)('anvil tutorial --list')} to see available tutorials.\n`
            )
          );
          return;
        }

        // Topic-specific reset: clean up artifacts created by that tutorial
        const workspaceRoot = getWorkspaceRoot();

        if (topic === 'core') {
          // Preserve non-core completedTutorials before wiping core files
          const existing = loadProgress(workspaceRoot);
          const nonCoreCompleted = (existing?.completedTutorials ?? []).filter((t) => t !== 'core');

          cleanupTutorialFiles(workspaceRoot);

          // Restore non-core completion tracking
          if (nonCoreCompleted.length > 0) {
            saveProgress(workspaceRoot, {
              currentStep: 0,
              totalSteps: 0,
              completedSteps: [],
              startedAt: new Date().toISOString(),
              completedTutorials: nonCoreCompleted,
            });
          }

          print(chalk.hex(theme.colours.steel)(`${theme.icons.success} Tutorial progress reset`));
          print(chalk.hex(theme.colours.smoke)('Run anvil tutorial to start fresh.'));
          return;
        }

        if (topic === 'policies') {
          if (cleanupPolicyTutorialFile(workspaceRoot)) {
            print(
              chalk.hex(theme.colours.steel)(`${theme.icons.success} Removed tutorial policy file`)
            );
          }
        }

        // Remove topic from completedTutorials so it can be re-selected
        const existing = loadProgress(workspaceRoot);
        if (existing?.completedTutorials?.includes(topic)) {
          saveProgress(workspaceRoot, {
            ...existing,
            completedTutorials: existing.completedTutorials.filter((t) => t !== topic),
          });
        }

        // For architecture, drift, ci — no persistent files are created
        print(chalk.hex(theme.colours.steel)(`${theme.icons.success} Tutorial '${topic}' reset`));
        print(
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
          print(chalk.hex(theme.colours.steel)(`${theme.icons.success} Tutorial progress reset`));
          print(chalk.hex(theme.colours.smoke)('Run anvil tutorial to start fresh.'));
          return;
        }
      }

      // Validate topic if provided
      if (topic) {
        const validTopics = TUTORIAL_OPTIONS.map((t) => t.topic);
        if (!validTopics.includes(topic)) {
          print(
            chalk.hex(theme.colours.slag)(
              `\nUnknown tutorial topic '${topic}'. Run ${chalk.hex(theme.colours.text)('anvil tutorial --list')} to see available tutorials.\n`
            )
          );
          return;
        }
      }

      // Tutorial loop: render tutorials until the user quits without selecting another
      const workspaceRoot = getWorkspaceRoot();
      const progress = loadProgress(workspaceRoot);
      const completedTopics: string[] = progress?.completedTutorials ?? [];

      let currentTopic = topic ?? undefined;
      while (true) {
        const result = await renderTutorial(currentTopic, options, completedTopics);
        // Only persist progress if the tutorial was actually completed (not quit early)
        // and cleanup wasn't performed
        if (!result.cleanedUp && (result.completed || result.nextTopic)) {
          const justCompleted = currentTopic ?? 'core';
          if (!completedTopics.includes(justCompleted)) {
            completedTopics.push(justCompleted);
          }
          const existing = loadProgress(workspaceRoot);
          saveProgress(workspaceRoot, {
            ...(existing ?? {
              currentStep: 0,
              totalSteps: 0,
              completedSteps: [],
              startedAt: new Date().toISOString(),
            }),
            completedTutorials: completedTopics,
          });
        }
        if (!result.nextTopic) break;
        currentTopic = result.nextTopic;
      }
    });

  return command;
}
