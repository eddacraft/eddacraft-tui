export const ANVIL_LOGO = `
    ▄▀█ █▄░█ █░█ █ █░░
    █▀█ █░▀█ ▀▄▀ █ █▄▄
`;

export const ANVIL_TAGLINE = 'Forge safe code changes';

export const VALUE_PROPOSITION = `Make AI-generated code changes safe for production.

Validates plans through quality gates, maintains audit trails,
and ensures every change is reversible.`;

export const EDDACRAFT_BADGE = '╔═╗ ■ ╔═╗';
export const EDDACRAFT_TEXT = 'Part of eddacraft';

export const QUICK_START_OPTIONS = [
  {
    key: 'login',
    label: 'Authenticate Beta Access',
    description: 'Sign in with your beta token (recommended)',
    command: 'anvil login',
  },
  {
    key: 'tutorial',
    label: 'Interactive Tutorial',
    description: 'Learn Anvil step-by-step after login',
    command: 'anvil tutorial',
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
