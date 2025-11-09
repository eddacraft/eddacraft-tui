import { Command } from 'commander';
import inquirer from 'inquirer';
import chalk from 'chalk';
import ora from 'ora';
import { existsSync } from 'fs';
import { join } from 'path';
import { EnvironmentDetector } from '../services/environment-detector.js';
import {
  TemplateGenerator,
  type PlanningFormat,
  type ConfigTemplate,
  type InitOptions,
} from '../services/template-generator.js';
import { success, error } from '../utils/output.js';

export function createInitCommand(): Command {
  const command = new Command('init');

  command
    .description('Initialise Anvil in the current project')
    .option('--force', 'Overwrite existing .anvilrc if present')
    .option('--non-interactive', 'Skip interactive prompts and use defaults')
    .action(async (options: { force?: boolean; nonInteractive?: boolean }) => {
      try {
        console.log(chalk.bold('\n🔨 Initialising Anvil in current project...\n'));

        const projectRoot = process.cwd();

        // Check if .anvilrc already exists
        const anvilrcPath = join(projectRoot, '.anvilrc');
        if (existsSync(anvilrcPath) && !options.force) {
          error('.anvilrc already exists. Use --force to overwrite.');
          console.log(
            chalk.dim('\nTip: Run `anvil gate:config --list` to view current configuration')
          );
          process.exit(1);
        }

        // Detect environment
        const detector = new EnvironmentDetector(projectRoot);
        const env = detector.detect();

        console.log(chalk.cyan('Detected environment:'));
        console.log(chalk.dim(`  Project: ${env.projectName || '(no package.json)'}`));
        console.log(chalk.dim(`  Package Manager: ${env.packageManager}`));
        console.log(chalk.dim(`  Git: ${env.hasGit ? '✓' : '✗'}`));
        console.log(chalk.dim(`  TypeScript: ${env.hasTypeScript ? '✓' : '✗'}`));
        console.log(chalk.dim(`  ESLint: ${env.hasEslint ? '✓' : '✗'}`));
        console.log(
          chalk.dim(`  Testing: ${env.hasVitest ? 'Vitest' : env.hasJest ? 'Jest' : '✗'}`)
        );
        console.log('');

        let initOptions: InitOptions;

        if (options.nonInteractive) {
          // Use defaults
          initOptions = {
            projectRoot,
            planningDir: 'docs/plans',
            format: 'generic',
            createExample: true,
            configTemplate: 'basic',
            enabledChecks: detector.getRecommendedChecks(env),
            coverageThreshold: 80,
          };
        } else {
          // Interactive prompts
          initOptions = await runInteractiveSetup(env, detector);
        }

        // Generate configuration and files
        const spinner = ora('Setting up Anvil...').start();

        try {
          const generator = new TemplateGenerator(initOptions);

          // Create .anvil directory
          generator.createAnvilDirectory();
          spinner.text = 'Created .anvil/ directory';

          // Create planning directory
          generator.createPlanningDirectory();
          spinner.text = `Created ${initOptions.planningDir}/ directory`;

          // Generate .anvilrc
          generator.generateAnvilrc();
          spinner.text = 'Generated .anvilrc configuration';

          // Update .gitignore
          if (env.hasGit) {
            generator.updateGitignore();
            spinner.text = 'Updated .gitignore';
          }

          // Generate example plans
          const exampleFiles = generator.generateExamplePlan(env);

          spinner.succeed(chalk.green('Anvil initialised successfully!'));

          // Show summary
          console.log('\n' + chalk.bold('Created files:'));
          console.log(chalk.dim('  ✓ .anvilrc'));
          console.log(chalk.dim('  ✓ .anvil/'));
          console.log(chalk.dim(`  ✓ ${initOptions.planningDir}/`));
          if (env.hasGit) {
            console.log(chalk.dim('  ✓ .gitignore (updated)'));
          }
          if (exampleFiles.length > 0) {
            console.log(chalk.dim('\n' + chalk.bold('Example files:')));
            exampleFiles.forEach((file) => {
              const relPath = file.replace(projectRoot + '/', '');
              console.log(chalk.dim(`  ✓ ${relPath}`));
            });
          }

          // Show next steps
          console.log('\n' + chalk.bold('Next steps:'));
          console.log(chalk.cyan('  1. Review configuration:'));
          console.log(chalk.dim('     anvil gate:config --list'));

          if (exampleFiles.length > 0) {
            const firstExample = exampleFiles[0].replace(projectRoot + '/', '');
            console.log(chalk.cyan('  2. Validate example plan:'));
            console.log(chalk.dim(`     anvil validate ${firstExample}`));
            console.log(chalk.cyan('  3. Run quality gates:'));
            console.log(chalk.dim(`     anvil gate ${firstExample}`));
          } else {
            console.log(chalk.cyan('  2. Create a planning document in:'));
            console.log(chalk.dim(`     ${initOptions.planningDir}/`));
            console.log(chalk.cyan('  3. Validate your plan:'));
            console.log(chalk.dim('     anvil validate <plan-file>'));
          }

          console.log('');
          success('Anvil is ready to use!');
        } catch (err) {
          spinner.fail('Initialisation failed');
          throw err;
        }
      } catch (err) {
        error(`Initialisation failed: ${err instanceof Error ? err.message : 'Unknown error'}`);
        process.exit(1);
      }
    });

  return command;
}

