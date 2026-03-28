---
id: frontend-component
name: Frontend Component
description: Create a reusable UI component with tests and documentation
category: frontend
tags: [react, component, ui, frontend, typescript]
variables:
  - name: component_name
    description: Name of the component (PascalCase)
    required: true
  - name: component_type
    description: Type of component (functional, class)
    default: functional
    required: false
  - name: styling
    description: Styling approach (css-modules, tailwind, styled-components)
    default: tailwind
    required: false
---

# Frontend Component: {{ component_name }}

## Intent

Create a reusable {{ component_name }} component with proper TypeScript types,
tests, and accessibility support.

## Changes

### 1. Create Component

- **File**: `src/components/{{ component_name }}/{{ component_name }}.tsx`
- **Action**: Create
- **Description**: Main component implementation

### 2. Create Types

- **File**: `src/components/{{ component_name }}/{{ component_name }}.types.ts`
- **Action**: Create
- **Description**: TypeScript interfaces for props and state

### 3. Create Tests

- **File**: `src/components/{{ component_name }}/{{ component_name }}.test.tsx`
- **Action**: Create
- **Description**: Unit tests for component behaviour

### 4. Create Stories (Optional)

- **File**:
  `src/components/{{ component_name }}/{{ component_name }}.stories.tsx`
- **Action**: Create
- **Description**: Storybook stories for visual testing

### 5. Create Index

- **File**: `src/components/{{ component_name }}/index.ts`
- **Action**: Create
- **Description**: Barrel export

### 6. Update Component Index

- **File**: `src/components/index.ts`
- **Action**: Modify
- **Description**: Export {{ component_name }} from components barrel

## Component Structure

```
src/components/{{ component_name }}/
├── {{ component_name }}.tsx
├── {{ component_name }}.types.ts
├── {{ component_name }}.test.tsx
├── {{ component_name }}.stories.tsx
└── index.ts
```

## Props Interface

```typescript
export interface {{ component_name }}Props {
  // Required props
  // Optional props
  className?: string;
  testId?: string;
}
```

## Accessibility Requirements

- [ ] Proper ARIA labels
- [ ] Keyboard navigation support
- [ ] Focus management
- [ ] Screen reader compatible
- [ ] Sufficient colour contrast

## Testing Requirements

- [ ] Renders without crashing
- [ ] Props work as expected
- [ ] User interactions handled
- [ ] Accessibility tests passing
- [ ] Snapshot tests (optional)

## Acceptance Criteria

- [ ] Component renders correctly
- [ ] TypeScript types complete
- [ ] Tests passing (>80% coverage)
- [ ] Accessible (WCAG 2.1 AA)
- [ ] Responsive design works
- [ ] Documentation complete
