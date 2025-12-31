export const ANVIL_LOGO = `
    ▄▀█ █▄░█ █░█ █ █░░
    █▀█ █░▀█ ▀▄▀ █ █▄▄
`;

export const ANVIL_TAGLINE = 'Stop secrets and bad patterns before they reach git';

export const VALUE_PROPOSITION = `Protect your codebase from accidental secret commits and anti-patterns.

Anvil scans your code at commit time and blocks dangerous changes
before they enter your repository.`;

export const EDDACRAFT_BADGE = '╔═╗ ■ ╔═╗';
export const EDDACRAFT_TEXT = 'Part of EddaCraft';

export const QUICK_START_OPTIONS = [
  {
    key: 'init',
    label: 'Install protection',
    description: 'Add pre-commit hooks (30 seconds)',
    command: 'anvil init',
  },
  {
    key: 'check',
    label: 'Scan existing code',
    description: 'Check for secrets and issues now',
    command: 'anvil check --changed',
  },
  {
    key: 'help',
    label: 'View commands',
    description: 'See all available commands',
    command: 'anvil --help',
  },
  {
    key: 'skip',
    label: 'Skip',
    description: 'Continue without setup',
    command: null,
  },
] as const;

export type QuickStartOption = (typeof QUICK_START_OPTIONS)[number];