async function runInteractiveSetup(
  env: ReturnType<EnvironmentDetector['detect']>,
  detector: EnvironmentDetector
): Promise<InitOptions> {
  const answers = await inquirer.prompt([
    {
      type: 'input',
      name: 'planningDir',
      message: 'Where should planning documents be stored?',
      default: 'docs/plans',
      validate: (input: string) => {
        if (!input || input.trim().length === 0) {
          return 'Directory path is required';
        }
        return true;
      },
    },
    {
      type: 'list',
      name: 'format',
      message: 'Which planning format do you use?',
      choices: [
        { name: 'SpecKit (GitHub spec-kit format)', value: 'speckit' },
        { name: 'BMAD (PRD/Architecture format)', value: 'bmad' },
        { name: 'Generic Markdown', value: 'generic' },
        { name: 'Skip example generation', value: 'skip' },
      ],
      default: 'generic',
    },
    {
      type: 'confirm',
      name: 'createExample',
      message: 'Create example planning document?',
      default: true,
      when: (answers: { format: string }) => answers.format !== 'skip',
    },
    {
      type: 'list',
      name: 'configTemplate',
      message: 'Configuration template:',
      choices: [
        { name: 'Basic (80% thresholds, recommended)', value: 'basic' },
        { name: 'Strict (90% thresholds, production-ready)', value: 'strict' },
        { name: 'CI-optimised (minimal checks, fast)', value: 'ci' },
      ],
      default: 'basic',
    },
  ] as any); // eslint-disable-line @typescript-eslint/no-explicit-any -- inquirer types require any

  // Gate checks configuration
  const recommendedChecks = detector.getRecommendedChecks(env);
  const checkAnswers = await inquirer.prompt([
    {
      type: 'confirm',
      name: 'eslint',
      message: `Enable ESLint gate?${env.hasEslint ? ' (detected)' : ''}`,
      default: env.hasEslint,
      when: recommendedChecks.includes('eslint') || env.hasEslint,
    },
    {
      type: 'confirm',
      name: 'test',
      message: `Enable test gate?${env.hasVitest || env.hasJest ? ' (detected)' : ''}`,
      default: env.hasVitest || env.hasJest,
      when: recommendedChecks.includes('test') || env.hasVitest || env.hasJest,
    },
    {
      type: 'confirm',
      name: 'coverage',
      message: 'Enable coverage gate?',
      default: true,
      when: (answers: { test: boolean }) => answers.test !== false,
    },
    {
      type: 'number',
      name: 'coverageThreshold',
      message: 'Coverage threshold (0-100):',
      default: answers.configTemplate === 'strict' ? 90 : 80,
      validate: (input: number) => (input >= 0 && input <= 100) || 'Must be between 0 and 100',
      when: (answers: { coverage: boolean }) => answers.coverage === true,
    },
    {
      type: 'confirm',
      name: 'secret',
      message: 'Enable secret scanning?',
      default: true,
    },
  ] as any); // eslint-disable-line @typescript-eslint/no-explicit-any -- inquirer types require any

  // Build enabled checks array
  const enabledChecks: string[] = [];
  if (checkAnswers.eslint) enabledChecks.push('eslint');
  if (checkAnswers.test) enabledChecks.push('test');
  if (checkAnswers.coverage) enabledChecks.push('coverage');
  if (checkAnswers.secret) enabledChecks.push('secret');

  return {
    projectRoot: process.cwd(),
    planningDir: answers.planningDir,
    format: answers.format as PlanningFormat,
    createExample: answers.createExample !== false,
    configTemplate: answers.configTemplate as ConfigTemplate,
    enabledChecks,
    coverageThreshold: checkAnswers.coverageThreshold || 80,
  };
}
