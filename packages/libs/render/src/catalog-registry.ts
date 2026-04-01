import { createElement } from 'react';
import { schema } from '@json-render/react';
import { defineRegistry } from '@json-render/react';
import { shadcnComponentDefinitions } from '@json-render/shadcn/catalog';
import { shadcnComponents } from '@json-render/shadcn';
import { z } from 'zod';

import { MetricCard, type MetricCardProps } from './components/metric-card.js';
import { StatusBadge, type StatusBadgeProps } from './components/status-badge.js';

/**
 * Anvil component catalog — shadcn built-ins plus custom Anvil components.
 */
export const catalog = schema.createCatalog({
  components: {
    // shadcn built-ins
    Card: shadcnComponentDefinitions.Card,
    Stack: shadcnComponentDefinitions.Stack,
    Grid: shadcnComponentDefinitions.Grid,
    Heading: shadcnComponentDefinitions.Heading,
    Text: shadcnComponentDefinitions.Text,
    Badge: shadcnComponentDefinitions.Badge,
    Separator: shadcnComponentDefinitions.Separator,
    Table: shadcnComponentDefinitions.Table,
    Alert: shadcnComponentDefinitions.Alert,
    Progress: shadcnComponentDefinitions.Progress,

    // Custom Anvil components
    MetricCard: {
      props: z.object({
        label: z.string(),
        value: z.string(),
        trend: z.enum(['up', 'down', 'flat']).nullable().optional(),
        format: z.enum(['number', 'percent', 'duration']).nullable().optional(),
      }),
      description: 'Single metric value with optional trend indicator',
      example: { label: 'Gate Pass Rate', value: '94%', trend: 'up', format: 'percent' },
    },
    StatusBadge: {
      props: z.object({
        status: z.enum(['pass', 'fail', 'warn', 'info']),
        label: z.string(),
      }),
      description: 'Pass/fail/warn/info status indicator',
      example: { status: 'pass', label: 'secret-detection: clean' },
    },
  },
  actions: {},
});

/**
 * Component registry — maps catalog entries to React implementations.
 *
 * The shadcn components are typed against their own catalog, so we cast them
 * to fit the combined catalog's ComponentFn type. The prop shapes are
 * identical — the cast is purely a TypeScript generic mismatch.
 */
// eslint-disable-next-line @typescript-eslint/no-explicit-any -- shadcn registry types don't align with combined catalog generics; cast is safe because prop shapes are identical
const components: any = {
  // shadcn built-ins
  Card: shadcnComponents.Card,
  Stack: shadcnComponents.Stack,
  Grid: shadcnComponents.Grid,
  Heading: shadcnComponents.Heading,
  Text: shadcnComponents.Text,
  Badge: shadcnComponents.Badge,
  Separator: shadcnComponents.Separator,
  Table: shadcnComponents.Table,
  Alert: shadcnComponents.Alert,
  Progress: shadcnComponents.Progress,

  // Custom Anvil components — use createElement since this is a .ts file
  MetricCard: ({ props }: { props: MetricCardProps }) => createElement(MetricCard, props),
  StatusBadge: ({ props }: { props: StatusBadgeProps }) => createElement(StatusBadge, props),
};

export const { registry } = defineRegistry(catalog, { components });
