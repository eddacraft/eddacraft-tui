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
    success: '\u2714',
    error: '\u2718',
    warning: '\u26A0',
    info: '\u2139',
    spinner: [
      '\u280B',
      '\u2819',
      '\u2839',
      '\u2838',
      '\u283C',
      '\u2834',
      '\u2826',
      '\u2827',
      '\u2807',
      '\u280F',
    ],
    arrow: '\u276F',
    check: '\u2713',
    cross: '\u2717',
    bullet: '\u2022',
  },
} as const;

export type Theme = typeof theme;
export type ThemeColour = keyof typeof theme.colours;
export type ThemeIcon = keyof typeof theme.icons;
