import React from 'react';
import { render, type Instance } from 'ink';
import { isTUIAvailable, type TUIDetectionOptions } from './tty-detection.js';
import { ErrorBoundary } from '../components/ErrorBoundary.js';

export interface RenderResult {
  instance: Instance;
  cleanup: () => void;
  waitUntilExit: () => Promise<void>;
}

export interface RenderOptions extends TUIDetectionOptions {
  componentName?: string;
  onError?: () => void;
}

export function renderTUI<P extends object>(
  Component: React.ComponentType<P>,
  props: P,
  options: RenderOptions = {}
): RenderResult | null {
  if (!isTUIAvailable(options)) {
    return null;
  }

  const { componentName, onError } = options;
  const name = componentName || Component.displayName || Component.name;
  const wrappedElement = (
    <ErrorBoundary componentName={name} onExit={onError}>
      <Component {...props} />
    </ErrorBoundary>
  );

  const instance = render(wrappedElement);

  return {
    instance,
    cleanup: () => instance.unmount(),
    waitUntilExit: async () => {
      await instance.waitUntilExit();
    },
  };
}

export async function renderTUIAndWait<P extends object>(
  Component: React.ComponentType<P>,
  props: P,
  options: RenderOptions = {}
): Promise<boolean> {
  const result = renderTUI(Component, props, options);

  if (!result) {
    return false;
  }

  await result.waitUntilExit();
  return true;
}
