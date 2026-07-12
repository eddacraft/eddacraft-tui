import { z } from 'zod';

const safeEnum = <T extends readonly [string, ...string[]]>(values: T, fallback: T[number]) =>
  z.preprocess(
    (value) => (typeof value === 'string' && values.includes(value) ? value : fallback),
    z.enum(values)
  );

export const dashboardSearchSchema = z.object({
  severity: safeEnum(['all', 'high', 'medium', 'low'], 'all'),
  view: safeEnum(['runs', 'warnings'], 'runs'),
  evidence: z.preprocess(
    (value) => (typeof value === 'string' && value.length <= 128 ? value : undefined),
    z.string().optional()
  ),
});

export type DashboardSearch = z.infer<typeof dashboardSearchSchema>;
