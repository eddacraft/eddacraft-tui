export const theme = {
  colours: {
    success: '#64748b',
    error: '#dc2626',
    warning: '#fbbf24',
    info: '#94a3b8',
    muted: '#475569',
    primary: '#f97316',
    text: '#cbd5e1',
    border: '#334155',
    ember: '#f97316',
    emberBright: '#fb923c',
    emberDim: '#ea580c',
    steel: '#64748b',
    slag: '#dc2626',
    molten: '#fbbf24',
    ash: '#94a3b8',
    smoke: '#475569',
    charcoal: '#334155',
    void: '#0f172a',
  },
  icons: {
    success: '◆',
    error: '✖',
    warning: '◈',
    info: '◇',
    running: '●',
    skipped: '○',
    spinner: ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'],
    arrow: '▸',
    backArrow: '◄',
    check: '◆',
    cross: '✖',
    bullet: '▪',
    section: '━',
  },
  borders: {
    heavy: 'double',
    standard: 'single',
    light: 'round',
  },
} as const;

export type Theme = typeof theme;
export type ThemeColour = keyof typeof theme.colours;
export type ThemeIcon = keyof typeof theme.icons;
export type ThemeBorder = keyof typeof theme.borders;
