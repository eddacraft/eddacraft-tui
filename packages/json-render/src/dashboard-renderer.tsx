'use client';

import { Component, type ErrorInfo, type ReactNode } from 'react';
import { Renderer } from '@json-render/react';
import type { Spec } from '@json-render/core';

import { registry } from './catalog-registry.js';
import { validateSpec } from './schema-validator.js';

// ---------------------------------------------------------------------------
// Error boundary
// ---------------------------------------------------------------------------

interface ErrorBoundaryProps {
  children: ReactNode;
}

interface ErrorBoundaryState {
  error: Error | null;
}

class RenderErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  constructor(props: ErrorBoundaryProps) {
    super(props);
    this.state = { error: null };
  }

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { error };
  }

  override componentDidCatch(error: Error, info: ErrorInfo): void {
    console.error('[DashboardRenderer] render error:', error, info);
  }

  override render(): ReactNode {
    if (this.state.error) {
      return (
        <div
          role="alert"
          style={{
            padding: '1rem',
            border: '1px solid var(--anvil)',
            backgroundColor: 'var(--surface)',
            color: 'var(--text-primary)',
            fontFamily: 'monospace',
          }}
        >
          <strong>Render error</strong>
          <pre style={{ whiteSpace: 'pre-wrap', marginTop: '0.5rem', color: 'var(--text-muted)' }}>
            {this.state.error.message}
          </pre>
        </div>
      );
    }

    return this.props.children;
  }
}

// ---------------------------------------------------------------------------
// Validation error display
// ---------------------------------------------------------------------------

function ValidationErrors({ errors }: { errors: string[] }): ReactNode {
  return (
    <div
      role="alert"
      style={{
        padding: '1rem',
        border: '1px solid var(--anvil)',
        backgroundColor: 'var(--surface)',
        color: 'var(--text-primary)',
        fontFamily: 'monospace',
      }}
    >
      <strong>Spec validation failed</strong>
      <ul style={{ marginTop: '0.5rem', paddingLeft: '1.25rem', color: 'var(--text-muted)' }}>
        {errors.map((e, i) => (
          <li key={i}>{e}</li>
        ))}
      </ul>
    </div>
  );
}

// ---------------------------------------------------------------------------
// DashboardRenderer
// ---------------------------------------------------------------------------

export interface DashboardRendererProps {
  spec: Spec;
  className?: string;
}

/**
 * Renders a JSON spec using the Anvil component catalog.
 *
 * Validates the spec before rendering and wraps the output in an error
 * boundary so malformed specs surface errors instead of crashing the page.
 */
export function DashboardRenderer({ spec, className }: DashboardRendererProps): ReactNode {
  const validation = validateSpec(spec);

  if (!validation.valid) {
    return <ValidationErrors errors={validation.errors} />;
  }

  return (
    <RenderErrorBoundary>
      <div className={className}>
        <Renderer spec={spec} registry={registry} />
      </div>
    </RenderErrorBoundary>
  );
}
