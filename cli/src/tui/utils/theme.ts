export const theme = {
  colours: {
    success: '#22c55e',
    error: '#ef4444',
    warning: '#f59e0b',
    info: '#3b82f6',
    muted: '#6b7280',
    primary: '#8b5cf6',
    text: '#e5e7eb',
    border: '#374151',
  },
  icons: {
    success: '[ok]',
    error: '[x]',
    warning: '[!]',
    info: '[i]',
    spinner: ['-', '\\', '|', '/'],
    arrow: '>',
    check: '[ok]',
    cross: '[x]',
    bullet: '*',
  },
} as const;

export type Theme = typeof theme;
export type ThemeColour = keyof typeof theme.colours;
export type ThemeIcon = keyof typeof theme.icons;
