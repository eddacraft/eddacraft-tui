export const dashboardQueryKeys = {
  protection: {
    overview: () => ['dashboard', 'protection', 'overview'] as const,
  },
  patterns: {
    catalogue: () => ['dashboard', 'patterns', 'catalogue'] as const,
  },
  plans: {
    all: () => ['dashboard', 'plans'] as const,
    detail: (id: string) => ['dashboard', 'plans', 'detail', id] as const,
  },
} as const;
