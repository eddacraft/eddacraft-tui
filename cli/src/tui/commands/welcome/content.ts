export const ANVIL_LOGO = `
    ▄▀█ █▄░█ █░█ █ █░░
    █▀█ █░▀█ ▀▄▀ █ █▄▄
`;

export const ANVIL_TAGLINE = 'Forge safe code changes';

export const VALUE_PROPOSITION = `Make AI-generated code changes safe for production.

Validates plans through quality gates, maintains audit trails,
and ensures every change is reversible.`;

export const EDDACRAFT_BADGE = '╔═╗ ■ ╔═╗';
export const EDDACRAFT_TEXT = 'Part of EddaCraft';

export const QUICK_START_OPTIONS = [
  {
    key: 'init',
    label: 'Initialise Anvil',
    description: 'Set up Anvil in this project',
    command: 'anvil init',
  },
  {
    key: 'doctor',
    label: 'Run diagnostics',
    description: 'Check your environment setup',
    command: 'anvil doctor',
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
