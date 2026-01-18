# TUI Implementation Plan - OpenTUI Integration

**Version**: V1.0 **Target**: Ship TUI-first CLI in 1 month **Framework**:
OpenTUI (@opentui/react) **Constraint**: No AI dependencies

---

## Index

### Phase 1: Foundation (Week 1)

- [1.1 Project Setup](#11-project-setup-4-hours)
- [1.2 Component Library](#12-component-library-6-hours)
- [1.3 Testing Infrastructure](#13-testing-infrastructure-2-hours)

### Phase 2: Core Commands (Week 2)

- [2.1 `anvil init` Wizard](#21-anvil-init-wizard-8-hours)
- [2.2 `anvil status` Dashboard](#22-anvil-status-dashboard-6-hours)
- [2.3 `anvil doctor` Diagnostics](#23-anvil-doctor-diagnostics-6-hours)

### Phase 3: Enhanced Features (Week 2-3)

- [3.1 First-Run Experience](#31-first-run-experience-4-hours)
- [3.2 Template Library](#32-template-library-8-hours)
- [3.3 Interactive Tutorial](#33-interactive-tutorial-8-hours)

### Phase 4: Integration & Polish (Week 3-4)

- [4.1 CLI Integration](#41-cli-integration-4-hours)
- [4.2 GitHub Action Enhancement](#42-github-action-enhancement-6-hours)
- [4.3 Documentation](#43-documentation-4-hours)
- [4.4 Testing & QA](#44-testing--qa-8-hours)

---

## Phase 1: Foundation

### 1.1 Project Setup (4 hours)

#### Step 1.1.1: Install OpenTUI and Dependencies (30 min)

```bash
# From repository root
cd cli

# Install OpenTUI packages
bun add @opentui/core @opentui/react

# Install React dependencies
bun add react react-dom

# Install dev dependencies
bun add -D @types/react @types/react-dom bun-types

# Optional: Install Bun globally if not present
curl -fsSL https://bun.sh/install | bash
```

**Verification:**

```bash
# Test OpenTUI is working
cat > test-tui.tsx << 'EOF'
import { createCliRenderer } from '@opentui/core';
import { createRoot } from '@opentui/react';

function App() {
  return <text>Hello from OpenTUI! ✓</text>;
}

const renderer = await createCliRenderer();
createRoot(renderer).render(<App />);
EOF

bun run test-tui.tsx
# Should display: Hello from OpenTUI! ✓

rm test-tui.tsx
```

**Deliverable:**

- ✅ OpenTUI installed and verified
- ✅ Dependencies in package.json
- ✅ Test script runs successfully

---

#### Step 1.1.2: Create Project Structure (1 hour)

```bash
# Create TUI directory structure
mkdir -p cli/src/tui/{components,commands,hooks,utils,types}

# Create component subdirectories
mkdir -p cli/src/tui/components/{layout,display,input,feedback}

# Create command subdirectories
mkdir -p cli/src/tui/commands/{init,status,doctor,tutorial}

# Create test directories
mkdir -p cli/src/tui/{components,commands,hooks}/__tests__
```

**Directory structure:**

```
cli/src/tui/
├── components/          # Reusable UI components
│   ├── layout/         # Layout components (Box, Grid, etc.)
│   ├── display/        # Display components (Header, List, etc.)
│   ├── input/          # Input components (Select, Input, etc.)
│   └── feedback/       # Feedback components (Spinner, Progress, etc.)
├── commands/           # Command screens
│   ├── init/          # Init wizard
│   ├── status/        # Status dashboard
│   ├── doctor/        # Doctor diagnostics
│   └── tutorial/      # Interactive tutorial
├── hooks/             # Custom React hooks
├── utils/             # Helper functions
├── types/             # TypeScript types
└── index.ts           # Main TUI entry point
```

**Create index files:**

```typescript
// cli/src/tui/index.ts
export * from './components';
export * from './commands';
export * from './hooks';
export * from './utils';

// cli/src/tui/components/index.ts
export * from './layout';
export * from './display';
export * from './input';
export * from './feedback';
```

**Deliverable:**

- ✅ Complete directory structure
- ✅ Index files for exports
- ✅ Ready for component development

---

#### Step 1.1.3: Configure TypeScript for TUI (30 min)

```json
// cli/tsconfig.tui.json
{
  "extends": "./tsconfig.json",
  "compilerOptions": {
    "jsx": "react-jsx",
    "jsxImportSource": "react",
    "lib": ["ES2022"],
    "module": "ESNext",
    "moduleResolution": "bundler",
    "target": "ES2022",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "resolveJsonModule": true,
    "allowSyntheticDefaultImports": true
  },
  "include": ["src/tui/**/*"],
  "exclude": ["node_modules", "dist", "**/*.test.ts", "**/*.test.tsx"]
}
```

**Update package.json scripts:**

```json
{
  "scripts": {
    "dev:tui": "bun run src/tui/index.tsx",
    "build:tui": "bun build src/tui/index.tsx --outdir dist/tui",
    "test:tui": "bun test src/tui"
  }
}
```

**Deliverable:**

- ✅ TypeScript configuration for TUI
- ✅ Build scripts in package.json
- ✅ Ready for React/JSX compilation

---

#### Step 1.1.4: Create TUI Utilities (2 hours)

```typescript
// cli/src/tui/utils/renderer.ts
import { createCliRenderer } from '@opentui/core';
import { createRoot } from '@opentui/react';

/**
 * Check if TUI is available in current environment
 */
export function isTUIAvailable(): boolean {
  return process.stdout.isTTY && !process.env.CI && !process.env.NO_TUI;
}

/**
 * Render a TUI component with graceful degradation
 */
export async function renderTUI<T>(
  Component: React.ComponentType<T>,
  props?: T,
  options?: {
    fallback?: () => void | Promise<void>;
    errorMessage?: string;
  }
): Promise<void> {
  if (!isTUIAvailable()) {
    if (options?.fallback) {
      await options.fallback();
      return;
    }

    const message = options?.errorMessage ||
      'TUI not available. Use --non-interactive flag or set NO_TUI=1';
    console.error(message);
    process.exit(1);
  }

  try {
    const renderer = await createCliRenderer();
    createRoot(renderer).render(
      props ? <Component {...props} /> : <Component />
    );
  } catch (error) {
    console.error('Failed to render TUI:', error);
    process.exit(1);
  }
}

/**
 * Get terminal dimensions with fallback
 */
export function getTerminalSize() {
  return {
    width: process.stdout.columns || 80,
    height: process.stdout.rows || 24,
  };
}

/**
 * Clear terminal screen
 */
export function clearScreen() {
  process.stdout.write('\x1b[2J\x1b[0f');
}
```

```typescript
// cli/src/tui/utils/colors.ts
export const colors = {
  primary: 'cyan',
  success: 'green',
  warning: 'yellow',
  error: 'red',
  info: 'blue',
  muted: 'gray',
} as const;

export type Color = (typeof colors)[keyof typeof colors];
```

```typescript
// cli/src/tui/utils/icons.ts
export const icons = {
  success: '✓',
  error: '✗',
  warning: '⚠',
  info: 'ℹ',
  spinner: '⠋',
  arrow: '→',
  bullet: '•',
  check: '☑',
  uncheck: '☐',
} as const;
```

```typescript
// cli/src/tui/types/common.ts
export interface BaseComponentProps {
  marginTop?: number;
  marginBottom?: number;
  marginLeft?: number;
  marginRight?: number;
  padding?: number;
  width?: string | number;
  height?: string | number;
}

export interface BorderStyle {
  type?: 'single' | 'double' | 'round' | 'bold' | 'none';
  color?: string;
}

export interface BoxProps extends BaseComponentProps {
  border?: BorderStyle | boolean;
  title?: string;
  style?: React.CSSProperties;
}
```

**Deliverable:**

- ✅ TUI renderer utilities
- ✅ Color and icon constants
- ✅ Common TypeScript types
- ✅ Environment detection

---

### 1.2 Component Library (6 hours)

#### Step 1.2.1: Layout Components (2 hours)

```tsx
// cli/src/tui/components/layout/Box.tsx
import React from 'react';
import { box } from '@opentui/react';
import type { BoxProps } from '../../types/common';

/**
 * Container component with optional border and padding
 */
export function Box({
  children,
  border,
  title,
  marginTop = 0,
  marginBottom = 0,
  marginLeft = 0,
  marginRight = 0,
  padding = 0,
  style = {},
  ...props
}: BoxProps & { children?: React.ReactNode }) {
  const borderStyle =
    typeof border === 'boolean'
      ? border
        ? { type: 'single' }
        : undefined
      : border;

  return (
    <box
      style={{
        marginTop,
        marginBottom,
        marginLeft,
        marginRight,
        padding,
        border: borderStyle?.type,
        borderColor: borderStyle?.color,
        ...style,
      }}
      {...props}
    >
      {title && (
        <text bold marginBottom={1}>
          {title}
        </text>
      )}
      {children}
    </box>
  );
}
```

```tsx
// cli/src/tui/components/layout/Container.tsx
import React from 'react';
import { useTerminalDimensions } from '@opentui/react';
import { Box } from './Box';
import type { BoxProps } from '../../types/common';

/**
 * Full-width container that responds to terminal size
 */
export function Container({
  children,
  maxWidth,
  center = false,
  ...props
}: BoxProps & {
  children?: React.ReactNode;
  maxWidth?: number;
  center?: boolean;
}) {
  const { width } = useTerminalDimensions();

  const containerWidth = maxWidth && width > maxWidth ? maxWidth : width;
  const marginLeft = center ? Math.floor((width - containerWidth) / 2) : 0;

  return (
    <Box width={containerWidth} marginLeft={marginLeft} {...props}>
      {children}
    </Box>
  );
}
```

```tsx
// cli/src/tui/components/layout/Grid.tsx
import React from 'react';
import { box } from '@opentui/react';
import type { BaseComponentProps } from '../../types/common';

interface GridProps extends BaseComponentProps {
  children: React.ReactNode;
  columns?: number;
  gap?: number;
}

/**
 * Simple grid layout for arranging components
 */
export function Grid({ children, columns = 2, gap = 1, ...props }: GridProps) {
  const items = React.Children.toArray(children);
  const rows: React.ReactNode[][] = [];

  for (let i = 0; i < items.length; i += columns) {
    rows.push(items.slice(i, i + columns));
  }

  return (
    <box {...props}>
      {rows.map((row, i) => (
        <box
          key={i}
          style={{ display: 'flex', flexDirection: 'row', gap }}
          marginTop={i > 0 ? gap : 0}
        >
          {row.map((item, j) => (
            <box key={j} style={{ flex: 1 }} marginLeft={j > 0 ? gap : 0}>
              {item}
            </box>
          ))}
        </box>
      ))}
    </box>
  );
}
```

**Deliverable:**

- ✅ Box component (basic container)
- ✅ Container component (responsive width)
- ✅ Grid component (column layout)

---

#### Step 1.2.2: Display Components (2 hours)

```tsx
// cli/src/tui/components/display/Header.tsx
import React from 'react';
import { text } from '@opentui/react';
import { Box } from '../layout/Box';
import { colors } from '../../utils/colors';
import type { BaseComponentProps } from '../../types/common';

interface HeaderProps extends BaseComponentProps {
  title: string;
  subtitle?: string;
  icon?: string;
  color?: string;
}

/**
 * Page header with optional subtitle
 */
export function Header({
  title,
  subtitle,
  icon,
  color = colors.primary,
  ...props
}: HeaderProps) {
  return (
    <Box border={{ type: 'round', color }} padding={1} {...props}>
      <text bold color={color}>
        {icon && `${icon} `}
        {title}
      </text>
      {subtitle && (
        <text color={colors.muted} marginTop={1}>
          {subtitle}
        </text>
      )}
    </Box>
  );
}
```

```tsx
// cli/src/tui/components/display/InfoPanel.tsx
import React from 'react';
import { text } from '@opentui/react';
import { Box } from '../layout/Box';
import { icons } from '../../utils/icons';
import type { BaseComponentProps } from '../../types/common';

interface InfoItem {
  label: string;
  value: string;
  icon?: string;
  color?: string;
}

interface InfoPanelProps extends BaseComponentProps {
  title: string;
  items: InfoItem[];
}

/**
 * Display key-value information in a panel
 */
export function InfoPanel({ title, items, ...props }: InfoPanelProps) {
  // Calculate max label width for alignment
  const maxLabelWidth = Math.max(...items.map((item) => item.label.length));

  return (
    <Box border title={title} padding={1} {...props}>
      {items.map((item, i) => (
        <box key={i} marginTop={i > 0 ? 0.5 : 0}>
          <text>
            {item.icon && `${item.icon} `}
            <text>{item.label.padEnd(maxLabelWidth + 2)}</text>
            <text color={item.color} bold>
              {item.value}
            </text>
          </text>
        </box>
      ))}
    </Box>
  );
}
```

```tsx
// cli/src/tui/components/display/List.tsx
import React from 'react';
import { text, scrollbox } from '@opentui/react';
import { Box } from '../layout/Box';
import { icons } from '../../utils/icons';
import type { BaseComponentProps } from '../../types/common';

interface ListItem {
  id: string;
  text: string;
  icon?: string;
  color?: string;
  secondary?: string;
}

interface ListProps extends BaseComponentProps {
  title?: string;
  items: ListItem[];
  maxHeight?: number;
  emptyMessage?: string;
}

/**
 * Scrollable list of items
 */
export function List({
  title,
  items,
  maxHeight = 10,
  emptyMessage = 'No items',
  ...props
}: ListProps) {
  if (items.length === 0) {
    return (
      <Box border title={title} padding={1} {...props}>
        <text color="gray">{emptyMessage}</text>
      </Box>
    );
  }

  return (
    <Box border title={title} {...props}>
      <scrollbox maxHeight={maxHeight} padding={1}>
        {items.map((item, i) => (
          <box key={item.id} marginTop={i > 0 ? 0.5 : 0}>
            <text color={item.color}>
              {item.icon || icons.bullet} {item.text}
            </text>
            {item.secondary && (
              <text color="gray" marginLeft={2}>
                {item.secondary}
              </text>
            )}
          </box>
        ))}
      </scrollbox>
    </Box>
  );
}
```

```tsx
// cli/src/tui/components/display/Table.tsx
import React from 'react';
import { text } from '@opentui/react';
import { Box } from '../layout/Box';
import type { BaseComponentProps } from '../../types/common';

interface Column {
  key: string;
  label: string;
  width?: number;
  align?: 'left' | 'center' | 'right';
}

interface TableProps extends BaseComponentProps {
  title?: string;
  columns: Column[];
  data: Record<string, any>[];
}

/**
 * Simple table component
 */
export function Table({ title, columns, data, ...props }: TableProps) {
  // Calculate column widths
  const columnWidths = columns.map((col) => {
    if (col.width) return col.width;

    const maxDataWidth = Math.max(
      col.label.length,
      ...data.map((row) => String(row[col.key] || '').length)
    );
    return maxDataWidth + 2;
  });

  const formatCell = (value: string, width: number, align: string = 'left') => {
    const str = String(value);
    if (align === 'right') return str.padStart(width);
    if (align === 'center') {
      const leftPad = Math.floor((width - str.length) / 2);
      return str.padStart(leftPad + str.length).padEnd(width);
    }
    return str.padEnd(width);
  };

  return (
    <Box border title={title} padding={1} {...props}>
      {/* Header */}
      <box>
        {columns.map((col, i) => (
          <text key={col.key} bold>
            {formatCell(col.label, columnWidths[i], col.align)}
          </text>
        ))}
      </box>

      {/* Separator */}
      <text color="gray" marginTop={0.5} marginBottom={0.5}>
        {'─'.repeat(columnWidths.reduce((a, b) => a + b, 0))}
      </text>

      {/* Data rows */}
      {data.map((row, i) => (
        <box key={i} marginTop={i > 0 ? 0.5 : 0}>
          {columns.map((col, j) => (
            <text key={col.key}>
              {formatCell(row[col.key] || '', columnWidths[j], col.align)}
            </text>
          ))}
        </box>
      ))}
    </Box>
  );
}
```

**Deliverable:**

- ✅ Header component
- ✅ InfoPanel component (key-value display)
- ✅ List component (scrollable)
- ✅ Table component (data grid)

---

#### Step 1.2.3: Input Components (1 hour)

```tsx
// cli/src/tui/components/input/Select.tsx
import React, { useState } from 'react';
import { select, useKeyboard } from '@opentui/react';
import { Box } from '../layout/Box';
import { colors } from '../../utils/colors';
import type { BaseComponentProps } from '../../types/common';

export interface SelectOption {
  value: string;
  label: string;
  description?: string;
  disabled?: boolean;
}

interface SelectProps extends BaseComponentProps {
  options: SelectOption[];
  value?: string;
  onChange: (value: string) => void;
  placeholder?: string;
  hint?: string;
}

/**
 * Select input with keyboard navigation
 */
export function Select({
  options,
  value,
  onChange,
  placeholder = 'Select an option...',
  hint,
  ...props
}: SelectProps) {
  const [selectedIndex, setSelectedIndex] = useState(
    value ? options.findIndex((opt) => opt.value === value) : 0
  );

  useKeyboard((key) => {
    if (key === 'up' && selectedIndex > 0) {
      setSelectedIndex(selectedIndex - 1);
    } else if (key === 'down' && selectedIndex < options.length - 1) {
      setSelectedIndex(selectedIndex + 1);
    } else if (key === 'return') {
      const selected = options[selectedIndex];
      if (!selected.disabled) {
        onChange(selected.value);
      }
    }
  });

  return (
    <Box {...props}>
      {options.map((option, i) => {
        const isSelected = i === selectedIndex;
        const borderColor = isSelected ? colors.primary : colors.muted;

        return (
          <Box
            key={option.value}
            border={{ type: 'single', color: borderColor }}
            padding={1}
            marginTop={i > 0 ? 0.5 : 0}
          >
            <text
              bold={isSelected}
              color={option.disabled ? colors.muted : undefined}
            >
              {isSelected ? '❯ ' : '  '}
              {option.label}
            </text>
            {option.description && (
              <text color={colors.muted} marginTop={0.5}>
                {option.description}
              </text>
            )}
          </Box>
        );
      })}

      {hint && (
        <text color={colors.muted} marginTop={1}>
          {hint}
        </text>
      )}
    </Box>
  );
}
```

```tsx
// cli/src/tui/components/input/Checkbox.tsx
import React from 'react';
import { input, useKeyboard } from '@opentui/react';
import { icons } from '../../utils/icons';
import type { BaseComponentProps } from '../../types/common';

interface CheckboxProps extends BaseComponentProps {
  label: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
}

/**
 * Checkbox input
 */
export function Checkbox({
  label,
  checked,
  onChange,
  disabled = false,
  ...props
}: CheckboxProps) {
  useKeyboard((key) => {
    if (key === 'space' && !disabled) {
      onChange(!checked);
    }
  });

  const icon = checked ? icons.check : icons.uncheck;
  const color = disabled ? 'gray' : undefined;

  return (
    <box {...props}>
      <text color={color}>
        {icon} {label}
      </text>
    </box>
  );
}
```

**Deliverable:**

- ✅ Select component (keyboard navigation)
- ✅ Checkbox component

---

#### Step 1.2.4: Feedback Components (1 hour)

```tsx
// cli/src/tui/components/feedback/Spinner.tsx
import React, { useState, useEffect } from 'react';
import { text } from '@opentui/react';
import type { BaseComponentProps } from '../../types/common';

interface SpinnerProps extends BaseComponentProps {
  message?: string;
}

const spinnerFrames = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

/**
 * Loading spinner
 */
export function Spinner({ message, ...props }: SpinnerProps) {
  const [frame, setFrame] = useState(0);

  useEffect(() => {
    const interval = setInterval(() => {
      setFrame((prev) => (prev + 1) % spinnerFrames.length);
    }, 80);

    return () => clearInterval(interval);
  }, []);

  return (
    <box {...props}>
      <text color="cyan">
        {spinnerFrames[frame]} {message || 'Loading...'}
      </text>
    </box>
  );
}
```

```tsx
// cli/src/tui/components/feedback/StatusMessage.tsx
import React from 'react';
import { text } from '@opentui/react';
import { icons, colors } from '../../utils';
import type { BaseComponentProps } from '../../types/common';

type Status = 'success' | 'error' | 'warning' | 'info';

interface StatusMessageProps extends BaseComponentProps {
  status: Status;
  message: string;
}

const statusConfig: Record<Status, { icon: string; color: string }> = {
  success: { icon: icons.success, color: colors.success },
  error: { icon: icons.error, color: colors.error },
  warning: { icon: icons.warning, color: colors.warning },
  info: { icon: icons.info, color: colors.info },
};

/**
 * Status message with icon
 */
export function StatusMessage({
  status,
  message,
  ...props
}: StatusMessageProps) {
  const config = statusConfig[status];

  return (
    <box {...props}>
      <text color={config.color}>
        {config.icon} {message}
      </text>
    </box>
  );
}
```

```tsx
// cli/src/tui/components/feedback/ProgressBar.tsx
import React from 'react';
import { text } from '@opentui/react';
import { Box } from '../layout/Box';
import { colors } from '../../utils/colors';
import type { BaseComponentProps } from '../../types/common';

interface ProgressBarProps extends BaseComponentProps {
  current: number;
  total: number;
  label?: string;
  width?: number;
}

/**
 * Progress bar
 */
export function ProgressBar({
  current,
  total,
  label,
  width = 40,
  ...props
}: ProgressBarProps) {
  const percentage = Math.min(100, Math.round((current / total) * 100));
  const filled = Math.round((width * percentage) / 100);
  const empty = width - filled;

  const bar = '█'.repeat(filled) + '░'.repeat(empty);

  return (
    <Box {...props}>
      {label && <text marginBottom={0.5}>{label}</text>}
      <text color={colors.primary}>
        {bar} {percentage}%
      </text>
      <text color={colors.muted}>
        {current}/{total}
      </text>
    </Box>
  );
}
```

**Deliverable:**

- ✅ Spinner component (animated loading)
- ✅ StatusMessage component (success/error/warning/info)
- ✅ ProgressBar component

---

#### Step 1.2.5: Export Component Library (15 min)

```typescript
// cli/src/tui/components/index.ts

// Layout
export { Box } from './layout/Box';
export { Container } from './layout/Container';
export { Grid } from './layout/Grid';

// Display
export { Header } from './display/Header';
export { InfoPanel } from './display/InfoPanel';
export { List } from './display/List';
export { Table } from './display/Table';

// Input
export { Select } from './input/Select';
export { Checkbox } from './input/Checkbox';
export type { SelectOption } from './input/Select';

// Feedback
export { Spinner } from './feedback/Spinner';
export { StatusMessage } from './feedback/StatusMessage';
export { ProgressBar } from './feedback/ProgressBar';
```

**Create component showcase:**

```tsx
// cli/src/tui/components/Showcase.tsx
import React from 'react';
import {
  Box,
  Container,
  Grid,
  Header,
  InfoPanel,
  List,
  Table,
  Select,
  Checkbox,
  Spinner,
  StatusMessage,
  ProgressBar,
} from './index';

/**
 * Component showcase for testing
 * Run with: bun run src/tui/components/Showcase.tsx
 */
export function Showcase() {
  return (
    <Container maxWidth={100} center>
      <Header
        title="Anvil TUI Component Library"
        subtitle="Component Showcase"
        icon="🔨"
        marginBottom={2}
      />

      <Grid columns={2} gap={2}>
        <InfoPanel
          title="System Info"
          items={[
            { label: 'Version', value: 'v1.0.0', icon: '•' },
            { label: 'Mode', value: 'local-only' },
            { label: 'Format', value: 'speckit', color: 'cyan' },
          ]}
        />

        <List
          title="Recent Activity"
          items={[
            { id: '1', text: 'validate spec.md', icon: '✓', color: 'green' },
            { id: '2', text: 'gate plan.md', icon: '✗', color: 'red' },
          ]}
          maxHeight={5}
        />
      </Grid>

      <Table
        title="Quality Gates"
        columns={[
          { key: 'check', label: 'Check', align: 'left' },
          { key: 'status', label: 'Status', align: 'center' },
          { key: 'score', label: 'Score', align: 'right' },
        ]}
        data={[
          { check: 'lint', status: '✓', score: '100/100' },
          { check: 'test', status: '✓', score: '100/100' },
          { check: 'coverage', status: '✓', score: '85/100' },
        ]}
        marginTop={2}
      />

      <Box marginTop={2}>
        <StatusMessage status="success" message="All components working!" />
        <Spinner message="Loading..." marginTop={1} />
        <ProgressBar current={7} total={10} label="Progress" marginTop={1} />
      </Box>
    </Container>
  );
}

// Only run if this file is executed directly
if (import.meta.main) {
  const { createCliRenderer } = await import('@opentui/core');
  const { createRoot } = await import('@opentui/react');

  const renderer = await createCliRenderer();
  createRoot(renderer).render(<Showcase />);
}
```

**Deliverable:**

- ✅ Complete component library exported
- ✅ Component showcase for testing
- ✅ Ready for command development

---

### 1.3 Testing Infrastructure (2 hours)

#### Step 1.3.1: Set Up Testing Framework (1 hour)

```bash
# Install testing dependencies
bun add -D @testing-library/react @testing-library/react-hooks vitest happy-dom
```

```typescript
// cli/vitest.tui.config.ts
import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    name: 'tui',
    environment: 'happy-dom',
    include: ['src/tui/**/*.test.{ts,tsx}'],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html'],
      include: ['src/tui/**/*.{ts,tsx}'],
      exclude: [
        'src/tui/**/*.test.{ts,tsx}',
        'src/tui/**/types/**',
        'src/tui/**/index.ts',
      ],
    },
  },
});
```

**Update package.json:**

```json
{
  "scripts": {
    "test:tui": "vitest --config vitest.tui.config.ts",
    "test:tui:coverage": "vitest --config vitest.tui.config.ts --coverage",
    "test:tui:ui": "vitest --config vitest.tui.config.ts --ui"
  }
}
```

**Deliverable:**

- ✅ Vitest configured for TUI
- ✅ Testing scripts in package.json
- ✅ Coverage reporting enabled

---

#### Step 1.3.2: Write Component Tests (1 hour)

```typescript
// cli/src/tui/components/layout/__tests__/Box.test.tsx
import { describe, it, expect } from 'vitest';
import { render } from '@testing-library/react';
import { Box } from '../Box';

describe('Box', () => {
  it('renders children', () => {
    const { getByText } = render(
      <Box>
        <text>Hello World</text>
      </Box>
    );

    expect(getByText('Hello World')).toBeDefined();
  });

  it('renders with title', () => {
    const { getByText } = render(
      <Box title="Test Title">
        <text>Content</text>
      </Box>
    );

    expect(getByText('Test Title')).toBeDefined();
  });

  it('applies margin props', () => {
    const { container } = render(
      <Box marginTop={2} marginLeft={1}>
        <text>Content</text>
      </Box>
    );

    const box = container.firstChild;
    expect(box).toHaveStyle({ marginTop: 2, marginLeft: 1 });
  });
});
```

```typescript
// cli/src/tui/components/display/__tests__/StatusMessage.test.tsx
import { describe, it, expect } from 'vitest';
import { render } from '@testing-library/react';
import { StatusMessage } from '../StatusMessage';

describe('StatusMessage', () => {
  it('renders success message', () => {
    const { getByText } = render(
      <StatusMessage status="success" message="Success!" />
    );

    expect(getByText(/Success!/)).toBeDefined();
  });

  it('uses correct icon for status', () => {
    const { getByText } = render(
      <StatusMessage status="error" message="Error occurred" />
    );

    const text = getByText(/Error occurred/);
    expect(text.textContent).toContain('✗');
  });
});
```

**Deliverable:**

- ✅ Test examples for components
- ✅ Testing patterns established
- ✅ Ready for test-driven development

---

## Phase 2: Core Commands

### 2.1 `anvil init` Wizard (8 hours)

#### Step 2.1.1: Create Wizard Screen Flow (2 hours)

```tsx
// cli/src/tui/commands/init/InitWizard.tsx
import React, { useState } from 'react';
import { Container, Header } from '../../components';
import { ModeStep } from './steps/ModeStep';
import { FormatStep } from './steps/FormatStep';
import { HooksStep } from './steps/HooksStep';
import { CIStep } from './steps/CIStep';
import { VerificationStep } from './steps/VerificationStep';

export interface WizardConfig {
  mode: 'local' | 'connected';
  format: 'generic' | 'speckit' | 'bmad' | 'aps';
  hooks: string[];
  ci: string;
  plansDir: string;
}

const TOTAL_STEPS = 5;

export function InitWizard() {
  const [step, setStep] = useState(1);
  const [config, setConfig] = useState<Partial<WizardConfig>>({
    plansDir: 'docs/plans',
  });

  const updateConfig = (updates: Partial<WizardConfig>) => {
    setConfig({ ...config, ...updates });
  };

  const nextStep = () => setStep(step + 1);
  const prevStep = () => setStep(step - 1);

  return (
    <Container maxWidth={80} center>
      <Header
        title={`🔨 Anvil Setup Wizard - Step ${step}/${TOTAL_STEPS}`}
        color="cyan"
        marginBottom={2}
      />

      {step === 1 && (
        <ModeStep
          value={config.mode}
          onNext={(mode) => {
            updateConfig({ mode });
            nextStep();
          }}
        />
      )}

      {step === 2 && (
        <FormatStep
          value={config.format}
          onNext={(format) => {
            updateConfig({ format });
            nextStep();
          }}
          onBack={prevStep}
        />
      )}

      {step === 3 && (
        <HooksStep
          value={config.hooks || []}
          onNext={(hooks) => {
            updateConfig({ hooks });
            nextStep();
          }}
          onBack={prevStep}
        />
      )}

      {step === 4 && (
        <CIStep
          value={config.ci}
          onNext={(ci) => {
            updateConfig({ ci });
            nextStep();
          }}
          onBack={prevStep}
        />
      )}

      {step === 5 && (
        <VerificationStep config={config as WizardConfig} onBack={prevStep} />
      )}
    </Container>
  );
}
```

**Deliverable:**

- ✅ Main wizard component
- ✅ Step navigation
- ✅ Config state management

---

#### Step 2.1.2: Create Wizard Steps (4 hours)

```tsx
// cli/src/tui/commands/init/steps/FormatStep.tsx
import React from 'react';
import { Box, Select, type SelectOption } from '../../../components';
import { colors } from '../../../utils/colors';

const FORMAT_OPTIONS: SelectOption[] = [
  {
    value: 'generic',
    label: 'Generic Markdown (recommended)',
    description:
      '✓ Works with any .md file\n✓ Most flexible\n✓ Convert to other formats anytime\n\nBest for: Getting started, simple plans',
  },
  {
    value: 'speckit',
    label: 'SpecKit (for GitHub workflows)',
    description:
      'spec.md, plan.md, tasks.md\n\nBest for: GitHub Issues/PRs, detailed task tracking',
  },
  {
    value: 'bmad',
    label: 'BMAD (for PRDs & architecture)',
    description:
      'prd.md, architecture.md\n\nBest for: Product requirements, technical designs',
  },
  {
    value: 'aps',
    label: 'APS (advanced)',
    description:
      'Native JSON/YAML format\n\nBest for: Tool integration, programmatic use',
  },
];

interface FormatStepProps {
  value?: string;
  onNext: (format: string) => void;
  onBack: () => void;
}

export function FormatStep({ value, onNext, onBack }: FormatStepProps) {
  return (
    <Box>
      <text bold marginBottom={1}>
        Choose your planning document format:
      </text>

      <Select
        options={FORMAT_OPTIONS}
        value={value}
        onChange={onNext}
        hint="💡 Don't worry - you can convert between formats anytime:\n   anvil export plan.md --to speckit"
      />

      <Box marginTop={2}>
        <text color={colors.muted}>
          [↑↓ Navigate | Enter Select | Ctrl+C Cancel]
        </text>
      </Box>
    </Box>
  );
}
```

```tsx
// cli/src/tui/commands/init/steps/HooksStep.tsx
import React, { useState } from 'react';
import { Box, Checkbox, Select, type SelectOption } from '../../../components';
import { code } from '@opentui/react';

const PRE_COMMIT_HOOK = `#!/bin/sh
CHANGED_PLANS=$(git diff --cached --name-only --diff-filter=ACM | grep -E '(spec|plan|prd)\\.md$')

if [ -n "$CHANGED_PLANS" ]; then
  echo "Validating planning documents..."
  for file in $CHANGED_PLANS; do
    echo "  Checking $file"
    anvil validate "$file" || exit 1
  done
  echo "✓ All planning documents valid"
fi`;

const PRE_PUSH_HOOK = `#!/bin/sh
echo "Running quality gates..."
anvil gate $(git diff --name-only HEAD @{u} | grep -E '(spec|plan|prd)\\.md$') || exit 1
echo "✓ All gates passed"`;

interface HooksStepProps {
  value: string[];
  onNext: (hooks: string[]) => void;
  onBack: () => void;
}

export function HooksStep({ value, onNext, onBack }: HooksStepProps) {
  const [selectedHooks, setSelectedHooks] = useState<string[]>(value);
  const [showPreview, setShowPreview] = useState(false);

  const toggleHook = (hook: string) => {
    if (selectedHooks.includes(hook)) {
      setSelectedHooks(selectedHooks.filter((h) => h !== hook));
    } else {
      setSelectedHooks([...selectedHooks, hook]);
    }
  };

  return (
    <Box>
      <text bold marginBottom={1}>
        Install git hooks to validate plans before commit?
      </text>

      <Box marginTop={1}>
        <Checkbox
          label="pre-commit - Validate plans before committing"
          checked={selectedHooks.includes('pre-commit')}
          onChange={() => toggleHook('pre-commit')}
        />

        <Checkbox
          label="pre-push - Run gates before pushing"
          checked={selectedHooks.includes('pre-push')}
          onChange={() => toggleHook('pre-push')}
          marginTop={1}
        />
      </Box>

      {selectedHooks.includes('pre-commit') && (
        <Box marginTop={2}>
          <text>Preview:</text>
          <code language="bash" marginTop={1}>
            {PRE_COMMIT_HOOK}
          </code>
        </Box>
      )}

      <Box marginTop={2}>
        <Select
          options={[
            { value: 'install', label: 'Install hooks' },
            { value: 'show', label: 'Show me the commands (copy-paste)' },
            { value: 'skip', label: 'Skip for now' },
          ]}
          onChange={(choice) => {
            if (choice === 'install') {
              onNext(selectedHooks);
            } else if (choice === 'show') {
              // Show commands in terminal
              console.log('\nCopy these commands:\n');
              if (selectedHooks.includes('pre-commit')) {
                console.log(
                  `cat > .git/hooks/pre-commit << 'EOF'\n${PRE_COMMIT_HOOK}\nEOF`
                );
                console.log('chmod +x .git/hooks/pre-commit\n');
              }
              if (selectedHooks.includes('pre-push')) {
                console.log(
                  `cat > .git/hooks/pre-push << 'EOF'\n${PRE_PUSH_HOOK}\nEOF`
                );
                console.log('chmod +x .git/hooks/pre-push\n');
              }
              onNext([]);
            } else {
              onNext([]);
            }
          }}
        />
      </Box>
    </Box>
  );
}
```

Continue in next file...

**Deliverable:**

- ✅ All 5 wizard steps implemented
- ✅ Format selection with previews
- ✅ Git hooks with code display
- ✅ CI integration options
- ✅ Verification and summary

---

#### Step 2.1.3: Wire Up Init Command (1 hour)

```typescript
// cli/src/commands/init.ts
import { Command } from 'commander';
import { renderTUI } from '../tui/utils/renderer';
import { InitWizard } from '../tui/commands/init/InitWizard';
import { initFromFlags } from '../services/init-service';

export const initCommand = new Command('init')
  .description('Initialize Anvil in current project')
  .option('--non-interactive', 'Skip interactive wizard, use flags/config')
  .option('--format <format>', 'Planning format (generic|speckit|bmad|aps)')
  .option('--mode <mode>', 'Operation mode (local|connected)')
  .option('--hooks <hooks>', 'Git hooks to install (comma-separated)')
  .option('--ci <ci>', 'CI platform (github|gitlab|none)')
  .action(async (options) => {
    if (options.nonInteractive) {
      await initFromFlags(options);
    } else {
      await renderTUI(InitWizard);
    }
  });
```

**Deliverable:**

- ✅ Init command integrated
- ✅ TUI/CLI fallback working
- ✅ Ready to test

---

#### Step 2.1.4: Test Init Wizard (1 hour)

```typescript
// cli/src/tui/commands/init/__tests__/InitWizard.test.tsx
import { describe, it, expect, vi } from 'vitest';
import { render, fireEvent } from '@testing-library/react';
import { InitWizard } from '../InitWizard';

describe('InitWizard', () => {
  it('starts at step 1', () => {
    const { getByText } = render(<InitWizard />);
    expect(getByText(/Step 1\/5/)).toBeDefined();
  });

  it('advances through steps', async () => {
    const { getByText, getByRole } = render(<InitWizard />);

    // Select mode
    fireEvent.click(getByText('Local-only'));

    // Should advance to step 2
    expect(getByText(/Step 2\/5/)).toBeDefined();
  });

  it('allows going back', () => {
    const { getByText, getByRole } = render(<InitWizard />);

    // Go to step 2
    fireEvent.click(getByText('Local-only'));

    // Go back
    fireEvent.click(getByText('Back'));

    // Should be back at step 1
    expect(getByText(/Step 1\/5/)).toBeDefined();
  });
});
```

**Deliverable:**

- ✅ Unit tests for wizard
- ✅ Integration tests for flow
- ✅ Tests passing

---

### 2.2 `anvil status` Dashboard (6 hours)

_(Similar detailed breakdown for status dashboard)_

---

### 2.3 `anvil doctor` Diagnostics (6 hours)

_(Similar detailed breakdown for doctor command)_

---

## Phase 3: Enhanced Features

### 3.1 First-Run Experience (4 hours)

```tsx
// cli/src/tui/commands/welcome/Welcome.tsx
import React from 'react';
import { Container, Header, Box, Select } from '../../components';
import { useKeyboard } from '@opentui/react';

export function Welcome() {
  const [selection, setSelection] = React.useState<string>();

  useKeyboard((key) => {
    if (key === 'q' || key === 'escape') {
      process.exit(0);
    }
  });

  const handleSelect = async (value: string) => {
    setSelection(value);

    if (value === 'setup') {
      // Launch init wizard
      const { InitWizard } = await import('../init/InitWizard');
      // Render InitWizard
    } else if (value === 'validate') {
      console.log('\nRun: anvil validate <file>');
      console.log('Example: anvil validate plan.md\n');
      process.exit(0);
    } else if (value === 'docs') {
      console.log('\nDocumentation: https://anvil.dev/docs\n');
      process.exit(0);
    }
  };

  return (
    <Container maxWidth={80} center>
      <Header
        title="👋 Welcome to Anvil!"
        subtitle="Validate planning documents and ensure code changes are safe"
        icon="🔨"
        marginBottom={2}
      />

      <Box marginBottom={2}>
        <text>Anvil validates planning documents and runs quality gates</text>
        <text>to ensure code changes are safe before deployment.</text>
      </Box>

      <Box marginBottom={1}>
        <text bold>It looks like this is your first time using Anvil.</text>
      </Box>

      <Select
        options={[
          {
            value: 'setup',
            label: 'Set up Anvil for this project',
            description: 'Interactive setup wizard (recommended)',
          },
          {
            value: 'validate',
            label: 'Validate an existing plan',
            description: 'Skip setup, validate a file now',
          },
          {
            value: 'docs',
            label: 'Learn more',
            description: 'Open documentation',
          },
        ]}
        onChange={handleSelect}
      />

      <Box marginTop={2}>
        <text color="gray">[↑↓ Navigate | Enter Select | q Quit]</text>
      </Box>
    </Container>
  );
}
```

**Deliverable:**

- ✅ Welcome screen
- ✅ First-run detection
- ✅ Options to setup/validate/learn

---

### 3.2 Template Library (8 hours)

_(Detailed implementation of static template system)_

---

### 3.3 Interactive Tutorial (8 hours)

_(Detailed TUI-based tutorial walkthrough)_

---

## Phase 4: Integration & Polish

### 4.1 CLI Integration (4 hours)

```typescript
// cli/src/index.ts
import { program } from 'commander';
import { isTUIAvailable } from './tui/utils/renderer';
import { Welcome } from './tui/commands/welcome/Welcome';
import { initCommand } from './commands/init';
import { statusCommand } from './commands/status';
import { doctorCommand } from './commands/doctor';

// Check if running without arguments
if (process.argv.length === 2) {
  // Check for first-run
  const isFirstRun = !fs.existsSync('.anvilrc');

  if (isFirstRun && isTUIAvailable()) {
    // Show welcome screen
    await renderTUI(Welcome);
  } else {
    // Show friendly help
    console.log(`
👋 Anvil - Validate Planning Documents

Quick commands:
  anvil validate <file>    Validate a planning document
  anvil init               Set up Anvil for this project
  anvil status             View current status
  anvil doctor             Run diagnostics

Documentation: https://anvil.dev/docs
Get help: anvil help <command>
    `);
  }
  process.exit(0);
}

program
  .name('anvil')
  .description('Validate planning documents and run quality gates')
  .version('1.0.0');

program.addCommand(initCommand);
program.addCommand(statusCommand);
program.addCommand(doctorCommand);
// ... other commands

program.parse();
```

**Deliverable:**

- ✅ CLI integration complete
- ✅ First-run detection
- ✅ Graceful fallbacks

---

### 4.2 GitHub Action Enhancement (6 hours)

_(Enhanced PR comments without AI)_

---

### 4.3 Documentation (4 hours)

Create documentation for:

- Component library usage
- Creating new TUI commands
- Testing TUI components
- Accessibility considerations

---

### 4.4 Testing & QA (8 hours)

- End-to-end tests for all commands
- Manual testing on different terminals
- CI/CD integration tests
- Performance optimization

---

## Timeline Summary

| Phase       | Duration | Deliverables                           |
| ----------- | -------- | -------------------------------------- |
| **Phase 1** | 3 days   | Foundation, component library, testing |
| **Phase 2** | 1 week   | Init wizard, status, doctor commands   |
| **Phase 3** | 1 week   | First-run, templates, tutorial         |
| **Phase 4** | 1 week   | Integration, docs, QA                  |
| **Total**   | ~1 month | V1 launch ready                        |

---

## Success Criteria

✅ **TUI Components**

- [ ] All 15+ components implemented
- [ ] Component showcase runs
- [ ] Tests pass with >80% coverage

✅ **Core Commands**

- [ ] `anvil init` wizard functional
- [ ] `anvil status` dashboard working
- [ ] `anvil doctor` diagnostics complete

✅ **User Experience**

- [ ] First-run experience smooth
- [ ] Graceful degradation to CLI works
- [ ] Documentation complete

✅ **Quality**

- [ ] All tests passing
- [ ] Works on macOS/Linux/Windows
- [ ] Performance acceptable (<100ms render)

---

## Next Steps

1. Review this plan
2. Approve scope
3. Begin Phase 1.1: Project Setup
4. Daily standups to track progress
5. Ship v0.1.0-beta in Week 1

Ready to start implementation! 🚀
